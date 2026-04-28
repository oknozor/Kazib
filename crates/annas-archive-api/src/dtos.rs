use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DetailsDto {
    pub id: String,
    pub file_unified_data: UnifiedData,
    pub additional: Option<Additional>,
}

#[derive(Debug, Deserialize)]
pub struct UnifiedData {
    pub title_best: String,
    pub author_best: Option<String>,
    pub extension_best: Option<String>,
    pub filesize_best: Option<u64>,
    pub language_codes: Vec<String>,
    pub publisher_best: Option<String>,
    pub year_best: Option<String>,
    pub stripped_description_best: Option<String>,
    pub cover_url_best: Option<String>,
    pub content_type_best: Option<String>,
    pub original_filename_best: Option<String>,
    pub added_date_best: Option<String>,
    pub pages_best: Option<String>,
    pub edition_variant_best: Option<String>,
    pub series_best: Option<String>,
    pub identifiers_unified: Option<IdentifiersUnified>,
    pub classification_unified: Option<ClassificationUnified>,
    pub ipfs_infos: Option<Vec<IpfsInfoDto>>,
}

#[derive(Debug, Deserialize)]
pub struct ClassificationUnified {
    pub filesize_bytes: Option<Vec<String>>,
    pub year: Option<Vec<String>>,
    pub date_zlib_source: Option<Vec<String>>,
    pub date_lgli_source: Option<Vec<String>>,
    pub date_lgrsfic_source: Option<Vec<String>>,
    pub zlib_category_id: Option<Vec<String>>,
    pub zlib_category_name: Option<Vec<String>>,
    pub content_type: Option<Vec<String>>,
    pub torrent: Option<Vec<String>>,
    pub collection: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct IdentifiersUnified {
    pub aarecord_id: Option<Vec<String>>,
    pub isbn10: Option<Vec<String>>,
    pub isbn13: Option<Vec<String>>,
    pub doi: Option<Vec<String>>,
    pub asin: Option<Vec<String>>,
    pub blake2b: Option<Vec<String>>,
    pub ol: Option<Vec<String>>,
    pub googlebookid: Option<Vec<String>>,
    pub goodreads: Option<Vec<String>>,
    pub amazon: Option<Vec<String>>,
    pub ipfs_cid: Option<Vec<String>>,
    pub filepath: Option<Vec<String>>,
    pub lgrsfic: Option<Vec<String>>,
    pub md5: Option<Vec<String>>,
    pub sha1: Option<Vec<String>>,
    pub sha256: Option<Vec<String>>,
    pub crc32: Option<Vec<String>>,
    pub lgli: Option<Vec<String>>,
    pub lgli_fiction_id: Option<Vec<String>>,
    pub zlib: Option<Vec<String>>,
    pub aacid: Option<Vec<String>>,
    pub server_path: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct IpfsInfoDto {
    pub ipfs_cid: Option<String>,
    pub from: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Additional {
    pub download_urls: Vec<Vec<String>>,
    pub torrent_paths: Vec<TorrentPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TorrentPath {
    pub collection: Option<String>,
    pub torrent_path: Option<String>,
    pub file_level1: Option<String>,
    pub file_level2: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenLibraryDetails {
    pub series: Option<Vec<OpenLibrarySerieList>>,
}

#[derive(Debug, Deserialize)]
pub struct OpenLibrarySerieList {
    pub series: OpenLibrarySerieEntry,
    pub position: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenLibrarySerieEntry {
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenLibrarySerie {
    pub name: String,
    pub seed_count: u64,
    #[serde(skip)]
    pub position: String,
}
