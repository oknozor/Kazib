use redb::{Database, TableDefinition};
use std::path::Path;

const API_KEY_TABLE: TableDefinition<&str, &str> = TableDefinition::new("api_keys");
const API_KEY_NAME: &str = "annas_archive";

const SETTINGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("settings");
const DOWNLOAD_FOLDER_KEY: &str = "download_folder";

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

pub fn save_api_key(db: &Database, api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(API_KEY_TABLE)?;
        table.insert(API_KEY_NAME, api_key)?;
    }
    write_txn.commit()?;
    Ok(())
}

pub fn load_api_key(db: &Database) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(API_KEY_TABLE)?;

    if let Some(value) = table.get(API_KEY_NAME)? {
        let key: &str = value.value();
        Ok(Some(key.to_string()))
    } else {
        Ok(None)
    }
}

#[allow(dead_code)]
pub fn delete_api_key(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(API_KEY_TABLE)?;
        table.remove(API_KEY_NAME)?;
    }
    write_txn.commit()?;
    Ok(())
}

pub fn save_download_folder(db: &Database, folder_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(SETTINGS_TABLE)?;
        table.insert(DOWNLOAD_FOLDER_KEY, folder_path)?;
    }
    write_txn.commit()?;
    Ok(())
}

pub fn load_download_folder(db: &Database) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(SETTINGS_TABLE)?;

    if let Some(value) = table.get(DOWNLOAD_FOLDER_KEY)? {
        let folder: &str = value.value();
        Ok(Some(folder.to_string()))
    } else {
        Ok(None)
    }
}
