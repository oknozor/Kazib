use dioxus::{
    fullstack::{WebSocketOptions, Websocket},
    prelude::*,
};

use crate::{
    model::{DownloadProgress, Library},
    views::{download_book, get_current_user, get_settings},
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
                    }
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
