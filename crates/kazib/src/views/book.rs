use annas_archive_api::{Identifiers, ItemDetails};
use dioxus::prelude::*;

use crate::model::Library;
use crate::views::check_book_in_library;
use crate::views::download::DownloadButton;

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

#[component]
pub fn Book(md5: String) -> Element {
    let book_details = use_resource(move || {
        let md5 = md5.clone();
        async move { get_book_details(md5).await }
    });

    rsx! {
        match book_details() {
            Some(Ok(details)) => rsx! {
                BookDetailsComponent { details }
            },
            Some(Err(ref err)) => rsx! {
                div { class: "error-container",
                    h2 { "Error Loading Book Details" }
                    p { "{err}" }
                }
            },
            None => rsx! {
                div { class: "loading-container",
                    p { "Loading book details..." }
                }
            },
        }
    }
}

#[component]
fn BookDetailsComponent(details: ItemDetails) -> Element {
    let md5 = details.md5.clone();
    let md5_for_check = md5.clone();
    let is_in_library = use_resource(move || {
        let md5 = md5_for_check.clone();
        async move { check_book_in_library(md5).await.unwrap_or(false) }
    });

    rsx! {
        div { class: "book-details-container",

            // Header with title and basic info
            div { class: "book-header",

                if let Some(ref cover_url) = details.cover_url {
                    img {
                        class: "book-cover",
                        src: "{cover_url}",
                        alt: "{details.title}",
                    }
                }

                div { class: "book-header-info",

                    h1 {
                        "{details.title}"
                        // Show if book is in library
                        match is_in_library() {
                            Some(true) => rsx! {
                                span { class: "library-badge", " 📚 In Library" }
                            },
                            Some(false) | None => rsx! {},
                        }
                    }

                    if let Some(ref author) = details.author {
                        p { class: "book-author",
                            strong { "Author: " }
                            "{author}"
                        }
                    }

                    div { class: "book-metadata",

                        if let Some(ref format) = details.format {
                            span { class: "metadata-item", "Format: {format}" }
                        }

                        if let Some(ref size) = details.size {
                            span { class: "metadata-item", "Size: {size}" }
                        }

                        if let Some(ref language) = details.language {
                            span { class: "metadata-item", "Language: {language}" }
                        }

                        if let Some(ref pages) = details.pages {
                            span { class: "metadata-item", "Pages: {pages}" }
                        }
                    }

                    DownloadButton { md5, is_in_library: is_in_library().unwrap_or(false) }
                }
            }

            // Publication details
            div { class: "book-section",

                h2 { "Publication Details" }

                if let Some(ref publisher) = details.publisher {
                    p {
                        strong { "Publisher: " }
                        "{publisher}"
                    }
                }

                if let Some(ref year) = details.year {
                    p {
                        strong { "Year: " }
                        "{year}"
                    }
                }

                if let Some(ref edition) = details.edition {
                    p {
                        strong { "Edition: " }
                        "{edition}"
                    }
                }

                if let Some(ref serie) = details.serie {
                    p {
                        strong { "Series: " }
                        "{serie.name} - {serie.position}  {serie.seed_count}"
                    }
                }
            }

            // Description
            if let Some(ref description) = details.description {
                div { class: "book-section",
                    h2 { "Description" }
                    p { "{description}" }
                }
            }

            // Identifiers
            if let Some(ref identifiers) = details.identifiers {
                div { class: "book-section",
                    h2 { "Identifiers" }

                    IdentifiersComponent { identifiers: identifiers.clone() }
                }
            }

            // Categories and Subjects
            if let Some(ref categories) = details.categories {
                if !categories.is_empty() {
                    div { class: "book-section",
                        h2 { "Categories" }
                        div { class: "tags",
                            for category in categories {
                                span { class: "tag", "{category}" }
                            }
                        }
                    }
                }
            }

            if let Some(ref subjects) = details.subjects {
                if !subjects.is_empty() {
                    div { class: "book-section",
                        h2 { "Subjects" }
                        div { class: "tags",
                            for subject in subjects {
                                span { class: "tag", "{subject}" }
                            }
                        }
                    }
                }
            }

            // Technical details
            div { class: "book-section",
                h2 { "Technical Details" }

                p {
                    strong { "MD5: " }
                    "{details.md5}"
                }

                if let Some(ref filename) = details.original_filename {
                    p {
                        strong { "Original Filename: " }
                        "{filename}"
                    }
                }

                if let Some(ref content_type) = details.content_type {
                    p {
                        strong { "Content Type: " }
                        "{content_type}"
                    }
                }

                if let Some(ref added_date) = details.added_date {
                    p {
                        strong { "Added Date: " }
                        "{added_date}"
                    }
                }
            }
        }
    }
}

#[component]
fn IdentifiersComponent(identifiers: Identifiers) -> Element {
    rsx! {
        div { class: "identifiers-grid",

            if let Some(ref isbn13) = identifiers.isbn13 {
                for isbn in isbn13 {
                    p {
                        strong { "ISBN-13: " }
                        "{isbn}"
                    }
                }
            }

            if let Some(ref isbn10) = identifiers.isbn10 {
                for isbn in isbn10 {
                    p {
                        strong { "ISBN-10: " }
                        "{isbn}"
                    }
                }
            }

            if let Some(ref doi) = identifiers.doi {
                for d in doi {
                    p {
                        strong { "DOI: " }
                        "{d}"
                    }
                }
            }

            if let Some(ref asin) = identifiers.asin {
                for a in asin {
                    p {
                        strong { "ASIN: " }
                        "{a}"
                    }
                }
            }

            if let Some(ref goodreads) = identifiers.goodreads {
                for g in goodreads {
                    p {
                        strong { "Goodreads: " }
                        "{g}"
                    }
                }
            }
        }
    }
}

#[get("/book-details?md5")]
async fn get_book_details(md5: String) -> Result<ItemDetails> {
    use crate::server::{CLIENT, OL_CLIENT};
    use dioxus::CapturedError;

    let client = CLIENT.read().await;
    let mut details = client
        .get_details(&md5)
        .await
        .map_err(CapturedError::from_display)?;

    let ol_id = details
        .identifiers
        .as_ref()
        .and_then(|identifiers| identifiers.open_library.as_ref().and_then(|ol| ol.first()));

    if let Some(ol_id) = &ol_id {
        let ol_client = OL_CLIENT.read().await;
        let serie = ol_client.get_serie(ol_id).await;
        match serie {
            Ok(Some(serie)) => {
                use annas_archive_api::Serie;

                details.serie = Some(Serie {
                    name: serie.name,
                    position: serie.position,
                    seed_count: serie.seed_count,
                });
            }
            Err(e) => {
                debug!("{e}");
            }
            _ => (),
        }
    }

    Ok(details)
}
