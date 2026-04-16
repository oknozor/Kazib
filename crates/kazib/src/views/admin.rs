use dioxus::prelude::*;

use crate::AppSettings;
use crate::model::Library;
use super::get_current_user;

#[component]
pub fn Settings() -> Element {
    let mut api_key_input = use_signal(String::new);
    let mut auth_header_input = use_signal(|| "x-authentik-username".to_string());
    let mut libraries_input = use_signal(Vec::<Library>::new);
    let mut detected_username = use_signal(|| None::<String>);
    let mut new_library_name = use_signal(String::new);
    let mut new_library_path_template = use_signal(String::new);
    let mut status_message = use_signal(String::new);
    let mut is_loading = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            match get_settings().await {
                Ok(settings) => {
                    if let Some(api_key) = settings.api_key {
                        api_key_input.set(api_key);
                    }
                    auth_header_input.set(settings.auth_header_name);
                    libraries_input.set(settings.libraries);
                }
                Err(err) => {
                    status_message.set(format!("Error loading settings: {}", err));
                }
            }
            if let Ok(username) = get_current_user().await {
                detected_username.set(username);
            }
        });
    });

    let add_library = move |_| {
        let name = new_library_name().trim().to_string();
        let path_template = new_library_path_template().trim().to_string();

        if name.is_empty() || path_template.is_empty() {
            status_message.set("Error: Library name and path template cannot be empty".to_string());
            return;
        }

        if libraries_input().iter().any(|lib| lib.name == name) {
            status_message.set("Error: Library name already exists".to_string());
            return;
        }

        libraries_input.with_mut(|libs| {
            libs.push(Library {
                name,
                path_template,
            });
        });
        new_library_name.set(String::new());
        new_library_path_template.set(String::new());
    };

    let mut remove_library = move |name: String| {
        libraries_input.with_mut(|libs| {
            libs.retain(|lib| lib.name != name);
        });
    };

    let mut handle_save_settings = move |_| {
        let api_key = api_key_input();
        let auth_header = auth_header_input();
        let libraries = libraries_input();

        // Validate that at least one library exists and has a non-empty path_template
        if libraries.is_empty() {
            status_message.set("Error: At least one library is required".to_string());
            return;
        }

        if libraries.iter().any(|lib| lib.path_template.is_empty()) {
            status_message.set("Error: Library path template cannot be empty".to_string());
            return;
        }

        let settings = AppSettings {
            api_key: if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            },
            auth_header_name: if auth_header.is_empty() {
                "x-authentik-username".to_string()
            } else {
                auth_header
            },
            libraries,
        };

        spawn({
            async move {
                is_loading.set(true);
                match save_settings(settings).await {
                    Ok(_) => {
                        status_message.set("Settings saved successfully!".to_string());
                    }
                    Err(err) => {
                        status_message.set(format!("Error: {}", err));
                    }
                }
                is_loading.set(false);
            }
        });
    };

    rsx! {
        div { id: "admin", class: "admin-container",

            h1 { "Settings" }

            form {
                onsubmit: move |e| {
                    e.prevent_default();
                    handle_save_settings(());
                },

                div { class: "settings-section",

                    h2 { "Anna's Archive API Key" }
                    p { "Set your API key to enable downloads" }

                    input {
                        r#type: "password",
                        placeholder: "Enter your API key",
                        value: "{api_key_input}",
                        disabled: is_loading(),
                        oninput: move |e| {
                            api_key_input.set(e.value());
                        },
                    }
                }

                div { class: "settings-section",

                    h2 { "Authentication Header" }
                    p { "Header name to extract username from (for reverse proxy authentication)" }
                    p { class: "help-text",
                        "Examples: x-authentik-username, Remote-User, X-Forwarded-User"
                    }

                    input {
                        r#type: "text",
                        placeholder: "x-authentik-username",
                        value: "{auth_header_input}",
                        oninput: move |e| auth_header_input.set(e.value()),
                        disabled: is_loading(),
                    }

                    match detected_username() {
                        Some(username) => rsx! {
                            p { class: "help-text",
                                "Detected username: "
                                strong { "{username}" }
                            }
                        },
                        None => rsx! {
                            p { class: "help-text", "No username detected" }
                        },
                    }
                }

                div { class: "settings-section",

                    h2 { "Libraries" }
                    p { "Define named libraries to organize downloads by category" }
                    p { class: "help-text",
                        "Use path templates with variables and operators to control where files are saved."
                    }

                    details { class: "template-reference",
                        summary { "Template Reference" }

                        div { class: "template-reference-content",
                            h4 { "Available Variables" }
                            table {
                                thead {
                                    tr {
                                        th { "Variable" }
                                        th { "Description" }
                                        th { "Availability" }
                                    }
                                }
                                tbody {
                                    tr { td { code { "title" } } td { "Book title" } td { "Always" } }
                                    tr { td { code { "author" } } td { "Author name" } td { "Optional" } }
                                    tr { td { code { "series" } } td { "Series name" } td { "Optional" } }
                                    tr { td { code { "series_number" } } td { "Position in series" } td { "Optional" } }
                                    tr { td { code { "language" } } td { "Language code (en, fr...)" } td { "Optional" } }
                                    tr { td { code { "year" } } td { "Publication year" } td { "Optional" } }
                                    tr { td { code { "ext" } } td { "File extension (epub, pdf...)" } td { "Optional" } }
                                }
                            }

                            h4 { "Template Operators" }
                            table {
                                thead {
                                    tr {
                                        th { "Syntax" }
                                        th { "Description" }
                                        th { "Example" }
                                    }
                                }
                                tbody {
                                    tr {
                                        td { code { "{{name}}" } }
                                        td { "Required variable (download fails if missing)" }
                                        td { code { "{{author}}" } " \u{2192} Tolkien" }
                                    }
                                    tr {
                                        td { code { "{{name:default}}" } }
                                        td { "Fallback value if variable is missing" }
                                        td { code { "{{series:Standalone}}" } " \u{2192} Standalone" }
                                    }
                                    tr {
                                        td { code { "{{name/}}" } }
                                        td { "Optional path segment (skipped if missing, adds /)" }
                                        td { code { "{{language/}}" } " \u{2192} en/ or nothing" }
                                    }
                                    tr {
                                        td { code { "{{name:default/}}" } }
                                        td { "Fallback + optional path segment" }
                                        td { code { "{{series:_oneshots/}}" } }
                                    }
                                    tr {
                                        td { code { "{{?name}}...{{/name}}" } }
                                        td { "Conditional block (only rendered if variable exists)" }
                                        td { code { "{{?series}}{{series}} #{{series_number}} - {{/series}}" } }
                                    }
                                }
                            }

                            h4 { "Examples" }
                            div { class: "template-examples",
                                p {
                                    strong { "Simple: " }
                                    code { "/books/{{author}}/{{title}}.{{ext}}" }
                                }
                                p {
                                    strong { "With optional language: " }
                                    code { "/books/{{language/}}{{author}}/{{title}}.{{ext}}" }
                                }
                                p {
                                    strong { "With series prefix: " }
                                    code { "/books/{{author}}/{{?series}}{{series}} - {{series_number}} - {{/series}}{{title}}.{{ext}}" }
                                }
                                p {
                                    strong { "Complete: " }
                                    code { "/ebooks/{{language}}/{{author}}/{{series:_oneshots}}/{{?series}}{{series}} - {{series_number}} - {{/series}}{{title}}.{{ext}}" }
                                }
                            }
                        }
                    }

                    div { class: "libraries-list",
                        for library in libraries_input() {
                            div { class: "library-item",
                                span { class: "library-name", "{library.name}" }
                                span { class: "library-separator", "→" }
                                span { class: "library-path", "{library.path_template}" }
                                button {
                                    class: "btn-remove-library",
                                    onclick: move |_| remove_library(library.name.clone()),
                                    "✕"
                                }
                            }
                        }
                    }

                    div { class: "add-library-form",
                        h3 { "Add Library" }
                        div { class: "library-input-group",
                            input {
                                r#type: "text",
                                placeholder: "Library name (e.g., Novels)",
                                value: "{new_library_name}",
                                oninput: move |e| new_library_name.set(e.value()),
                                disabled: is_loading(),
                            }
                            input {
                                r#type: "text",
                                placeholder: "Path template (e.g., /books/{{author}}/{{title}}.{{ext}})",
                                value: "{new_library_path_template}",
                                oninput: move |e| new_library_path_template.set(e.value()),
                                disabled: is_loading(),
                            }
                            button {
                                class: "btn-add-library",
                                onclick: add_library,
                                disabled: is_loading() || new_library_name().is_empty()
                                    || new_library_path_template().is_empty(),
                                "+"
                            }
                        }
                    }
                }

                div { class: "settings-actions",

                    button { r#type: "submit", disabled: is_loading(),
                        if is_loading() {
                            "Saving..."
                        } else {
                            "Save Settings"
                        }
                    }
                }
            }

            if !status_message().is_empty() {
                div { class: "status-message", "{status_message}" }
            }
        }
    }
}

#[post("/save-settings")]
async fn save_settings(settings: AppSettings) -> Result<()> {
    use crate::server::{CLIENT, DATABASE};
    use annas_archive_api::AnnasArchiveClient;
    use dioxus::CapturedError;

    let db = DATABASE.clone();
    settings.save(&db).map_err(CapturedError::from_display)?;

    if settings.api_key.is_some() {
        *CLIENT.write().expect("failed to acquire write lock") =
            AnnasArchiveClient::new("annas-archive.gl".to_string(), settings.api_key);
    }

    Ok(())
}

#[get("/get-settings")]
pub async fn get_settings() -> Result<AppSettings> {
    use crate::server::DATABASE;
    use dioxus::CapturedError;

    let db = DATABASE.clone();
    AppSettings::get(&db).map_err(CapturedError::from_display)
}
