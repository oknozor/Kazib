use annas_archive_api::{ItemDetails, SearchResult};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use views::{Book, Search, Settings, History};

pub use dioxus_fullstack::{WebSocketOptions, Websocket};

#[cfg(feature = "server")]
use {
    annas_archive_api::{AnnasArchiveClient, SearchOptions},
    dioxus::{CapturedError, fullstack::Lazy},
    futures_util::StreamExt,
    redb::Database,
    std::{
        path::Path,
        sync::{Arc, RwLock},
    },
    tokio::{fs::File, io::AsyncWriteExt},
    db::TemplateError,
};

#[cfg(feature = "server")]
mod db;

#[cfg(feature = "server")]
mod path_template;

mod views;

/// A missing metadata field (shared between client and server)
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct MissingField {
    pub variable: String,
    pub description: String,
}

#[cfg(feature = "server")]
static DATABASE: Lazy<Arc<Database>> = Lazy::new(async move || {
    let db_path = std::path::Path::new("data/kazib.db");
    let db = db::init_db(db_path).map_err(|e| CapturedError::from_display(e))?;
    Ok::<Arc<Database>, CapturedError>(Arc::new(db))
});

#[cfg(feature = "server")]
static CLIENT: Lazy<Arc<RwLock<AnnasArchiveClient>>> = Lazy::new(async move || {
    let db = DATABASE.clone();
    let settings = AppSettings::get(&db).map_err(CapturedError::from_display)?;

    Ok::<Arc<RwLock<AnnasArchiveClient>>, CapturedError>(Arc::new(RwLock::new(
        AnnasArchiveClient::new("annas-archive.gl".to_string(), settings.api_key),
    )))
});

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Search{},
    #[route("/book/:md5")]
    Book{ md5: String },
    #[route("/history")]
    History{},
    #[route("/admin")]
    Settings{},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[server]
#[get("/search?query&lang")]
async fn search(query: String, lang: Option<String>) -> Result<Vec<SearchResult>> {
    if query.is_empty() {
        return Ok(vec![]);
    }

    let mut query = SearchOptions::new(query);

    if let Some(lang) = lang {
        query = query.with_lang(lang.into());
    }

    CLIENT
        .read()
        .unwrap()
        .search(query)
        .await
        .map_err(CapturedError::from_display)
        .map(|response| response.results)
}

#[server]
#[get("/book-details?md5")]
async fn get_book_details(md5: String) -> Result<ItemDetails> {
    CLIENT
        .read()
        .unwrap()
        .get_details(&md5)
        .await
        .map_err(CapturedError::from_display)
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub api_key: Option<String>,
    pub download_path_template: String,
    pub auth_header_name: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_path_template = dirs::download_dir()
            .or(dirs::document_dir())
            .or(dirs::data_dir())
            .or(dirs::home_dir())
            .expect("failed to get default download location");

        let download_path_template = download_path_template.join("Kazib");
        let download_path_template = download_path_template.to_string_lossy().into_owned();

        Self {
            api_key: None,
            download_path_template,
            auth_header_name: "x-authentik-username".to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum DownloadProgress {
    Started,
    Progress {
        md5: String,
        downloaded: u64,
        total: u64,
        percent: f32,
    },
    Completed {
        md5: String,
        file_path: String,
    },
    Error {
        md5: String,
        error: String,
    },
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum HistoryStatus {
    Success {
        resolved_path: String,
    },
    Pending {
        missing_fields: Vec<MissingField>,
        temp_path: String,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DownloadHistoryEntry {
    pub md5: String,
    pub item_details: ItemDetails,
    pub status: HistoryStatus,
    pub download_date: String,
    pub file_path: Option<String>,
    pub error_details: Option<String>,
    pub username: Option<String>,
}

#[server]
#[post("/save-settings")]
async fn save_settings(settings: AppSettings) -> Result<()> {
    let db = DATABASE.clone();
    settings.save(&db).map_err(CapturedError::from_display)?;

    if settings.api_key.is_some() {
        *CLIENT.write().expect("failed to acquire write lock") =
            AnnasArchiveClient::new("annas-archive.gl".to_string(), settings.api_key);
    }

    Ok(())
}

#[get("/get-settings")]
async fn get_settings() -> Result<AppSettings> {
    let db = DATABASE.clone();
    AppSettings::get(&db).map_err(CapturedError::from_display)
}

#[server]
async fn get_current_user() -> Result<Option<String>> {
    // Extract headers from HTTP request
    // Note: Using deprecated extract() for now, works with Dioxus 0.7.4
    #[allow(deprecated)]
    let headers: axum::http::HeaderMap = dioxus::fullstack::extract()
        .await
        .map_err(|_| CapturedError::from_display("Failed to extract headers"))?;

    let db = DATABASE.clone();
    let settings = AppSettings::get(&db).map_err(CapturedError::from_display)?;

    // Extract username from the configured auth header
    let username = headers
        .get(&settings.auth_header_name)
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
        .map(|s: &str| s.to_string());

    Ok(username)
}

#[server]
#[get("/api/download-history")]
async fn get_download_history() -> Result<Vec<DownloadHistoryEntry>> {
    let db = DATABASE.clone();
    DownloadHistoryEntry::get_all(&db).map_err(CapturedError::from_display)
}

#[server]
#[delete("/api/delete-history?md5&delete_file")]
async fn delete_history_entry(md5: String, delete_file: bool) -> Result<()> {
    let db = DATABASE.clone();

    // Get entry to find file path
    if delete_file {
        if let Ok(Some(entry)) = DownloadHistoryEntry::get(&md5, &db) {
            if let Some(file_path) = entry.file_path {
                let _ = tokio::fs::remove_file(&file_path).await;
            }
        }
    }

    DownloadHistoryEntry::delete(&md5, &db).map_err(CapturedError::from_display)
}


#[get("/api/download-book?md5&username")]
async fn download_book(
    md5: String,
    username: Option<String>,
    options: WebSocketOptions,
) -> Result<Websocket<(), DownloadProgress>> {
    Ok(options.on_upgrade(move |mut socket| async move {
        let _ = socket.send(DownloadProgress::Started).await;

        let item_details = {
            let client = CLIENT.read().unwrap();
            client.get_details(&md5).await.unwrap()
        };

        let md5 = item_details.md5.clone();
        let title = match item_details.format.as_ref() {
            Some(format) => format!("{}.{}", item_details.title, format.to_lowercase()),
            None => item_details.title.clone(),
        };

        let db = DATABASE.clone();
        let Ok(settings) = AppSettings::get(&db) else {
            let _ = socket
                .send(DownloadProgress::Error {
                    md5: md5.clone(),
                    error: "Download folder not configured".to_string(),
                })
                .await;
            return;
        };

        // Try to resolve download path, fallback to temp if template has missing fields
        let (download_folder, history_status) = match settings.download_path(&item_details) {
            Ok(folder) => (folder, None), // Will set to Success after download
            Err(TemplateError::MissingFields(fields)) => {
                // Create temp directory for pending downloads
                let temp_dir = std::env::temp_dir().join("kazib_pending").join(&md5);
                let temp_path = temp_dir.to_string_lossy().to_string();
                if let Err(e) = tokio::fs::create_dir_all(&temp_dir).await {
                    let _ = socket
                        .send(DownloadProgress::Error {
                            md5: md5.clone(),
                            error: format!("Failed to create temp directory: {}", e),
                        })
                        .await;
                    return;
                }
                (temp_dir, Some(HistoryStatus::Pending {
                    missing_fields: fields,
                    temp_path,
                }))
            }
            Err(e) => {
                let _ = socket
                    .send(DownloadProgress::Error {
                        md5: md5.clone(),
                        error: format!("Template error: {}", e),
                    })
                    .await;
                return;
            }
        };

        let download_info = match CLIENT
            .read()
            .unwrap()
            .get_download_url(&md5, None, None)
            .await
        {
            Ok(info) => info,
            Err(e) => {
                let _ = socket
                    .send(DownloadProgress::Error {
                        md5: md5.clone(),
                        error: format!("Failed to get download URL: {}", e),
                    })
                    .await;
                return;
            }
        };

        let response = match reqwest::get(&download_info.download_url).await {
            Ok(resp) => resp,
            Err(e) => {
                let _ = socket
                    .send(DownloadProgress::Error {
                        md5: md5.clone(),
                        error: format!("Failed to start download: {}", e),
                    })
                    .await;
                return;
            }
        };

        let total_size = response.content_length().unwrap_or(0);

        let filename = sanitize_filename(&title);
        let file_path = Path::new(&download_folder).join(&filename);

        let mut file = match File::create(&file_path).await {
            Ok(f) => f,
            Err(e) => {
                let _ = socket
                    .send(DownloadProgress::Error {
                        md5: md5.clone(),
                        error: format!("Failed to create file: {}", e),
                    })
                    .await;
                return;
            }
        };

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    let _ = socket
                        .send(DownloadProgress::Error {
                            md5: md5.clone(),
                            error: format!("Download failed: {}", e),
                        })
                        .await;
                    return;
                }
            };

            if let Err(e) = file.write_all(&chunk).await {
                let _ = socket
                    .send(DownloadProgress::Error {
                        md5: md5.clone(),
                        error: format!("Failed to write file: {}", e),
                    })
                    .await;
                return;
            }

            downloaded += chunk.len() as u64;
            let percent = if total_size > 0 {
                (downloaded as f32 / total_size as f32) * 100.0
            } else {
                0.0
            };

            let _ = socket
                .send(DownloadProgress::Progress {
                    md5: md5.clone(),
                    downloaded,
                    total: total_size,
                    percent,
                })
                .await;
        }

        // Flush file
        if let Err(e) = file.flush().await {
            let _ = socket
                .send(DownloadProgress::Error {
                    md5: md5.clone(),
                    error: format!("Failed to flush file: {}", e),
                })
                .await;
            return;
        }

        // Determine final status
        let final_status = history_status.unwrap_or_else(|| HistoryStatus::Success {
            resolved_path: file_path.to_string_lossy().to_string(),
        });

        // Save to history
        let history_entry = DownloadHistoryEntry {
            md5: md5.clone(),
            item_details: item_details.clone(),
            status: final_status,
            download_date: chrono::Utc::now().to_rfc3339(),
            file_path: Some(file_path.to_string_lossy().to_string()),
            error_details: None,
            username,
        };

        if let Err(e) = history_entry.save(&db) {
            eprintln!("Failed to save download history: {}", e);
        }

        // Send completion message
        let _ = socket
            .send(DownloadProgress::Completed {
                md5,
                file_path: file_path.to_string_lossy().to_string(),
            })
            .await;
    }))
}

#[cfg(feature = "server")]
fn sanitize_filename(title: &str) -> String {
    let sanitized = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>();

    let max_len = 200;
    if sanitized.len() > max_len {
        format!("{}", &sanitized[..max_len])
    } else {
        format!("{}", sanitized)
    }
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn Navbar() -> Element {
    rsx! {
        div { id: "navbar",
            Link { to: Route::Search {}, "Home" }
            Link { to: Route::History {}, "History" }
            Link { to: Route::Settings {}, "Settings" }
        }

        Outlet::<Route> {}
    }
}
