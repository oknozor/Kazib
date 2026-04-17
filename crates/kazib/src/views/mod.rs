use dioxus::prelude::*;

mod admin;
mod book;
mod download;
mod history;
mod search;

pub use admin::{Settings, get_settings};
pub use book::Book;
pub use history::{History, check_book_in_library};
pub use search::Search;

#[cfg(feature = "server")]
pub(crate) fn extract_username(
    headers: &dioxus::fullstack::http::HeaderMap,
    header_name: &str,
) -> Option<String> {
    headers
        .get(header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

#[get("/users/me", headers: dioxus_fullstack::HeaderMap)]
pub async fn get_current_user() -> Result<Option<String>> {
    use crate::AppSettings;
    use crate::server::DATABASE;
    use dioxus::CapturedError;

    let db = DATABASE.clone();
    let settings = AppSettings::get(&db).map_err(CapturedError::from_display)?;

    Ok(extract_username(&headers, &settings.auth_header_name))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::extract_username;
    use dioxus::fullstack::http::HeaderMap;

    #[test]
    fn extract_username_present() {
        let mut headers = HeaderMap::new();
        headers.insert("x-authentik-username", "alice".parse().unwrap());
        assert_eq!(
            extract_username(&headers, "x-authentik-username"),
            Some("alice".to_string())
        );
    }

    #[test]
    fn extract_username_missing_header() {
        let headers = HeaderMap::new();
        assert_eq!(extract_username(&headers, "x-authentik-username"), None);
    }

    #[test]
    fn extract_username_different_header_name() {
        let mut headers = HeaderMap::new();
        headers.insert("remote-user", "bob".parse().unwrap());
        assert_eq!(
            extract_username(&headers, "remote-user"),
            Some("bob".to_string())
        );
        assert_eq!(extract_username(&headers, "x-authentik-username"), None);
    }

    #[test]
    fn extract_username_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-User", "charlie".parse().unwrap());
        // HTTP headers are case-insensitive; HeaderMap normalizes to lowercase
        assert_eq!(
            extract_username(&headers, "x-forwarded-user"),
            Some("charlie".to_string())
        );
    }

    #[test]
    fn extract_username_non_utf8() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-authentik-username",
            dioxus::fullstack::http::HeaderValue::from_bytes(&[0x80, 0x81]).unwrap(),
        );
        assert_eq!(extract_username(&headers, "x-authentik-username"), None);
    }
}
