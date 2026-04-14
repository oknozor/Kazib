use dioxus::prelude::*;

use crate::{get_settings, save_settings, AppSettings};

#[component]
pub fn SettingsView() -> Element {
    let mut api_key_input = use_signal(|| String::new());
    let mut download_folder_input = use_signal(|| String::new());
    let mut status_message = use_signal(|| String::new());
    let mut is_loading = use_signal(|| false);

    // Load current settings on mount
    use_effect(move || {
        spawn(async move {
            match get_settings().await {
                Ok(settings) => {
                    api_key_input.set(settings.api_key);
                    download_folder_input.set(settings.download_folder);
                }
                Err(err) => {
                    status_message.set(format!("Error loading settings: {}", err));
                }
            }
        });
    });

    let mut handle_save_settings = move |_| {
        let settings = AppSettings {
            api_key: api_key_input(),
            download_folder: download_folder_input(),
        };

        if settings.download_folder.is_empty() {
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
        div {
            id: "admin",
            class: "admin-container",

            h1 { "Settings" }

            form {
                onsubmit: move |e| {
                    e.prevent_default();
                    handle_save_settings(());
                },

                div {
                    class: "settings-section",

                    h2 { "Anna's Archive API Key" }
                    p { "Set your API key to access premium features (optional)" }

                    input {
                        r#type: "password",
                        placeholder: "Enter your API key",
                        value: "{api_key_input}",
                        oninput: move |e| api_key_input.set(e.value()),
                        disabled: is_loading(),
                    }
                }

                div {
                    class: "settings-section",

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

                div {
                    class: "settings-actions",

                    button {
                        r#type: "submit",
                        disabled: is_loading(),
                        if is_loading() {
                            "Saving..."
                        } else {
                            "Save Settings"
                        }
                    }
                }
            }

            if !status_message().is_empty() {
                div {
                    class: "status-message",
                    "{status_message}"
                }
            }
        }
    }
}
