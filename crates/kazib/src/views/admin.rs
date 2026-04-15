use dioxus::prelude::*;

use crate::AppSettings;

#[component]
pub fn Settings() -> Element {
    let mut api_key_input = use_signal(String::new);
    let mut download_folder_input = use_signal(String::new);
    let mut auth_header_input = use_signal(|| "x-authentik-username".to_string());
    let mut status_message = use_signal(String::new);
    let mut is_loading = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            match get_settings().await {
                Ok(settings) => {
                    if let Some(api_key) = settings.api_key {
                        api_key_input.set(api_key);
                    }
                    download_folder_input.set(settings.download_path_template);
                    auth_header_input.set(settings.auth_header_name);
                }
                Err(err) => {
                    status_message.set(format!("Error loading settings: {}", err));
                }
            }
        });
    });

    let mut handle_save_settings = move |_| {
        let api_key = api_key_input();
        let auth_header = auth_header_input();

        let settings = AppSettings {
            api_key: if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            },
            download_path_template: download_folder_input(),
            auth_header_name: if auth_header.is_empty() {
                "x-authentik-username".to_string()
            } else {
                auth_header
            },
        };

        if settings.download_path_template.is_empty() {
            status_message.set("Error: Download folder cannot be empty".to_string());
            return;
        }

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
                        oninput: move |e| api_key_input.set(e.value()),
                        disabled: is_loading(),
                    }
                }

                div { class: "settings-section",

                    h2 { "Download Folder" }
                    p { "Set the default folder where downloaded books will be saved" }

                    input {
                        r#type: "text",
                        placeholder: "Enter download folder path",
                        value: "{download_folder_input}",
                        oninput: move |e| download_folder_input.set(e.value()),
                        disabled: is_loading(),
                    }
                }

                div { class: "settings-section",

                    h2 { "Authentication Header" }
                    p { "Header name to extract username from (for reverse proxy authentication)" }
                    p { class: "help-text", "Examples: x-authentik-username, Remote-User, X-Forwarded-User" }

                    input {
                        r#type: "text",
                        placeholder: "x-authentik-username",
                        value: "{auth_header_input}",
                        oninput: move |e| auth_header_input.set(e.value()),
                        disabled: is_loading(),
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
    use crate::{CLIENT, DATABASE};
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
async fn get_settings() -> Result<AppSettings> {
    use crate::DATABASE;
    use dioxus::CapturedError;

    let db = DATABASE.clone();
    AppSettings::get(&db).map_err(CapturedError::from_display)
}
