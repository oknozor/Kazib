use annas_archive_api::{Lang, SearchResult};
use dioxus::fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;
use std::collections::HashMap;
use strum::IntoEnumIterator;

use crate::model::{DownloadProgress, FileFormat, FilterState, Filterable};
use crate::{Route, views::download_book};

#[component]
pub fn Search() -> Element {
    let mut input = use_signal(String::new);
    let mut format_filters = use_signal(|| {
        let mut map = HashMap::new();
        for format in FileFormat::iter() {
            map.insert(format, FilterState::Off);
        }
        map
    });

    let mut lang_filters = use_signal(|| {
        let mut map = HashMap::new();
        for lang in Lang::iter() {
            map.insert(lang, FilterState::Off);
        }
        map
    });

    let mut search_results = use_action(
        async move |input: String, ext_filters: Vec<String>, lang_filters: Vec<String>| {
            if input.is_empty() {
                return Ok(vec![]);
            }

            search(input, ext_filters, lang_filters).await
        },
    );

    let mut trigger_search = move || {
        let ext_filters = build_filters(&format_filters());
        let lang_query_filters = build_filters(&lang_filters());
        search_results.call(input(), ext_filters, lang_query_filters);
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

    let on_lang_change = move |lang: Lang| {
        lang_filters.with_mut(|filters| {
            let current_state = filters.get(&lang).copied().unwrap_or(FilterState::Off);
            filters.insert(lang, current_state.cycle());
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

                FilterListComponent::<Lang> {
                    label: "Language",
                    filters: lang_filters(),
                    on_change: on_lang_change,
                }

                FilterListComponent::<FileFormat> {
                    label: "File Format",
                    filters: format_filters(),
                    on_change: on_format_change,
                }
            }

            main { class: "search-main",
                SearchInputComponent { oninput }
                {results}
            }
        }
    }
}

// Helper function to build filter strings from filter states
fn build_filters<T: Filterable>(filters: &HashMap<T, FilterState>) -> Vec<String> {
    let mut result = Vec::new();

    for (item, state) in filters {
        match state {
            FilterState::Include => {
                result.push(item.as_str().to_string());
            }
            FilterState::Exclude => {
                result.push(format!("anti__{}", item.as_str()));
            }
            FilterState::Off => {}
        }
    }

    result
}

#[component]
pub fn SearchInputComponent(oninput: EventHandler<String>) -> Element {
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
fn FilterListComponent<T: Filterable + IntoEnumIterator + 'static>(
    label: &'static str,
    filters: HashMap<T, FilterState>,
    on_change: EventHandler<T>,
) -> Element
where
    T: PartialEq + Clone,
{
    let mut show_more = use_signal(|| false);

    rsx! {
        div { class: "filter-section",
            label { "{label}" }
            div { class: "format-filter-list",
                // Primary items (always visible)
                for item in T::primary() {
                    FilterCheckbox {
                        item: *item,
                        state: filters.get(item).copied().unwrap_or(FilterState::Off),
                        on_click: move |_| on_change.call(*item)
                    }
                }

                // Secondary items (shown when expanded)
                if show_more() {
                    for item in T::secondary() {
                        FilterCheckbox {
                            item,
                            state: filters.get(&item).copied().unwrap_or(FilterState::Off),
                            on_click: move |_| on_change.call(item)
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
fn FilterCheckbox<T: Filterable + 'static>(
    item: T,
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
            span { class: "format-name", "{item}" }
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

#[server]
async fn search(
    query: String,
    ext_filters: Vec<String>,
    lang_filters: Vec<String>,
) -> Result<Vec<SearchResult>> {
    use crate::CLIENT;
    use annas_archive_api::SearchOptions;
    use dioxus::CapturedError;

    if query.is_empty() {
        return Ok(vec![]);
    }

    let mut search_options = SearchOptions::new(query);

    if !ext_filters.is_empty() {
        search_options = search_options.with_ext_filters(ext_filters);
    }

    if !lang_filters.is_empty() {
        search_options = search_options.with_lang_filters(lang_filters);
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
