use dioxus::prelude::*;

use super::get_current_user;
use crate::AppSettings;
use crate::model::Library;

#[component]
pub fn Settings() -> Element {
    let mut api_key_input = use_signal(String::new);
    let mut auth_header_input = use_signal(|| "x-authentik-username".to_string());
    let mut archive_urls_input = use_signal(Vec::<String>::new);
    let mut new_archive_url = use_signal(String::new);
    let mut libraries_input = use_signal(Vec::<Library>::new);
    let mut detected_username = use_signal(|| None::<String>);
    let mut file_permissions_input = use_signal(|| "755".to_string());
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
                    archive_urls_input.set(settings.archive_urls);
                    file_permissions_input.set(format!("{:o}", settings.file_permissions));
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

    let add_archive_url = move |_| {
        let url = new_archive_url().trim().to_string();

        if url.is_empty() {
            status_message.set("Error: Archive URL cannot be empty".to_string());
            return;
        }

        if archive_urls_input().iter().any(|u| u == &url) {
            status_message.set("Error: Archive URL already exists".to_string());
            return;
        }

        archive_urls_input.with_mut(|urls| {
            urls.push(url);
        });
        new_archive_url.set(String::new());
    };

    let mut remove_archive_url = move |url: String| {
        archive_urls_input.with_mut(|urls| {
            urls.retain(|u| u != &url);
        });
    };

    let mut handle_save_settings = move |_| {
        let api_key = api_key_input();
        let auth_header = auth_header_input();
        let archive_urls = archive_urls_input();
        let libraries = libraries_input();
        let perms_str = file_permissions_input();

        // Validate that at least one library exists and has a non-empty path_template
        if libraries.is_empty() {
            status_message.set("Error: At least one library is required".to_string());
            return;
        }

        if libraries.iter().any(|lib| lib.path_template.is_empty()) {
            status_message.set("Error: Library path template cannot be empty".to_string());
            return;
        }

        // Validate that at least one archive URL exists
        if archive_urls.is_empty() {
            status_message.set("Error: At least one archive URL is required".to_string());
            return;
        }

        // Parse octal permissions
        let file_permissions = match u32::from_str_radix(&perms_str, 8) {
            Ok(p) if p <= 0o777 => p,
            _ => {
                status_message
                    .set("Error: Invalid file permissions (use octal, e.g. 755)".to_string());
                return;
            }
        };

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
            archive_urls,
            file_permissions,
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

                    h2 { "Anna's Archive URLs" }
                    p { "Configure multiple archive domain URLs for failover support" }
                    p { class: "help-text",
                        "Examples: annas-archive.gl, annas-archive.se, 10.0.0.1:8080"
                    }

                    div { class: "archive-urls-list",
                        for url in archive_urls_input() {
                            div { class: "archive-url-item",
                                span { class: "archive-url", "{url}" }
                                button {
                                    class: "btn-remove-archive-url",
                                    onclick: move |_| remove_archive_url(url.clone()),
                                    "✕"
                                }
                            }
                        }
                    }

                    div { class: "add-archive-url-form",
                        h3 { "Add Archive URL" }
                        div { class: "archive-url-input-group",
                            input {
                                r#type: "text",
                                placeholder: "Archive URL (e.g., annas-archive.gl)",
                                value: "{new_archive_url}",
                                oninput: move |e| new_archive_url.set(e.value()),
                                disabled: is_loading(),
                            }
                            button {
                                class: "btn-add-archive-url",
                                onclick: add_archive_url,
                                disabled: is_loading() || new_archive_url().is_empty(),
                                "+"
                            }
                        }
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

                    h2 { "File Permissions" }
                    p { "Unix file permissions for downloaded books (octal notation)" }
                    p { class: "help-text",
                        "Examples: 755 (rwxr-xr-x), 644 (rw-r--r--), 600 (rw-------)"
                    }

                    input {
                        r#type: "text",
                        placeholder: "755",
                        value: "{file_permissions_input}",
                        oninput: move |e| file_permissions_input.set(e.value()),
                        disabled: is_loading(),
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

    let archive_urls = if settings.archive_urls.is_empty() {
        vec!["annas-archive.gl".to_string()]
    } else {
        settings.archive_urls.clone()
    };

    let mut client = CLIENT.write().await;
    *client = AnnasArchiveClient::new_with_domains(archive_urls, settings.api_key);

    Ok(())
}

#[get("/get-settings")]
pub async fn get_settings() -> Result<AppSettings> {
    use crate::server::DATABASE;
    use dioxus::CapturedError;

    let db = DATABASE.clone();
    AppSettings::get(&db).map_err(CapturedError::from_display)
}
