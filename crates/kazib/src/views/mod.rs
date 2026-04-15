use dioxus::prelude::*;
use dioxus_fullstack::{WebSocketOptions, Websocket};

mod admin;
mod book;
mod history;
mod search;

use crate::model::DownloadProgress;
pub use admin::{Settings, get_settings};
pub use book::Book;
pub use history::{History, check_book_in_library};
pub use search::Search;

#[cfg(feature = "server")]
fn resolve_library_path(
    library: &crate::model::Library,
    item: &annas_archive_api::ItemDetails,
) -> Result<std::path::PathBuf, crate::server::db::TemplateError> {
    use crate::server::db::TemplateError;
    use crate::server::path_template::{PathTemplate, TemplateResult};
    use std::collections::HashMap;

    let mut metadata = HashMap::new();
    metadata.insert("title".into(), item.title.clone());

    if let Some(author) = &item.author {
        metadata.insert("author".into(), author.clone());
    }

    if let Some(series) = &item.series {
        metadata.insert("series".into(), series.clone());
    }

    if let Some(language) = &item.language {
        metadata.insert("language".into(), language.clone());
    }

    if let Some(year) = &item.year {
        metadata.insert("year".into(), year.clone());
    }

    if let Some(ext) = &item.format {
        metadata.insert("ext".into(), ext.clone());
    }

    match PathTemplate::resolve(&library.path_template, &metadata) {
        TemplateResult::Path {
            directory,
            filename,
        } => {
            let mut path = std::path::PathBuf::from(&directory);
            path.push(filename);
            Ok(path)
        }
        TemplateResult::MissingFields(fields) => Err(TemplateError::MissingFields(fields)),
    }
}

#[get("/users/me", headers: dioxus_fullstack::HeaderMap)]
pub async fn get_current_user() -> Result<Option<String>> {
    use crate::AppSettings;
    use crate::server::DATABASE;
    use dioxus::CapturedError;

    let db = DATABASE.clone();
    let settings = AppSettings::get(&db).map_err(CapturedError::from_display)?;

    let username = headers
        .get(&settings.auth_header_name)
        .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
        .map(|s: &str| s.to_string());

    Ok(username)
}

#[cfg(feature = "server")]
use {
    futures_util::StreamExt,
    std::path::Path,
    tokio::{fs::File, io::AsyncWriteExt},
};

#[get("/api/download-book?md5&username&library")]
async fn download_book(
    md5: String,
    username: Option<String>,
    library: String,
    options: WebSocketOptions,
) -> Result<Websocket<(), DownloadProgress>> {
    Ok(options.on_upgrade(move |mut socket| async move {
        use crate::AppSettings;
        use crate::model::{DownloadHistoryEntry, HistoryStatus};
        use crate::server::{CLIENT, DATABASE};

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
                    error: "Settings not configured".to_string(),
                })
                .await;
            return;
        };

        // Find the library to use
        let selected_lib = settings.libraries.iter().find(|l| l.name == library);

        let Some(selected_lib) = selected_lib else {
            let _ = socket
                .send(DownloadProgress::Error {
                    md5: md5.clone(),
                    error: format!("Library '{}' not found", library),
                })
                .await;
            return;
        };

        // Resolve the library path using the template
        let (download_folder, history_status) =
            match resolve_library_path(selected_lib, &item_details) {
                Ok(path) => {
                    // Create directory if it doesn't exist
                    if let Err(e) = tokio::fs::create_dir_all(&path).await {
                        let _ = socket
                            .send(DownloadProgress::Error {
                                md5: md5.clone(),
                                error: format!("Failed to create library directory: {}", e),
                            })
                            .await;
                        return;
                    }
                    (path, None)
                }
                Err(crate::server::db::TemplateError::MissingFields(fields)) => {
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
                    (
                        temp_dir,
                        Some(HistoryStatus::Pending {
                            missing_fields: fields,
                            temp_path,
                        }),
                    )
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
        sanitized[..max_len].to_string()
    } else {
        sanitized.to_string()
    }
}
