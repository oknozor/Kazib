use reqwest::Client;
use reqwest::cookie::Jar;
use std::sync::Arc;

use crate::dtos::{DetailsDto, OpenLibraryDetails, OpenLibrarySerie};
use crate::error::Error;
use crate::scraper::parse_search_results;
use crate::types::{DownloadInfo, ItemDetails, SearchOptions, SearchResponse};

pub struct AnnasArchiveClient {
    client: Client,
    api_key: Option<String>,
    #[allow(dead_code)] // Used by cookie_provider, but not directly accessed
    cookie_jar: Arc<Jar>,
    domains: Vec<String>,
    authenticated: std::sync::atomic::AtomicBool,
}

pub struct OpenLibraryClient {
    client: Client,
}

impl OpenLibraryClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn get_serie(&self, id: &str) -> Result<Option<OpenLibrarySerie>, reqwest::Error> {
        let url = format!("https://openlibrary.org/works/{id}.json");
        let details: OpenLibraryDetails = self.client.get(url).send().await?.json().await?;

        if let Some(series) = details.series {
            if let Some(serie) = series.first() {
                let id = serie
                    .series
                    .key
                    .strip_prefix("/series/")
                    .unwrap_or(&serie.series.key);

                let position = serie.position.clone();
                let url = format!("https://openlibrary.org/series/{id}.json");
                let mut serie: OpenLibrarySerie = self.client.get(url).send().await?.json().await?;
                serie.position = position;
                Ok(Some(serie))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

impl AnnasArchiveClient {
    pub fn new(domain: String, api_key: Option<String>) -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .cookie_provider(cookie_jar.clone())
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            cookie_jar,
            authenticated: std::sync::atomic::AtomicBool::new(false),
            domains: vec![domain],
        }
    }

    pub fn new_with_domains(domains: Vec<String>, api_key: Option<String>) -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .cookie_provider(cookie_jar.clone())
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            cookie_jar,
            authenticated: std::sync::atomic::AtomicBool::new(false),
            domains,
        }
    }

    pub fn add_domain(&mut self, domain: String) {
        self.domains.push(domain);
    }

    /// Authenticate with Anna's Archive using the secret key.
    /// This sets the aa_account_id2 cookie needed for API access.
    async fn authenticate(&self) -> Result<(), Error> {
        let api_key = self.api_key.as_ref().ok_or(Error::MissingApiKey)?;

        // Try each domain for authentication
        for domain in &self.domains {
            let url = format!("https://{domain}/account/");

            let response = self
                .client
                .post(&url)
                .form(&[("key", api_key.as_str())])
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
                    self.authenticated
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    return Ok(());
                }
                Ok(resp) if resp.status().is_client_error() => {
                    return Err(Error::Api {
                        message: "Invalid secret key".to_string(),
                    });
                }
                _ => continue, // Try next domain
            }
        }

        Err(Error::AllDomainsFailed {
            message: "Failed to authenticate with any domain".to_string(),
        })
    }

    async fn ensure_authenticated(&self) -> Result<(), Error> {
        if !self.authenticated.load(std::sync::atomic::Ordering::SeqCst) {
            self.authenticate().await?;
        }
        Ok(())
    }

    async fn fetch_with_failover(&self, path: &str) -> Result<String, Error> {
        let mut last_error = None;

        for domain in &self.domains {
            let url = format!("https://{domain}{path}");

            match self.client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return response.text().await.map_err(Error::Network);
                    } else if response.status().is_client_error() {
                        // Client errors (4xx) won't be fixed by trying another domain
                        return Err(Error::Http {
                            status: response.status().as_u16(),
                        });
                    }
                    // Server error - try next domain
                    last_error = Some(Error::Http {
                        status: response.status().as_u16(),
                    });
                }
                Err(e) => {
                    // Connection error - try next domain
                    last_error = Some(Error::Network(e));
                }
            }
        }

        Err(last_error.unwrap_or(Error::AllDomainsFailed {
            message: "No domains available".to_string(),
        }))
    }

    pub async fn search(&self, options: SearchOptions) -> Result<SearchResponse, Error> {
        let page = options.page.unwrap_or(1);
        let query = urlencoding::encode(&options.query);
        let mut path = format!("/search?q={query}&page={page}");

        // Legacy single lang support (deprecated)
        if let Some(lang) = options.lang {
            path = format!("{}&lang={}", path, lang.as_str());
        }

        // Add format filters (ext=pdf&ext=epub or ext=anti_mobi)
        for filter in &options.ext_filters {
            path = format!("{}&ext={}", path, urlencoding::encode(filter));
        }

        // Add language filters (lang=en&lang=fr or lang=anti_es)
        for filter in &options.lang_filters {
            path = format!("{}&lang={}", path, urlencoding::encode(filter));
        }

        // Add content filters (content=book_nonfiction&content=anti__book_fiction)
        for filter in &options.content_filters {
            path = format!("{}&content={}", path, urlencoding::encode(filter));
        }

        let html = self.fetch_with_failover(&path).await?;
        let (results, has_more) = parse_search_results(&html)?;

        Ok(SearchResponse {
            results,
            page,
            has_more,
        })
    }

    /// Get detailed metadata for an item. Requires API key (secret key).
    pub async fn get_details(&self, md5: &str) -> Result<ItemDetails, Error> {
        self.ensure_authenticated().await?;

        let path = format!("/db/aarecord_elasticsearch/md5:{md5}.json");

        let mut last_error = None;

        for domain in &self.domains {
            let url = format!("https://{domain}{path}");
            match self.client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let details: DetailsDto = response.json().await.map_err(Error::Network)?;
                        return Ok(details.into());
                    } else if response.status().is_client_error() {
                        let status = response.status().as_u16();
                        if status == 403 {
                            // Re-authenticate and retry once
                            self.authenticated
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                            self.authenticate().await?;

                            // Retry request
                            if let Ok(resp) = self.client.get(&url).send().await
                                && resp.status().is_success()
                            {
                                let details: DetailsDto =
                                    response.json().await.map_err(Error::Network)?;
                                return Ok(details.into());
                            }
                        }
                        return Err(Error::Http { status });
                    }
                    last_error = Some(Error::Http {
                        status: response.status().as_u16(),
                    });
                }
                Err(e) => {
                    last_error = Some(Error::Network(e));
                }
            }
        }

        Err(last_error.unwrap_or(Error::AllDomainsFailed {
            message: "Failed to get details from any domain".to_string(),
        }))
    }

    pub async fn get_download_url(
        &self,
        md5: &str,
        path_index: Option<u32>,
        domain_index: Option<u32>,
    ) -> Result<DownloadInfo, Error> {
        let api_key = self.api_key.as_ref().ok_or(Error::MissingApiKey)?;

        let path_idx = path_index.unwrap_or(0);
        let domain_idx = domain_index.unwrap_or(0);

        // Try each domain for the fast download API
        let mut last_error = None;

        for domain in &self.domains {
            let url = format!(
                "https://{domain}/dyn/api/fast_download.json?md5={md5}&path_index={path_idx}&domain_index={domain_idx}&key={api_key}"
            );

            let response = match self.client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_error = Some(Error::Network(e));
                    continue;
                }
            };

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();

                // Check for common API errors
                if body.contains("no_membership") {
                    return Err(Error::Api {
                        message: "No active membership for this API key".to_string(),
                    });
                }
                if body.contains("invalid") {
                    return Err(Error::Api {
                        message: "Invalid API key".to_string(),
                    });
                }

                last_error = Some(Error::Http { status });
                continue;
            }

            #[derive(serde::Deserialize)]
            struct ApiResponse {
                download_url: Option<String>,
                error: Option<String>,
            }

            let api_response: ApiResponse = match response.json().await {
                Ok(r) => r,
                Err(e) => {
                    last_error = Some(Error::Network(e));
                    continue;
                }
            };

            if let Some(error) = api_response.error {
                return Err(Error::Api { message: error });
            }

            let download_url = api_response.download_url.ok_or(Error::Api {
                message: "No download URL in response".to_string(),
            })?;

            return Ok(DownloadInfo { download_url });
        }

        Err(last_error.unwrap_or(Error::AllDomainsFailed {
            message: "Failed to get download URL from any domain".to_string(),
        }))
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use crate::{AnnasArchiveClient, OpenLibraryClient, SearchOptions};

    #[tokio::test]
    async fn should_search_books() {
        dotenv::from_filename(".env.secret").ok();
        let key = env::var("ANNAS_ARCHIVE_API_KEY").unwrap();
        let client = AnnasArchiveClient::new("annas-archive.gl".to_string(), Some(key));
        let result = client
            .search(SearchOptions::new("Victor Hugo"))
            .await
            .unwrap();

        println!("{:?}", result.results);
    }

    #[tokio::test]
    async fn should_get_book_details() {
        dotenv::from_filename(".env.secret").ok();
        let key = env::var("ANNAS_ARCHIVE_API_KEY").unwrap();
        let client = AnnasArchiveClient::new("annas-archive.gl".to_string(), Some(key));
        let result = client
            .get_details("b98acc50fb0b8337683628a43aca64bd")
            .await
            .unwrap();

        println!("{:?}", result);
    }

    #[tokio::test]
    async fn should_get_book_details_with_serie() {
        dotenv::from_filename(".env.secret").ok();
        let key = env::var("ANNAS_ARCHIVE_API_KEY").unwrap();
        let client = AnnasArchiveClient::new("annas-archive.gl".to_string(), Some(key));
        let result = client
            .get_details("8efbf8e9f8b4592c7b0dbedec9c0ec05")
            .await
            .unwrap();

        println!("{:?}", result);
    }

    #[tokio::test]
    async fn should_get_serie_via_openlibrary() {
        let client = OpenLibraryClient::new();
        let serie = client.get_serie("OL82560W").await.unwrap();
        println!("{:?}", serie)
    }
}
