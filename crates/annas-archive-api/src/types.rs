use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub md5: String,
    pub title: String,
    pub author: Option<String>,
    pub format: Option<String>,
    pub size: Option<String>,
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub query: String,
    pub page: Option<u32>,
    pub lang: Option<Lang>,
}

#[derive(Debug, Clone)]
pub enum Lang {
    En,
    Fr,
}

impl From<String> for Lang {
    fn from(s: String) -> Self {
        match s.as_str() {
            "en" => Lang::En,
            "fr" => Lang::Fr,
            _ => Lang::En,
        }
    }
}

impl Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lang = match self {
            Lang::En => "en",
            Lang::Fr => "fr",
        };

        write!(f, "{}", lang)
    }
}

impl SearchOptions {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            page: None,
            lang: None,
        }
    }

    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    pub fn with_lang(mut self, lang: Lang) -> Self {
        self.lang = Some(lang);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub page: u32,
    pub has_more: bool,
}

/// Identifiers for an item (ISBN, DOI, etc.)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Identifiers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn10: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn13: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asin: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crc32: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blake2b: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_library: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_books: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goodreads: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amazon: Option<Vec<String>>,
}

/// IPFS availability information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpfsInfo {
    pub cid: String,
    pub from: String,
}

/// Download source information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadSource {
    pub name: String,
    pub url: String,
}

/// Full item details from the JSON API
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemDetails {
    pub md5: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifiers: Option<Identifiers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subjects: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipfs_cids: Option<Vec<IpfsInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_sources: Option<Vec<DownloadSource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub torrent_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadInfo {
    pub download_url: String,
}
