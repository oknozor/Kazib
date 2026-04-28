#[cfg(feature = "server")]
use crate::server::{errors::ServerError, path_template::DownloadPath};
use dioxus_fullstack::{WebSocketOptions, Websocket};

use dioxus::prelude::*;

use crate::{
    model::{DownloadProgress, Library},
    views::{get_current_user, get_settings},
};

#[component]
pub fn DownloadButton(md5: String, is_in_library: bool) -> Element {
    let mut download_state = use_signal(|| None::<DownloadProgress>);
    let mut ws_socket: Signal<Option<Websocket<(), DownloadProgress>>> = use_signal(|| None);
    let mut show_library_selector = use_signal(|| false);
    let mut selected_library = use_signal(|| None::<String>);

    let available_libraries = use_resource(move || async move {
        match get_settings().await {
            Ok(settings) => settings.libraries,
            Err(_) => Vec::new(),
        }
    });

    let handle_select_library = {
        let mut selected_library = selected_library;
        move |lib_name: String| {
            selected_library.set(Some(lib_name));
        }
    };

    let close_library_selector = {
        let mut show_library_selector = show_library_selector;
        let mut selected_library = selected_library;
        move |_| {
            show_library_selector.set(false);
            selected_library.set(None);
        }
    };

    let open_library_selector = {
        move |e: Event<MouseData>| {
            e.stop_propagation();
            if !available_libraries().unwrap_or_default().is_empty() {
                show_library_selector.set(true);
            }
        }
    };

    let start_download = {
        let md5 = md5.clone();

        move |library_name: String| {
            let md5 = md5.clone();

            spawn(async move {
                let username = get_current_user().await.ok().flatten();

                if let Ok(socket) =
                    download_book(md5, username, library_name, WebSocketOptions::new()).await
                {
                    show_library_selector.set(false);
                    selected_library.set(None);
                    ws_socket.set(Some(socket));

                    // Listen for progress updates
                    spawn(async move {
                        let mut ws_socket_lock = ws_socket.write().take();
                        if let Some(socket) = ws_socket_lock.as_mut() {
                            while let Ok(progress) = socket.recv().await {
                                download_state.set(Some(progress));
                            }
                        }
                    });
                }
            });
        }
    };

    let confirm_download = {
        let selected = selected_library;
        let start_download = start_download.clone();

        move |_| {
            if let Some(ref lib) = selected() {
                start_download(lib.clone());
            }
        }
    };

    rsx! {

        div { class: "search-result-actions",
            match download_state() {
                Some(DownloadProgress::Started) => rsx! {
                    button { class: "download-button downloading", disabled: true,
                        span { class: "spinner" }
                        "Starting..."
                    }
                },
                Some(DownloadProgress::Progress { percent, .. }) => rsx! {
                    button { class: "download-button downloading", disabled: true,
                        span { class: "spinner" }
                        "Downloading {percent:.0}%"
                        ,}
                },
                Some(DownloadProgress::Completed { .. }) => rsx! {
                    button { class: "download-button completed", disabled: true, "✓ Downloaded" }
                },
                Some(DownloadProgress::Error { ref error, .. }) => rsx! {
                    button {
                        class: "download-button error",
                        title: "{error}",
                        onclick: open_library_selector,
                        "⚠ Retry"
                    }
                },
                None => {
                    rsx! {
                        button {
                            class: "download-button",
                            disabled: is_in_library,
                            onclick: open_library_selector,
                            if is_in_library {
                                "📚 Already in Library"
                            } else {
                                "⬇ Download"
                            }
                        }
                    }
                }
            }
        }

        if show_library_selector() {
            if let Some(libs) = available_libraries() {
                LibrarySelectorModal {
                    libraries: libs.clone(),
                    selected: selected_library(),
                    on_select: handle_select_library,
                    on_cancel: close_library_selector,
                    on_download: confirm_download,
                }
            }
        }
    }
}

#[component]
fn LibrarySelectorModal(
    libraries: Vec<Library>,
    selected: Option<String>,
    on_select: EventHandler<String>,
    on_cancel: EventHandler<()>,
    on_download: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_cancel.call(()),
            div {
                class: "modal-content library-selector",
                onclick: move |e| e.stop_propagation(),
                h2 { "Select Download Library" }
                p { class: "modal-subtitle", "Choose where to save this book" }

                div { class: "library-list",
                    for library in libraries.clone() {
                        button {
                            class: if selected.as_ref() == Some(&library.name) { "library-option selected" } else { "library-option" },
                            onclick: move |_| on_select.call(library.name.clone()),
                            "{library.name} → {library.path_template}"
                        }
                    }
                }

                div { class: "modal-actions",
                    button {
                        class: "btn-cancel",
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    button {
                        class: "btn-download",
                        disabled: selected.is_none(),
                        onclick: move |_| {
                            let selected_clone = selected.clone();
                            on_download.call(selected_clone.unwrap_or_default())
                        },
                        "Download"
                    }
                }
            }
        }
    }
}

#[cfg(feature = "server")]
use {
    crate::server::ServerResult,
    futures_util::StreamExt,
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

        let Ok(item_details) = ({
            let client = CLIENT.read().await;
            client.get_details(&md5).await
        }) else {
            let _ = socket
                .send(DownloadProgress::Error {
                    md5: md5.clone(),
                    error: "Failed to get details".to_string(),
                })
                .await;
            return;
        };

        let md5 = item_details.md5.clone();
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
        let (download_path, history_status) =
            match resolve_library_path(selected_lib, &item_details) {
                Ok(path) => {
                    // Create directory if it doesn't exist
                    if let Err(e) = tokio::fs::create_dir_all(&path.directory).await {
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
                Err(ServerError::MissingTemplateFields(fields)) => {
                    // Create temp directory for pending downloads
                    let temp_dir = std::env::temp_dir().join("kazib_pending");
                    let temp_file = temp_dir.join(&md5);

                    let temp_dir = temp_dir.to_string_lossy().to_string();
                    let temp_path = temp_file.to_string_lossy().to_string();

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
                        DownloadPath {
                            directory: temp_dir,
                            filename: md5.clone(),
                        },
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

        let client = CLIENT.read().await;
        let download_info = match client.get_download_url(&md5, None, None).await {
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
        let file_path = download_path.full_path();

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

        // Set file permissions
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(settings.file_permissions);
            if let Err(e) = tokio::fs::set_permissions(&file_path, perms.clone()).await {
                eprintln!("Failed to set file permissions: {}", e);
            }

            // Recursively set permissions for parent directories
            let mut current_path = file_path.parent();
            while let Some(parent) = current_path {
                if let Err(e) = tokio::fs::set_permissions(parent, perms.clone()).await {
                    eprintln!(
                        "Failed to set directory permissions for {}: {}",
                        parent.display(),
                        e
                    );
                }
                current_path = parent.parent();
            }
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

// /books/Romans/{language}/{author}/{?series}/{series}/{series} - {series_number} - {/series}{title}.{ext}
#[cfg(feature = "server")]
fn resolve_library_path(
    library: &crate::model::Library,
    item: &annas_archive_api::ItemDetails,
) -> ServerResult<DownloadPath> {
    use crate::server::{
        errors::ServerError,
        path_template::{PathTemplate, TemplateResult},
    };
    use std::collections::HashMap;

    let mut metadata = HashMap::new();
    metadata.insert("title".into(), item.title.clone());

    if let Some(author) = &item.author {
        metadata.insert("author".into(), author.clone());
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

    if let Some(serie) = &item.serie {
        metadata.insert("series".into(), serie.name.clone());
        metadata.insert("series_number".into(), serie.position.to_string());
        metadata.insert("series_count".into(), serie.seed_count.to_string());
    }

    match PathTemplate::resolve(&library.path_template, &metadata) {
        TemplateResult::Path(download_path) => Ok(download_path),
        TemplateResult::MissingFields(fields) => Err(ServerError::MissingTemplateFields(fields)),
    }
}
