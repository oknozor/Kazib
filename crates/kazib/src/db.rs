use std::collections::HashMap;
use std::fs;
use redb::{Database, TableDefinition};
use std::path::{Path, PathBuf};
use annas_archive_api::ItemDetails;
use crate::AppSettings;
use crate::path_template::{PathTemplate, TemplateResult};

const API_KEY_TABLE: TableDefinition<&str, &str> = TableDefinition::new("api_keys");
const API_KEY_NAME: &str = "annas_archive";

const SETTINGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("settings");
const SETTINGS_KEY: &str = "settings";

const DOWNLOAD_PATH_TEMPLATE_KEY: &str = "download_path_template";

pub fn init_db(path: &Path) -> Result<Database, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    let db = Database::create(path)?;

    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(API_KEY_TABLE);
        let _ = write_txn.open_table(SETTINGS_TABLE);
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
            unreachable!("replace me with proper error handling");
        }
    }

    pub fn download_path(&self, item: &ItemDetails) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let template = &self.download_path_template;
        let mut metadata = HashMap::new();
        metadata.insert("title".into(), item.title.clone());

        if let Some(author) = &item.author {
            metadata.insert("author".into(), author.clone());
        };

        if let Some(series) = &item.series {
            metadata.insert("series".into(), series.clone());
        };

        if let Some(language) = &item.language {
            metadata.insert("language".into(), language.clone());
        };

        if let Some(ext) = &item.format {
            metadata.insert("ext".into(), ext.clone());
        };

        if let Some(year) = &item.year {
            metadata.insert("year".into(), year.clone());
        };

        let TemplateResult::Path(path) = PathTemplate::resolve(template, &metadata) else {
            panic!("No path template found + Please replace this panic with error handling");
        };

        let path = PathBuf::from(path);
        
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        Ok(path)
    }
}
