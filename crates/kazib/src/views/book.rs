use annas_archive_api::{Identifiers, ItemDetails};
use dioxus::fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;

use crate::model::DownloadProgress;
use crate::views::{download_book, get_current_user};

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
    let mut download_state = use_signal(|| None::<DownloadProgress>);
    let mut ws_socket: Signal<Option<Websocket<(), DownloadProgress>>> = use_signal(|| None);

    let handle_download = move |_| {
        let md5 = md5.clone();

        spawn(async move {
            let username = get_current_user().await.ok().flatten();

            if let Ok(socket) = download_book(md5, username, WebSocketOptions::new()).await {
                ws_socket.set(Some(socket));

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

                    h1 { "{details.title}" }

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

                    // Download button
                    div { class: "book-download-button",
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

                if let Some(ref series) = details.series {
                    p {
                        strong { "Series: " }
                        "{series}"
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
    use crate::CLIENT;
    use dioxus::CapturedError;

    CLIENT
        .read()
        .unwrap()
        .get_details(&md5)
        .await
        .map_err(CapturedError::from_display)
}
