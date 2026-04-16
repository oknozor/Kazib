use crate::model::AppSettings;
use annas_archive_api::AnnasArchiveClient;
use dioxus::{CapturedError, fullstack::Lazy};
use redb::Database;
use std::sync::{Arc, RwLock};

pub mod db;
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

    Ok::<Arc<RwLock<AnnasArchiveClient>>, CapturedError>(Arc::new(RwLock::new(
        AnnasArchiveClient::new("annas-archive.gl".to_string(), settings.api_key),
    )))
});
