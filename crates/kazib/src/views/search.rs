use annas_archive_api::{Lang, SearchResult};
use dioxus::prelude::*;

use crate::{download_book, DownloadProgress, Route, WebSocketOptions, Websocket};

#[component]
pub fn Search() -> Element {
    let mut input = use_signal(String::new);
    let mut lang = use_signal(|| None::<Lang>);
    let mut search_results = use_action(async move |input: String, lang: Option<String>| {
        if input.is_empty() {
            return Ok(vec![]);
        }

        crate::search(input, lang).await
    });

    let oninput = move |value: String| {
        input.set(value.clone());
        let lang = lang().map(|lang| lang.to_string());

        search_results.call(value, lang)
    };

    let onlang = move |selected_lang: Option<Lang>| {
        lang.set(selected_lang);
    };

    let results = match search_results.value() {
        Some(Ok(results)) => rsx! {
            for result in results() {
                SearchResultComponent { result }
            }
        },
        Some(Err(err)) => rsx! {
            div { "Error: {err}" }
        },
        None => rsx! {},
    };

    rsx! {
        div {
            SearchInputComponent { oninput, onlang }
            {results}
        }
    }
}

#[component]
pub fn SearchInputComponent(
    oninput: EventHandler<String>,
    onlang: EventHandler<Option<Lang>>,
) -> Element {
    rsx! {
        div { class: "search-controls",
            input {
                id: "search-input",
                r#type: "search",
                placeholder: "Search...",
                oninput: move |e| oninput.call(e.value()),
            }
            select {
                id: "lang-select",
                onchange: move |e| {
                    let lang = match e.value().as_str() {
                        "en" => Some(Lang::En),
                        "fr" => Some(Lang::Fr),
                        _ => None,
                    };
                    onlang.call(lang);
                },
                option { value: "", "All languages" }
                option { value: "en", "English" }
                option { value: "fr", "French" }
            }
        }
    }
}

#[component]
pub fn SearchResultComponent(result: SearchResult) -> Element {
    let md5 = result.md5.clone();
    let mut download_state = use_signal(|| None::<DownloadProgress>);
    let mut ws_socket: Signal<Option<Websocket<(), DownloadProgress>>> = use_signal(|| None);

    let handle_download = move |e: Event<MouseData>| {
        let md5 = md5.clone();
        e.stop_propagation();

        // Connect to websocket and start download
        spawn(async move {
            if let Ok(socket) = download_book(md5, WebSocketOptions::new()).await {
                ws_socket.set(Some(socket));

                // Listen for progress updates
                spawn(async move {
                    let ws_socket = ws_socket.write().take();
                    if let Some(socket) = ws_socket {
                        while let Ok(progress) = socket.recv().await {
                            download_state.set(Some(progress));
                        }
                    }
                });
            }
        });
    };

    rsx! {
        div { class: "search-result-container",

            Link {
                to: Route::Book {
                    md5: result.md5.clone(),
                },
                class: "search-result",

                if let Some(ref cover_url) = result.cover_url {
                    img {
                        class: "search-result-cover",
                        src: "{cover_url}",
                        alt: "{result.title}",
                    }
                }

                div { class: "search-result-content",

                    h4 { class: "search-result-title", "{result.title}" }

                    if let Some(ref author) = result.author {
                        p { class: "search-result-author", "{author}" }
                    }

                    div { class: "search-result-metadata",

                        if let Some(ref format) = result.format {
                            span { class: "metadata-badge", "{format}" }
                        }

                        if let Some(ref size) = result.size {
                            span { class: "metadata-badge", "{size}" }
                        }

                        if let Some(ref language) = result.language {
                            span { class: "metadata-badge", "{language}" }
                        }
                    }
                }
            }

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
                            onclick: handle_download,
                            "⚠ Retry"
                        }
                    },
                    None => rsx! {
                        button { class: "download-button", onclick: handle_download, "⬇ Download" }
                    },
                }
            }
        }
    }
}
