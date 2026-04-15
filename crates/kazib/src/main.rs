use annas_archive_api::{ItemDetails, SearchResult};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use views::{Book, Search, Settings};

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
};

#[cfg(feature = "server")]
mod db;

#[cfg(feature = "server")]
mod path_template;

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

        let download_folder = match settings.download_path(&item_details) {
            Ok(folder) => folder,
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
        div { id: "navbar",
            Link { to: Route::Search {}, "Home" }
            Link { to: Route::Settings {}, "Settings" }
        }

        Outlet::<Route> {}
    }
}
