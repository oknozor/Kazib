use crate::model::AppSettings;
use annas_archive_api::AnnasArchiveClient;
use dioxus::{CapturedError, fullstack::Lazy};
pub use errors::ServerResult;
use redb::Database;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod db;
pub mod errors;
pub mod path_template;

pub static DATABASE: Lazy<Arc<Database>> = Lazy::new(async move || {
    let data_dir = std::env::var("KAZIB_DATA_DIR").unwrap_or_else(|_| ".".to_string());
    let db_path = std::path::Path::new(&data_dir).join("kazib.db");
    let db = db::init_db(&db_path).map_err(CapturedError::from_display)?;
    Ok::<Arc<Database>, CapturedError>(Arc::new(db))
});

pub static CLIENT: Lazy<Arc<RwLock<AnnasArchiveClient>>> = Lazy::new(async move || {
    let db = DATABASE.clone();
    let settings = AppSettings::get(&db).map_err(CapturedError::from_display)?;

    let archive_urls = if settings.archive_urls.is_empty() {
        vec!["annas-archive.gl".to_string()]
    } else {
        settings.archive_urls.clone()
    };

    let client = AnnasArchiveClient::new_with_domains(archive_urls, settings.api_key);

    Ok::<Arc<RwLock<AnnasArchiveClient>>, CapturedError>(Arc::new(RwLock::new(client)))
});
