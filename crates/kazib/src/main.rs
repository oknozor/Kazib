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
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_path_template = dirs::download_dir()
            .or(dirs::document_dir())
            .or(dirs::data_dir())
            .expect("failed to get default download location");

        let download_path_template = download_path_template.join("Kazib");
        let download_path_template = download_path_template.to_string_lossy().into_owned();

        Self {
            api_key: None,
            download_path_template,
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

#[server]
#[post("/api/update-history-metadata")]
async fn update_history_metadata(
    md5: String,
    updated_metadata: std::collections::HashMap<String, String>,
) -> Result<DownloadHistoryEntry> {
    use path_template::{PathTemplate, TemplateResult};

    let db = DATABASE.clone();

    // Get existing entry
    let mut entry = DownloadHistoryEntry::get(&md5, &db)
        .map_err(CapturedError::from_display)?
        .ok_or_else(|| CapturedError::from_display("Entry not found"))?;

    // Update item details with new metadata
    if let Some(title) = updated_metadata.get("title") {
        entry.item_details.title = title.clone();
    }
    if let Some(author) = updated_metadata.get("author") {
        entry.item_details.author = Some(author.clone());
    }
    if let Some(series) = updated_metadata.get("series") {
        entry.item_details.series = Some(series.clone());
    }
    if let Some(language) = updated_metadata.get("language") {
        entry.item_details.language = Some(language.clone());
    }
    if let Some(year) = updated_metadata.get("year") {
        entry.item_details.year = Some(year.clone());
    }
    if let Some(ext) = updated_metadata.get("ext") {
        entry.item_details.format = Some(ext.clone());
    }

    // Try to resolve path again
    let settings = AppSettings::get(&db).map_err(CapturedError::from_display)?;
    let template = &settings.download_path_template;

    match PathTemplate::resolve(template, &updated_metadata) {
        TemplateResult::Path { directory, filename } => {
            // Create new path
            let new_dir = std::path::PathBuf::from(&directory);
            if let Err(e) = std::fs::create_dir_all(&new_dir) {
                return Err(CapturedError::from_display(format!("Failed to create directory: {}", e)));
            }

            let new_file_path = new_dir.join(&filename);

            // Move file from temp to final location
            if let Some(old_path) = &entry.file_path {
                if let Err(e) = tokio::fs::rename(old_path, &new_file_path).await {
                    return Err(CapturedError::from_display(format!("Failed to move file: {}", e)));
                }
            }

            // Update entry
            entry.status = HistoryStatus::Success {
                resolved_path: new_file_path.to_string_lossy().to_string(),
            };
            entry.file_path = Some(new_file_path.to_string_lossy().to_string());
            entry.error_details = None;
        }
        TemplateResult::MissingFields(fields) => {
            // Still missing fields
            if let Some(ref temp_path) = entry.file_path {
                entry.status = HistoryStatus::Pending {
                    missing_fields: fields,
                    temp_path: temp_path.clone(),
                };
            }
        }
    }

    // Save updated entry
    entry.save(&db).map_err(CapturedError::from_display)?;

    Ok(entry)
}

#[get("/api/download-book?md5")]
async fn download_book(
    md5: String,
    options: WebSocketOptions,
) -> Result<Websocket<(), DownloadProgress>> {
    Ok(options.on_upgrade(move |mut socket| async move {
        let _ = socket.send(DownloadProgress::Started).await;

        let item_details = {
            let client = CLIENT.read().unwrap();
            client.get_details(&md5).await.unwrap()
        };

        let md5 = item_details.md5.clone();
        let title = item_details.title.clone();

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

        let filename = sanitize_filename(&title, &md5);
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
fn sanitize_filename(title: &str, md5: &str) -> String {
    let sanitized = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>();

    // Limit filename length and add md5 suffix
    let max_len = 200;
    if sanitized.len() > max_len {
        format!("{}_{}.epub", &sanitized[..max_len], &md5[..8])
    } else {
        format!("{}_{}.epub", sanitized, &md5[..8])
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
