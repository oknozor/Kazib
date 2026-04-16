use annas_archive_api::{ContentType, ItemDetails, Lang};
use serde::{Deserialize, Serialize};
use std::fmt;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

/// Trait for types that can be used in tri-state filters
pub trait Filterable: Copy + Eq + std::hash::Hash + fmt::Display {
    /// Get the lowercase string for API calls
    fn as_str(&self) -> &str;

    /// Get primary items (shown by default)
    fn primary() -> &'static [Self];

    /// Get secondary items (shown after "more...")
    fn secondary() -> Vec<Self>;
}

/// A library represents a named destination for downloaded books
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Library {
    pub name: String,
    pub path_template: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub api_key: Option<String>,
    pub auth_header_name: String,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default = "default_archive_urls")]
    pub archive_urls: Vec<String>,
    #[serde(default = "default_file_permissions")]
    pub file_permissions: u32,
}

fn default_archive_urls() -> Vec<String> {
    vec!["annas-archive.gl".to_string()]
}

fn default_file_permissions() -> u32 {
    0o755
}

impl Default for AppSettings {
    fn default() -> Self {
        let default_path = dirs::download_dir()
            .or(dirs::document_dir())
            .or(dirs::data_dir())
            .or(dirs::home_dir())
            .expect("failed to get default download location");

        let default_path = default_path.join("Kazib");
        let default_path = default_path.to_string_lossy().into_owned();

        Self {
            api_key: None,
            auth_header_name: "x-authentik-username".to_string(),
            libraries: vec![Library {
                name: "Default".to_string(),
                path_template: default_path,
            }],
            archive_urls: vec!["annas-archive.gl".to_string()],
            file_permissions: default_file_permissions(),
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
    Off, // Not filtered
    Include, // Include this format (✓)
    Exclude, // Exclude this format (✗)
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
#[derive(
    Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Display, EnumIter, EnumString,
)]
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

    /// Primary formats shown by default (before "more..." button)
    pub const PRIMARY: &'static [FileFormat] = &[
        FileFormat::Pdf,
        FileFormat::Epub,
        FileFormat::Cbz,
        FileFormat::Mobi,
        FileFormat::Fb2,
    ];

    /// Secondary formats shown after clicking "more..."
    pub fn secondary() -> Vec<FileFormat> {
        FileFormat::iter()
            .filter(|f| !Self::PRIMARY.contains(f))
            .collect()
    }
}

impl Filterable for FileFormat {
    fn as_str(&self) -> &str {
        FileFormat::as_str(self)
    }

    fn primary() -> &'static [Self] {
        Self::PRIMARY
    }

    fn secondary() -> Vec<Self> {
        FileFormat::secondary()
    }
}

impl Filterable for Lang {
    fn as_str(&self) -> &str {
        Lang::as_str(self)
    }

    fn primary() -> &'static [Self] {
        Lang::PRIMARY
    }

    fn secondary() -> Vec<Self> {
        Lang::secondary()
    }
}

impl Filterable for ContentType {
    fn as_str(&self) -> &str {
        ContentType::as_str(self)
    }

    fn primary() -> &'static [Self] {
        &[
            ContentType::BookNonfiction,
            ContentType::BookFiction,
            ContentType::Magazine,
            ContentType::BookComic,
        ]
    }

    fn secondary() -> Vec<Self> {
        ContentType::iter()
            .filter(|c| !Self::primary().contains(c))
            .collect()
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
