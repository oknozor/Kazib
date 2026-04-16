use annas_archive_api::{ContentType, Lang, SearchResult};
use dioxus::prelude::*;
use std::collections::HashMap;
use strum::IntoEnumIterator;

use crate::model::{FileFormat, FilterState, Filterable};
use crate::views::download::DownloadButton;
use crate::{Route, views::check_book_in_library};

#[component]
fn FilterModal(
    filters: HashMap<Lang, FilterState>,
    format_filters: HashMap<FileFormat, FilterState>,
    content_type_filters: HashMap<ContentType, FilterState>,
    on_lang_change: EventHandler<Lang>,
    on_format_change: EventHandler<FileFormat>,
    on_content_type_change: EventHandler<ContentType>,
    on_close: EventHandler<()>,
    on_search: EventHandler<()>,
    is_searching: bool,
) -> Element {
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_close.call(()),
            div {
                class: "modal-content filter-modal",
                onclick: move |e| e.stop_propagation(),
                h2 { "Filters" }

                FilterListComponent::<Lang> {
                    label: "Language",
                    filters,
                    on_change: on_lang_change,
                }

                FilterListComponent::<FileFormat> {
                    label: "File Format",
                    filters: format_filters,
                    on_change: on_format_change,
                }

                FilterListComponent::<ContentType> {
                    label: "Content",
                    filters: content_type_filters,
                    on_change: on_content_type_change,
                }

                div { class: "modal-actions",
                    button {
                        class: "btn-save",
                        disabled: is_searching,
                        onclick: move |_| {
                            on_search.call(());
                            on_close.call(());
                        },
                        if is_searching {
                            span { class: "spinner" }
                        } else {
                            "Apply & Search"
                        }
                    }
                    button {
                        class: "btn-cancel",
                        onclick: move |_| on_close.call(()),
                        "Close"
                    }
                }
            }
        }
    }
}

#[component]
pub fn Search() -> Element {
    let mut input = use_signal(String::new);
    let mut show_filter_modal = use_signal(|| false);

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
        input.set(value);
    };

    let on_search = move || {
        trigger_search();
    };

    let on_format_change = move |format: FileFormat| {
        format_filters.with_mut(|filters| {
            let current_state = filters.get(&format).copied().unwrap_or(FilterState::Off);
            filters.insert(format, current_state.cycle());
        });
    };

    let on_lang_change = move |lang: Lang| {
        lang_filters.with_mut(|filters| {
            let current_state = filters.get(&lang).copied().unwrap_or(FilterState::Off);
            filters.insert(lang, current_state.cycle());
        });
    };

    let on_content_type_change = move |content_type: ContentType| {
        content_type_filters.with_mut(|filters| {
            let current_state = filters
                .get(&content_type)
                .copied()
                .unwrap_or(FilterState::Off);
            filters.insert(content_type, current_state.cycle());
        });
    };

    let toggle_filter_modal = {
        move |_| {
            show_filter_modal.set(!show_filter_modal());
        }
    };

    let close_filter_modal = {
        move |_| {
            show_filter_modal.set(false);
        }
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
                SearchInputComponent {
                    oninput,
                    show_filter_button: true,
                    on_filter_click: toggle_filter_modal,
                    on_search,
                    is_searching: search_results.pending(),
                }
                {results}
            }

            // Filter modal for mobile
            if show_filter_modal() {
                FilterModal {
                    filters: lang_filters(),
                    format_filters: format_filters(),
                    content_type_filters: content_type_filters(),
                    on_lang_change,
                    on_format_change,
                    on_content_type_change,
                    on_close: close_filter_modal,
                    on_search,
                    is_searching: search_results.pending(),
                }
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
pub fn SearchInputComponent(
    oninput: EventHandler<String>,
    show_filter_button: bool,
    on_filter_click: EventHandler<()>,
    on_search: EventHandler<()>,
    is_searching: bool,
) -> Element {
    rsx! {
        div { class: "search-controls",
            input {
                id: "search-input",
                r#type: "search",
                placeholder: "Search...",
                oninput: move |e| oninput.call(e.value()),
            }
            button {
                class: "btn-search",
                disabled: is_searching,
                onclick: move |_| on_search.call(()),
                if is_searching {
                    span { class: "spinner" }
                } else {
                    "Search"
                }
            }
            if show_filter_button {
                button {
                    class: "btn-filter-toggle",
                    onclick: move |_| on_filter_click.call(()),
                    "Search settings"
                }
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
                        on_click: move |_| on_change.call(*item),
                    }
                }

                // Secondary items (shown when expanded)
                if show_more() {
                    for item in T::secondary() {
                        FilterCheckbox {
                            item,
                            state: filters.get(&item).copied().unwrap_or(FilterState::Off),
                            on_click: move |_| on_change.call(item),
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
        button { class: "{class}", onclick: move |_| on_click.call(()),
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
    let is_in_library = use_resource(move || {
        let md5 = md5_for_check.clone();
        async move { check_book_in_library(md5).await.unwrap_or(false) }
    });

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

                        match is_in_library() {
                            Some(true) => rsx! {
                                span { class: "metadata-badge library-badge", "📚 In Library" }
                            },
                            Some(false) | None => rsx! {},
                        }
                    }
                }
            }

            DownloadButton { md5, is_in_library: is_in_library().unwrap_or_default() }
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
    use crate::server::CLIENT;
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

    let client = CLIENT.read().await;

    client
        .search(search_options)
        .await
        .map_err(CapturedError::from_display)
        .map(|response| response.results)
}
