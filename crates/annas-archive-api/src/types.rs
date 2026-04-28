use serde::{Deserialize, Serialize};
use strum::{Display as StrumDisplay, EnumIter, EnumString, IntoEnumIterator};

use crate::dtos::{DetailsDto, IdentifiersUnified, IpfsInfoDto, TorrentPath};

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
    pub lang: Option<Lang>,           // Deprecated: use lang_filters instead
    pub ext_filters: Vec<String>,     // e.g., ["pdf", "epub", "anti_mobi"]
    pub lang_filters: Vec<String>,    // e.g., ["en", "fr", "anti_es"]
    pub content_filters: Vec<String>, // e.g., ["book_nonfiction", "anti__book_fiction"]
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    StrumDisplay,
    EnumIter,
    EnumString,
)]
#[strum(serialize_all = "lowercase")]
pub enum Lang {
    #[strum(serialize = "English")]
    En,
    #[strum(serialize = "French")]
    Fr,
    #[strum(serialize = "Spanish")]
    Es,
    #[strum(serialize = "German")]
    De,
    #[strum(serialize = "Italian")]
    It,
    #[strum(serialize = "Portuguese")]
    Pt,
    #[strum(serialize = "Russian")]
    Ru,
    #[strum(serialize = "Chinese")]
    Zh,
    #[strum(serialize = "Japanese")]
    Ja,
}

impl Lang {
    /// Get lowercase language code for API calls
    pub fn as_str(&self) -> &str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::It => "it",
            Lang::Pt => "pt",
            Lang::Ru => "ru",
            Lang::Zh => "zh",
            Lang::Ja => "ja",
        }
    }

    /// Primary languages shown by default
    pub const PRIMARY: &'static [Lang] = &[Lang::En, Lang::Fr, Lang::Es, Lang::De];

    /// Secondary languages shown after clicking "more..."
    pub fn secondary() -> Vec<Lang> {
        Lang::iter()
            .filter(|l| !Self::PRIMARY.contains(l))
            .collect()
    }
}

impl From<String> for Lang {
    fn from(s: String) -> Self {
        match s.as_str() {
            "en" => Lang::En,
            "fr" => Lang::Fr,
            "es" => Lang::Es,
            "de" => Lang::De,
            "it" => Lang::It,
            "pt" => Lang::Pt,
            "ru" => Lang::Ru,
            "zh" => Lang::Zh,
            "ja" => Lang::Ja,
            _ => Lang::En,
        }
    }
}

/// Content types for filtering search results
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    StrumDisplay,
    EnumIter,
    EnumString,
)]
#[strum(serialize_all = "lowercase")]
pub enum ContentType {
    #[strum(serialize = "📘 Book (non fiction)")]
    BookNonfiction,
    #[strum(serialize = "📕 Book (fiction)")]
    BookFiction,
    #[strum(serialize = "📗 Book (unknown)")]
    BookUnknown,
    #[strum(serialize = "📰 Magazine")]
    Magazine,
    #[strum(serialize = "💬 Comic book")]
    BookComic,
    #[strum(serialize = "📝 Standard document")]
    StandardsDocument,
    #[strum(serialize = "🎶 Musical score")]
    MusicalScore,
    #[strum(serialize = "🤨 Other")]
    Other,
}

impl ContentType {
    /// Get the content type string for API calls
    pub fn as_str(&self) -> &str {
        match self {
            ContentType::BookNonfiction => "book_nonfiction",
            ContentType::BookFiction => "book_fiction",
            ContentType::BookUnknown => "book_unknown",
            ContentType::Magazine => "magazine",
            ContentType::BookComic => "book_comic",
            ContentType::StandardsDocument => "standards_document",
            ContentType::MusicalScore => "musical_score",
            ContentType::Other => "other",
        }
    }
}

impl SearchOptions {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            page: None,
            lang: None,
            ext_filters: Vec::new(),
            lang_filters: Vec::new(),
            content_filters: Vec::new(),
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

    pub fn with_ext_filters(mut self, filters: Vec<String>) -> Self {
        self.ext_filters = filters;
        self
    }

    pub fn with_lang_filters(mut self, filters: Vec<String>) -> Self {
        self.lang_filters = filters;
        self
    }

    pub fn with_content_filters(mut self, filters: Vec<String>) -> Self {
        self.content_filters = filters;
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

impl From<IpfsInfoDto> for IpfsInfo {
    fn from(value: IpfsInfoDto) -> Self {
        IpfsInfo {
            cid: value.ipfs_cid.unwrap_or_default(),
            from: value.from.unwrap_or_default(),
        }
    }
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
    pub torrent_paths: Option<Vec<TorrentPath>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serie: Option<Serie>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Serie {
    pub name: String,
    pub position: String,
    pub seed_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadInfo {
    pub download_url: String,
}

impl From<DetailsDto> for ItemDetails {
    fn from(value: DetailsDto) -> Self {
        let unified_data = value.file_unified_data;
        let additional = value.additional;

        let download_sources = additional.as_ref().map(|additional| {
            additional
                .download_urls
                .iter()
                .map(|url| DownloadSource {
                    name: url.get(0).cloned().unwrap_or_default(),
                    url: url.get(1).cloned().unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        });

        let ipfs_cids = unified_data.ipfs_infos.map(|ipfs_infos| {
            ipfs_infos
                .into_iter()
                .map(IpfsInfo::from)
                .collect::<Vec<_>>()
        });

        ItemDetails {
            md5: value
                .id
                .strip_prefix("md5:")
                .expect("MD5 missing")
                .to_string(),
            title: unified_data.title_best,
            author: unified_data.author_best,
            format: unified_data.extension_best,
            size: unified_data.filesize_best.clone().map(format_filesize),
            size_bytes: unified_data.filesize_best,
            language: unified_data.language_codes.first().cloned(),
            publisher: unified_data.publisher_best,
            year: unified_data.year_best,
            description: unified_data.stripped_description_best,
            cover_url: unified_data.cover_url_best,
            content_type: unified_data.content_type_best,
            original_filename: unified_data.original_filename_best,
            added_date: unified_data.added_date_best,
            pages: unified_data.pages_best,
            edition: unified_data.edition_variant_best,
            identifiers: unified_data.identifiers_unified.map(Into::into),
            categories: unified_data
                .classification_unified
                .as_ref()
                .and_then(|classification| classification.collection.clone()),
            //TODO
            subjects: unified_data
                .classification_unified
                .and_then(|classification| classification.collection),
            ipfs_cids,
            download_sources,
            torrent_paths: additional.map(|additional| additional.torrent_paths),
            serie: None,
        }
    }
}

impl From<IdentifiersUnified> for Identifiers {
    fn from(value: IdentifiersUnified) -> Self {
        Identifiers {
            isbn10: value.isbn10,
            isbn13: value.isbn13,
            doi: value.doi,
            asin: value.asin,
            sha1: value.sha1.and_then(|v| v.first().cloned()),
            sha256: value.sha256.and_then(|v| v.first().cloned()),
            crc32: value.crc32.and_then(|v| v.first().cloned()),
            blake2b: value.blake2b.and_then(|v| v.first().cloned()),
            open_library: value.ol,
            google_books: value.googlebookid,
            goodreads: value.goodreads,
            amazon: value.amazon,
        }
    }
}

fn format_filesize(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}
