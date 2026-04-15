use annas_archive_api::ItemDetails;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub api_key: Option<String>,
    pub download_path_template: String,
    pub auth_header_name: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_path_template = dirs::download_dir()
            .or(dirs::document_dir())
            .or(dirs::data_dir())
            .or(dirs::home_dir())
            .expect("failed to get default download location");

        let download_path_template = download_path_template.join("Kazib");
        let download_path_template = download_path_template.to_string_lossy().into_owned();

        Self {
            api_key: None,
            download_path_template,
            auth_header_name: "x-authentik-username".to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DownloadHistoryEntry {
    pub md5: String,
    pub item_details: ItemDetails,
    pub status: HistoryStatus,
    pub download_date: String,
    pub file_path: Option<String>,
    pub error_details: Option<String>,
    pub username: Option<String>,
}

/// A missing metadata field (shared between client and server)
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct MissingField {
    pub variable: String,
    pub description: String,
}

/// Three-state filter for format filtering
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug, Default)]
pub enum FilterState {
    #[default]
    Off,      // Not filtered
    Include,  // Include this format (✓)
    Exclude,  // Exclude this format (✗)
}

impl FilterState {
    pub fn cycle(self) -> Self {
        match self {
            FilterState::Off => FilterState::Include,
            FilterState::Include => FilterState::Exclude,
            FilterState::Exclude => FilterState::Off,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            FilterState::Off => "",
            FilterState::Include => "✓",
            FilterState::Exclude => "✗",
        }
    }
}

/// Supported file formats for filtering
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Display, EnumIter, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum FileFormat {
    #[strum(serialize = "PDF")]
    Pdf,
    #[strum(serialize = "EPUB")]
    Epub,
    #[strum(serialize = "MOBI")]
    Mobi,
    #[strum(serialize = "AZW3")]
    Azw3,
    #[strum(serialize = "DjVu")]
    Djvu,
    #[strum(serialize = "FB2")]
    Fb2,
    #[strum(serialize = "CBZ")]
    Cbz,
    #[strum(serialize = "CBR")]
    Cbr,
    #[strum(serialize = "TXT")]
    Txt,
}

impl FileFormat {
    /// Get lowercase format string for API calls
    pub fn as_str(&self) -> &str {
        match self {
            FileFormat::Pdf => "pdf",
            FileFormat::Epub => "epub",
            FileFormat::Mobi => "mobi",
            FileFormat::Azw3 => "azw3",
            FileFormat::Djvu => "djvu",
            FileFormat::Fb2 => "fb2",
            FileFormat::Cbz => "cbz",
            FileFormat::Cbr => "cbr",
            FileFormat::Txt => "txt",
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum DownloadProgress {
    Started,
    Progress {
        md5: String,
        downloaded: u64,
        total: u64,
        percent: f32,
    },
    Completed {
        md5: String,
        file_path: String,
    },
    Error {
        md5: String,
        error: String,
    },
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum HistoryStatus {
    Success {
        resolved_path: String,
    },
    Pending {
        missing_fields: Vec<MissingField>,
        temp_path: String,
    },
    Error {
        message: String,
    },
}
