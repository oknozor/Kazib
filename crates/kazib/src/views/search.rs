use annas_archive_api::{Lang, SearchResult};
use dioxus::fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;
use std::collections::HashMap;
use strum::IntoEnumIterator;

use crate::model::{DownloadProgress, FileFormat, FilterState};
use crate::{Route, views::download_book};

#[component]
pub fn Search() -> Element {
    let mut input = use_signal(String::new);
    let mut lang = use_signal(|| None::<Lang>);
    let mut format_filters = use_signal(|| {
        let mut map = HashMap::new();
        for format in FileFormat::iter() {
            map.insert(format, FilterState::Off);
        }
        map
    });

    let mut search_results = use_action(async move |input: String, lang: Option<String>, ext_filters: Vec<String>| {
        if input.is_empty() {
            return Ok(vec![]);
        }

        search(input, lang, ext_filters).await
    });

    let mut trigger_search = move || {
        let lang_str = lang().map(|lang| lang.to_string());
        let ext_filters = build_ext_filters(&format_filters());
        search_results.call(input(), lang_str, ext_filters);
    };

    let oninput = move |value: String| {
        input.set(value.clone());
        trigger_search();
    };

    let on_format_change = move |format: FileFormat| {
        format_filters.with_mut(|filters| {
            let current_state = filters.get(&format).copied().unwrap_or(FilterState::Off);
            filters.insert(format, current_state.cycle());
        });
        trigger_search();
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
        div { class: "search-page",
            aside { class: "search-sidebar",
                h3 { "Filters" }

                div { class: "filter-section",
                    label { "Language" }
                    select {
                        id: "lang-select",
                        onchange: move |e| {
                            let selected_lang = match e.value().as_str() {
                                "en" => Some(Lang::En),
                                "fr" => Some(Lang::Fr),
                                _ => None,
                            };
                            lang.set(selected_lang);
                            trigger_search();
                        },
                        option { value: "", "All languages" }
                        option { value: "en", "English" }
                        option { value: "fr", "French" }
                    }
                }

                FormatFiltersComponent {
                    format_filters: format_filters(),
                    on_format_change
                }
            }

            main { class: "search-main",
                SearchInputComponent { oninput }
                {results}
            }
        }
    }
}

// Helper function to build ext filter strings from filter states
fn build_ext_filters(filters: &HashMap<FileFormat, FilterState>) -> Vec<String> {
    let mut ext_filters = Vec::new();

    for (format, state) in filters {
        match state {
            FilterState::Include => {
                ext_filters.push(format.as_str().to_string());
            }
            FilterState::Exclude => {
                ext_filters.push(format!("anti_{}", format.as_str()));
            }
            FilterState::Off => {}
        }
    }

    ext_filters
}

#[component]
pub fn SearchInputComponent(
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "search-controls",
            input {
                id: "search-input",
                r#type: "search",
                placeholder: "Search...",
                oninput: move |e| oninput.call(e.value()),
            }
        }
    }
}

#[component]
fn FormatFiltersComponent(
    format_filters: HashMap<FileFormat, FilterState>,
    on_format_change: EventHandler<FileFormat>,
) -> Element {
    let mut show_more = use_signal(|| false);

    rsx! {
        div { class: "filter-section",
            label { "File Format" }
            div { class: "format-filter-list",
                // Primary formats (always visible)
                for format in FileFormat::PRIMARY {
                    FormatFilterCheckbox {
                        format: *format,
                        state: format_filters.get(format).copied().unwrap_or(FilterState::Off),
                        on_click: move |_| on_format_change.call(*format)
                    }
                }

                // Secondary formats (shown when expanded)
                if show_more() {
                    for format in FileFormat::secondary() {
                        FormatFilterCheckbox {
                            format,
                            state: format_filters.get(&format).copied().unwrap_or(FilterState::Off),
                            on_click: move |_| on_format_change.call(format)
                        }
                    }
                }

                // More/Less button
                button {
                    class: "more-button",
                    onclick: move |_| show_more.set(!show_more()),
                    if show_more() {
                        "Less..."
                    } else {
                        "More..."
                    }
                }
            }
        }
    }
}

#[component]
fn FormatFilterCheckbox(
    format: FileFormat,
    state: FilterState,
    on_click: EventHandler<()>,
) -> Element {
    let class = match state {
        FilterState::Off => "format-checkbox",
        FilterState::Include => "format-checkbox include",
        FilterState::Exclude => "format-checkbox exclude",
    };

    rsx! {
        button {
            class: "{class}",
            onclick: move |_| on_click.call(()),
            span { class: "format-name", "{format}" }
            if state != FilterState::Off {
                span { class: "format-symbol", "{state.symbol()}" }
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
            // Get current user from auth header
            let username = get_current_user().await.ok().flatten();

            if let Ok(socket) = download_book(md5, username, WebSocketOptions::new()).await {
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

#[get("/search?query&lang&ext_filters")]
async fn search(query: String, lang: Option<String>, ext_filters: Vec<String>) -> Result<Vec<SearchResult>> {
    use crate::CLIENT;
    use annas_archive_api::SearchOptions;
    use dioxus::CapturedError;

    if query.is_empty() {
        return Ok(vec![]);
    }

    let mut search_options = SearchOptions::new(query);

    if let Some(lang) = lang {
        search_options = search_options.with_lang(lang.into());
    }

    if !ext_filters.is_empty() {
        search_options = search_options.with_ext_filters(ext_filters);
    }

    CLIENT
        .read()
        .unwrap()
        .search(search_options)
        .await
        .map_err(CapturedError::from_display)
        .map(|response| response.results)
}

#[get("/users/me", headers: dioxus::fullstack::HeaderMap)]
async fn get_current_user() -> Result<Option<String>> {
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
