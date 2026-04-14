use annas_archive_api::{ItemDetails, SearchResult};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use views::{BookView, SearchView, SettingsView};

pub use dioxus_fullstack::{WebSocketOptions, Websocket};

#[cfg(feature = "server")]
use {
    annas_archive_api::{AnnasArchiveClient, SearchOptions},
    dioxus::{fullstack::Lazy, CapturedError},
    futures_util::StreamExt,
    redb::Database,
    std::{
        path::Path,
        sync::{Arc, RwLock},
    },
    tokio::{fs::File, io::AsyncWriteExt},
};

#[cfg(feature = "server")]
mod db;

mod views;

#[cfg(feature = "server")]
static DATABASE: Lazy<Arc<Database>> = Lazy::new(async move || {
    let db_path = std::path::Path::new("data/kazib.db");
    let db = db::init_db(db_path).map_err(|e| CapturedError::from_display(e))?;
    Ok::<Arc<Database>, CapturedError>(Arc::new(db))
});

#[cfg(feature = "server")]
static CLIENT: Lazy<Arc<RwLock<AnnasArchiveClient>>> = Lazy::new(async move || {
    let db = DATABASE.clone();

    // Try to load API key from database
    let api_key = db::load_api_key(&db).map_err(|e| CapturedError::from_display(e))?;

    Ok::<Arc<RwLock<AnnasArchiveClient>>, CapturedError>(Arc::new(RwLock::new(
        AnnasArchiveClient::new("annas-archive.gl".to_string(), api_key),
    )))
});

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    SearchView {},
    #[route("/book/:md5")]
    BookView { md5: String },
    #[route("/admin")]
    SettingsView {},
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
    pub api_key: String,
    pub download_folder: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum DownloadProgress {
    Started {
        md5: String,
        title: String,
    },
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

#[server]
#[post("/save-settings")]
async fn save_settings(settings: AppSettings) -> Result<()> {
    let db = DATABASE.clone();

    // Save API key if provided
    if !settings.api_key.is_empty() {
        db::save_api_key(&db, &settings.api_key).map_err(CapturedError::from_display)?;
        // Update the client with the new API key
        *CLIENT.write().unwrap() =
            AnnasArchiveClient::new("annas-archive.gl".to_string(), Some(settings.api_key));
    }

    // Save download folder
    if !settings.download_folder.is_empty() {
        db::save_download_folder(&db, &settings.download_folder)
            .map_err(CapturedError::from_display)?;
    }

    Ok(())
}

#[server]
#[get("/get-settings")]
async fn get_settings() -> Result<AppSettings> {
    let db = DATABASE.clone();

    let api_key = db::load_api_key(&db)
        .map_err(CapturedError::from_display)?
        .unwrap_or_default();

    let download_folder = db::load_download_folder(&db)
        .map_err(CapturedError::from_display)?
        .unwrap_or_else(|| {
            // Default download folder
            #[cfg(target_os = "windows")]
            let default = format!(
                "{}\\Downloads",
                std::env::var("USERPROFILE").unwrap_or_else(|_| "C:".to_string())
            );
            #[cfg(not(target_os = "windows"))]
            let default = format!(
                "{}/Downloads",
                std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
            );
            default
        });

    Ok(AppSettings {
        api_key,
        download_folder,
    })
}

#[get("/api/download-book?md5&title")]
async fn download_book(
    md5: String,
    title: String,
    options: WebSocketOptions,
) -> Result<Websocket<(), DownloadProgress>> {
    Ok(options.on_upgrade(move |mut socket| async move {
        let _ = socket
            .send(DownloadProgress::Started {
                md5: md5.clone(),
                title: title.clone(),
            })
            .await;

        let db = DATABASE.clone();
        let download_folder = match db::load_download_folder(&db) {
            Ok(Some(folder)) => folder,
            _ => {
                let _ = socket
                    .send(DownloadProgress::Error {
                        md5: md5.clone(),
                        error: "Download folder not configured".to_string(),
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
        div {
            id: "navbar",
            Link {
                to: Route::SearchView {},
                "Home"
            }
            Link {
                to: Route::SettingsView {},
                "Settings"
            }
        }

        Outlet::<Route> {}
    }
}
