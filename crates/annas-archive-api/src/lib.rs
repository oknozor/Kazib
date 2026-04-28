#[cfg(feature = "server")]
mod client;

#[cfg(feature = "server")]
mod scraper;

mod dtos;
mod error;
mod types;

#[cfg(feature = "server")]
pub use client::{AnnasArchiveClient, OpenLibraryClient};

pub use error::Error;
pub use types::{
    ContentType, DownloadInfo, DownloadSource, Identifiers, IpfsInfo, ItemDetails, Lang,
    SearchOptions, SearchResponse, SearchResult, Serie,
};
