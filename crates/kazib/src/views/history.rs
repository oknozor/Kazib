use dioxus::prelude::*;
use crate::{DownloadHistoryEntry, HistoryStatus, delete_history_entry, get_download_history, update_history_metadata};
use std::collections::HashMap;

#[component]
pub fn History() -> Element {
    let mut history = use_resource(|| async move {
        get_download_history().await
    });

    let mut filter = use_signal(|| "all".to_string());
    let mut edit_entry = use_signal(|| None::<DownloadHistoryEntry>);

    let mut refresh = move |_| {
        history.restart();
    };

    rsx! {
        div { class: "history-container",
            h1 { "Download History" }

            div { class: "history-controls",
                button {
                    class: if filter() == "all" { "filter-btn active" } else { "filter-btn" },
                    onclick: move |_| filter.set("all".to_string()),
                    "All"
                }
                button {
                    class: if filter() == "success" { "filter-btn active" } else { "filter-btn" },
                    onclick: move |_| filter.set("success".to_string()),
                    "Success"
                }
                button {
                    class: if filter() == "pending" { "filter-btn active" } else { "filter-btn" },
                    onclick: move |_| filter.set("pending".to_string()),
                    "Pending"
                }
                button {
                    class: if filter() == "error" { "filter-btn active" } else { "filter-btn" },
                    onclick: move |_| filter.set("error".to_string()),
                    "Error"
                }
            }

            match &*history.read() {
                Some(Ok(entries)) => {
                    let filtered_entries: Vec<_> = entries.iter()
                        .filter(|entry| {
                            let current_filter = filter();
                            if current_filter == "all" {
                                true
                            } else {
                                match &entry.status {
                                    HistoryStatus::Success { .. } => current_filter == "success",
                                    HistoryStatus::Pending { .. } => current_filter == "pending",
                                    HistoryStatus::Error { .. } => current_filter == "error",
                                }
                            }
                        })
                        .cloned()
                        .collect();

                    rsx! {
                        if filtered_entries.is_empty() {
                            div { class: "empty-state", "No downloads in this category" }
                        } else {
                            for entry in filtered_entries {
                                HistoryEntry {
                                    entry: entry.clone(),
                                    on_delete: move |md5: String| {
                                        spawn(async move {
                                            let _ = delete_history_entry(md5, false).await;
                                        });
                                        refresh(());
                                    },
                                    on_edit: move |e: DownloadHistoryEntry| {
                                        edit_entry.set(Some(e));
                                    }
                                }
                            }
                        }
                    }
                },
                Some(Err(err)) => rsx! {
                    div { class: "error-container", "Error loading history: {err}" }
                },
                None => rsx! {
                    div { class: "loading-container", "Loading..." }
                }
            }

            if let Some(entry) = edit_entry() {
                EditMetadataModal {
                    entry: entry.clone(),
                    on_close: move |_| {
                        edit_entry.set(None);
                        refresh(());
                    }
                }
            }
        }
    }
}

#[component]
fn HistoryEntry(
    entry: ReadSignal<DownloadHistoryEntry>,
    on_delete: EventHandler<String>,
    on_edit: EventHandler<DownloadHistoryEntry>,
) -> Element {
    let entry_data = entry.read();
    let status_class = match &entry_data.status {
        HistoryStatus::Success { .. } => "status-success",
        HistoryStatus::Pending { .. } => "status-pending",
        HistoryStatus::Error { .. } => "status-error",
    };

    rsx! {
        div { class: "history-entry",
            if let Some(ref cover_url) = entry_data.item_details.cover_url {
                img { class: "history-cover", src: "{cover_url}", alt: "{entry_data.item_details.title}" }
            }

            div { class: "history-content",
                h3 { class: "history-title", "{entry_data.item_details.title}" }
                if let Some(ref author) = entry_data.item_details.author {
                    p { class: "history-author", "{author}" }
                }

                div { class: "history-metadata",
                    span { class: "history-date", "{entry_data.download_date}" }
                    span { class: "status-badge {status_class}",
                        match &entry_data.status {
                            HistoryStatus::Success { .. } => "✓ Success",
                            HistoryStatus::Pending { .. } => "⏳ Pending",
                            HistoryStatus::Error { .. } => "⚠ Error",
                        }
                    }
                }

                match &entry_data.status {
                    HistoryStatus::Success { resolved_path } => rsx! {
                        p { class: "history-path", "📁 {resolved_path}" }
                    },
                    HistoryStatus::Pending { missing_fields, temp_path } => rsx! {
                        p { class: "history-temp-path", "Temporary: {temp_path}" }
                        div { class: "missing-fields",
                            p { "Missing metadata:" }
                            for field in missing_fields {
                                span { class: "missing-field-badge", "{field.variable}" }
                            }
                        }
                    },
                    HistoryStatus::Error { message } => rsx! {
                        p { class: "history-error", "Error: {message}" }
                    }
                }
            }

            div { class: "history-actions",
                {
                    let entry_clone = entry_data.clone();
                    match &entry_data.status {
                        HistoryStatus::Pending { .. } | HistoryStatus::Error { .. } => rsx! {
                            button {
                                class: "btn-edit",
                                onclick: move |_| {
                                    on_edit.call(entry_clone.clone());
                                },
                                "✏️ Edit Metadata"
                            }
                        },
                        _ => rsx! {}
                    }
                }

                {
                    let md5 = entry_data.md5.clone();
                    rsx! {
                        button {
                            class: "btn-delete",
                            onclick: move |_| {
                                on_delete.call(md5.clone());
                            },
                            "🗑️ Delete"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EditMetadataModal(
    entry: DownloadHistoryEntry,
    on_close: EventHandler<()>,
) -> Element {
    let mut title = use_signal(|| entry.item_details.title.clone());
    let mut author = use_signal(|| entry.item_details.author.clone().unwrap_or_default());
    let mut series = use_signal(|| entry.item_details.series.clone().unwrap_or_default());
    let mut language = use_signal(|| entry.item_details.language.clone().unwrap_or_default());
    let mut year = use_signal(|| entry.item_details.year.clone().unwrap_or_default());
    let mut ext = use_signal(|| entry.item_details.format.clone().unwrap_or_default());
    let mut saving = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    let missing_fields: Vec<String> = match &entry.status {
        HistoryStatus::Pending { missing_fields, .. } => {
            missing_fields.iter().map(|f| f.variable.clone()).collect()
        }
        _ => Vec::new(),
    };

    let handle_save = move |_| {
        saving.set(true);
        error_msg.set(None);

        let mut metadata = HashMap::new();
        metadata.insert("title".to_string(), title());
        metadata.insert("author".to_string(), author());
        metadata.insert("series".to_string(), series());
        metadata.insert("language".to_string(), language());
        metadata.insert("year".to_string(), year());
        metadata.insert("ext".to_string(), ext());

        let md5 = entry.md5.clone();

        spawn(async move {
            match update_history_metadata(md5, metadata).await {
                Ok(_) => {
                    on_close.call(());
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to update: {}", e)));
                    saving.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: "modal-overlay",
            onclick: move |_| on_close.call(()),

            div { class: "modal-content",
                onclick: move |e| e.stop_propagation(),

                h2 { "Edit Metadata" }
                p { class: "modal-subtitle", "Fill in missing fields to resolve the download path" }

                if let Some(err) = error_msg() {
                    div { class: "error-message", "{err}" }
                }

                div { class: "form-group",
                    label {
                        class: if missing_fields.contains(&"title".to_string()) { "required" } else { "" },
                        "Title"
                        if missing_fields.contains(&"title".to_string()) {
                            span { class: "required-badge", " *" }
                        }
                    }
                    input {
                        r#type: "text",
                        value: "{title}",
                        oninput: move |e| title.set(e.value()),
                    }
                }

                div { class: "form-group",
                    label {
                        class: if missing_fields.contains(&"author".to_string()) { "required" } else { "" },
                        "Author"
                        if missing_fields.contains(&"author".to_string()) {
                            span { class: "required-badge", " *" }
                        }
                    }
                    input {
                        r#type: "text",
                        value: "{author}",
                        oninput: move |e| author.set(e.value()),
                    }
                }

                div { class: "form-group",
                    label {
                        class: if missing_fields.contains(&"series".to_string()) { "required" } else { "" },
                        "Series"
                        if missing_fields.contains(&"series".to_string()) {
                            span { class: "required-badge", " *" }
                        }
                    }
                    input {
                        r#type: "text",
                        value: "{series}",
                        oninput: move |e| series.set(e.value()),
                    }
                }

                div { class: "form-group",
                    label {
                        class: if missing_fields.contains(&"language".to_string()) { "required" } else { "" },
                        "Language"
                        if missing_fields.contains(&"language".to_string()) {
                            span { class: "required-badge", " *" }
                        }
                    }
                    input {
                        r#type: "text",
                        value: "{language}",
                        oninput: move |e| language.set(e.value()),
                    }
                }

                div { class: "form-group",
                    label {
                        class: if missing_fields.contains(&"year".to_string()) { "required" } else { "" },
                        "Year"
                        if missing_fields.contains(&"year".to_string()) {
                            span { class: "required-badge", " *" }
                        }
                    }
                    input {
                        r#type: "text",
                        value: "{year}",
                        oninput: move |e| year.set(e.value()),
                    }
                }

                div { class: "form-group",
                    label {
                        class: if missing_fields.contains(&"ext".to_string()) { "required" } else { "" },
                        "Format"
                        if missing_fields.contains(&"ext".to_string()) {
                            span { class: "required-badge", " *" }
                        }
                    }
                    input {
                        r#type: "text",
                        value: "{ext}",
                        oninput: move |e| ext.set(e.value()),
                    }
                }

                div { class: "modal-actions",
                    button {
                        class: "btn-cancel",
                        onclick: move |_| on_close.call(()),
                        disabled: saving(),
                        "Cancel"
                    }
                    button {
                        class: "btn-save",
                        onclick: handle_save,
                        disabled: saving(),
                        if saving() { "Saving..." } else { "Save & Resolve Path" }
                    }
                }
            }
        }
    }
}

