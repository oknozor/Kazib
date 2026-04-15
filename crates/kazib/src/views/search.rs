use annas_archive_api::{ContentType, Lang, SearchResult};
use dioxus::fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;
use std::collections::HashMap;
use strum::IntoEnumIterator;

use crate::model::{DownloadProgress, FileFormat, FilterState, Filterable, Library};
use crate::{
    Route,
    views::{check_book_in_library, download_book, get_current_user, get_settings},
};

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
            div { class: "modal-content library-selector", onclick: move |e| e.stop_propagation(),
                h2 { "Select Download Library" }
                p { "Choose where to save this book" }

                div { class: "library-list",
                    for library in libraries.clone() {
                        button {
                            class: if selected.as_ref() == Some(&library.name) {
                                "library-option selected"
                            } else {
                                "library-option"
                            },
                            onclick: move |_| on_select.call(library.name.clone()),
                            "{library.name} → {library.path_template}"
                        }
                    }
                }

                div { class: "modal-actions",
                    button { class: "btn-cancel", onclick: move |_| on_cancel.call(()), "Cancel" }
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

    let mut content_type_filters = use_signal(|| {
        let mut map = HashMap::new();
        for content_type in ContentType::iter() {
            map.insert(content_type, FilterState::Off);
        }
        map
    });

    let mut search_results = use_action(
        async move |(input, ext_filters, lang_filters, content_filters): (
            String,
            Vec<String>,
            Vec<String>,
            Vec<String>,
        )| {
            if input.is_empty() {
                return Ok(vec![]);
            }

            search(input, ext_filters, lang_filters, content_filters).await
        },
    );

    let mut trigger_search = move || {
        let ext_filters = build_filters(&format_filters());
        let lang_query_filters = build_filters(&lang_filters());
        let content_filters = build_filters(&content_type_filters());
        search_results.call((input(), ext_filters, lang_query_filters, content_filters));
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

    let on_content_type_change = move |content_type: ContentType| {
        content_type_filters.with_mut(|filters| {
            let current_state = filters
                .get(&content_type)
                .copied()
                .unwrap_or(FilterState::Off);
            filters.insert(content_type, current_state.cycle());
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

                FilterListComponent::<ContentType> {
                    label: "Content",
                    filters: content_type_filters(),
                    on_change: on_content_type_change,
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
fn FilterListComponent<T>(
    label: &'static str,
    filters: HashMap<T, FilterState>,
    on_change: EventHandler<T>,
) -> Element
where
    T: Filterable + IntoEnumIterator + 'static + PartialEq + Clone,
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
    let md5_for_check = result.md5.clone();
    let download_state = use_signal(|| None::<DownloadProgress>);
    let ws_socket: Signal<Option<Websocket<(), DownloadProgress>>> = use_signal(|| None);
    let is_in_library = use_resource(move || {
        let md5 = md5_for_check.clone();
        async move { check_book_in_library(md5).await.unwrap_or(false) }
    });

    // Library selection state
    let show_library_selector = use_signal(|| false);
    let selected_library = use_signal(|| None::<String>);
    let available_libraries = use_resource(move || async move {
        match get_settings().await {
            Ok(settings) => settings.libraries,
            Err(_) => Vec::new(),
        }
    });

    let start_download = {
        let md5 = md5.clone();
        let mut ws_socket = ws_socket.clone();
        let mut download_state = download_state.clone();
        let mut show_library_selector = show_library_selector.clone();
        let mut selected_library = selected_library.clone();

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

    let open_library_selector = {
        let available_libraries = available_libraries.clone();
        let mut show_library_selector = show_library_selector.clone();

        move |e: Event<MouseData>| {
            e.stop_propagation();
            if !available_libraries().unwrap_or_default().is_empty() {
                show_library_selector.set(true);
            }
        }
    };

    let handle_select_library = {
        let mut selected_library = selected_library.clone();
        move |lib_name: String| {
            selected_library.set(Some(lib_name));
        }
    };

    let close_library_selector = {
        let mut show_library_selector = show_library_selector.clone();
        let mut selected_library = selected_library.clone();
        move |_| {
            show_library_selector.set(false);
            selected_library.set(None);
        }
    };

    let confirm_download = {
        let selected = selected_library.clone();
        let start_download = start_download.clone();

        move |_| {
            if let Some(ref lib) = selected() {
                start_download(lib.clone());
            }
        }
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

                        // Show if book is in library
                        match is_in_library() {
                            Some(true) => rsx! { span { class: "metadata-badge library-badge", "📚 In Library" } },
                            Some(false) | None => rsx! {},
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
                            onclick: open_library_selector.clone(),
                            "⚠ Retry"
                        }
                    },
                    None => {
                        let in_library = is_in_library().unwrap_or(false);
                        rsx! {
                            button {
                                class: "download-button",
                                disabled: in_library,
                                onclick: open_library_selector.clone(),
                                if in_library {
                                    "📚 Already in Library"
                                } else {
                                    "⬇ Download"
                                }
                            }
                        }
                    },
                }
            }

            // Library selector modal
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
}

#[server]
async fn search(
    query: String,
    ext_filters: Vec<String>,
    lang_filters: Vec<String>,
    content_filters: Vec<String>,
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

    if !content_filters.is_empty() {
        search_options = search_options.with_content_filters(content_filters);
    }

    CLIENT
        .read()
        .unwrap()
        .search(search_options)
        .await
        .map_err(CapturedError::from_display)
        .map(|response| response.results)
}
