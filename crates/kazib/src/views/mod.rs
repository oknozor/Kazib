use dioxus::prelude::*;
use dioxus_fullstack::{WebSocketOptions, Websocket};

mod admin;
mod book;
mod history;
mod search;

use crate::model::DownloadProgress;
pub use admin::Settings;
pub use book::Book;
pub use history::History;
pub use search::Search;

#[get("/users/me", headers: dioxus_fullstack::HeaderMap)]
pub async fn get_current_user() -> Result<Option<String>> {
    use crate::{AppSettings, DATABASE};
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
    crate::db::TemplateError,
    futures_util::StreamExt,
    std::path::Path,
    tokio::{fs::File, io::AsyncWriteExt},
};

#[get("/api/download-book?md5&username")]
async fn download_book(
    md5: String,
    username: Option<String>,
    options: WebSocketOptions,
) -> Result<Websocket<(), DownloadProgress>> {
    Ok(options.on_upgrade(move |mut socket| async move {
        use crate::model::{DownloadHistoryEntry, HistoryStatus};
        use crate::{AppSettings, CLIENT, DATABASE};

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
        format!("{}", &sanitized[..max_len])
    } else {
        format!("{}", sanitized)
    }
}
