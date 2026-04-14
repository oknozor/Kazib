mod client;
mod error;
mod scraper;
mod types;

pub use client::AnnasArchiveClient;
pub use error::Error;
pub use types::{
    DownloadInfo, DownloadSource, Identifiers, IpfsInfo, ItemDetails, Lang, SearchOptions,
    SearchResponse, SearchResult,
};

#[cfg(test)]
mod tests {
    use crate::{AnnasArchiveClient, SearchOptions};

    #[tokio::test]
    async fn search_test() {
        let client = AnnasArchiveClient::new("annas-archive.gl".to_string(), None);
        let response = client.search(SearchOptions::new("rust")).await.unwrap();
        println!("{:?}", response);
    }
}
