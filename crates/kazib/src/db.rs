use crate::AppSettings;
use crate::path_template::{PathTemplate, TemplateResult};
use annas_archive_api::ItemDetails;
use redb::{Database, ReadableTable, TableDefinition};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum TemplateError {
    MissingFields(Vec<MissingField>),
    IoError(std::io::Error),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::MissingFields(fields) => {
                write!(f, "Missing template fields: {:?}", fields)
            }
            TemplateError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for TemplateError {}

impl From<std::io::Error> for TemplateError {
    fn from(error: std::io::Error) -> Self {
        TemplateError::IoError(error)
    }
}

const SETTINGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("settings");
const SETTINGS_KEY: &str = "settings";
const HISTORY_TABLE: TableDefinition<&str, &str> = TableDefinition::new("download_history");

pub fn init_db(path: &Path) -> Result<Database, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    let db = Database::create(path)?;

    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(SETTINGS_TABLE);
        let _ = write_txn.open_table(HISTORY_TABLE);
    }
    write_txn.commit()?;

    Ok(db)
}

impl AppSettings {
    pub fn save(&self, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let write_txn = db.begin_write()?;
        {
            let settings = serde_json::to_string(&self)?;
            let mut table = write_txn.open_table(SETTINGS_TABLE)?;
            table.insert(SETTINGS_KEY, settings.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(db: &Database) -> Result<AppSettings, Box<dyn std::error::Error>> {
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(SETTINGS_TABLE)?;

        if let Some(value) = table.get(SETTINGS_KEY)? {
            let value: &str = value.value();
            let settings = serde_json::from_str::<AppSettings>(value)?;
            Ok(settings)
        } else {
            Ok(AppSettings::default())
        }
    }
}

use crate::model::{DownloadHistoryEntry, MissingField};

impl DownloadHistoryEntry {
    pub fn save(&self, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(self)?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(HISTORY_TABLE)?;
            table.insert(self.md5.as_str(), json.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(md5: &str, db: &Database) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(HISTORY_TABLE)?;

        if let Some(value) = table.get(md5)? {
            let value: &str = value.value();
            let entry = serde_json::from_str::<DownloadHistoryEntry>(value)?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(db: &Database) -> Result<Vec<Self>, Box<dyn std::error::Error>> {
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(HISTORY_TABLE)?;

        let mut entries = Vec::new();
        for item in table.iter()? {
            let (_key, value) = item?;
            let value: &str = value.value();
            let entry = serde_json::from_str::<DownloadHistoryEntry>(value)?;
            entries.push(entry);
        }

        // Sort by date, newest first
        entries.sort_by(|a, b| b.download_date.cmp(&a.download_date));
        Ok(entries)
    }

    pub fn delete(md5: &str, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(HISTORY_TABLE)?;
            table.remove(md5)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}
