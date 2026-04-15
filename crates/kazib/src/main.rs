use dioxus::prelude::*;
use views::{Book, History, Search, Settings};

mod model;

use crate::model::AppSettings;
#[cfg(feature = "server")]
use {
    annas_archive_api::AnnasArchiveClient,
    db::TemplateError,
    dioxus::{CapturedError, fullstack::Lazy},
    redb::Database,
    std::{
        path::Path,
        sync::{Arc, RwLock},
    },
    tokio::{fs::File, io::AsyncWriteExt},
};

#[cfg(feature = "server")]
mod db;

#[cfg(feature = "server")]
mod path_template;

mod views;

#[cfg(feature = "server")]
static DATABASE: Lazy<Arc<Database>> = Lazy::new(async move || {
    let db_path = std::path::Path::new("data/kazib.db");
    let db = db::init_db(db_path).map_err(|e| CapturedError::from_display(e))?;
    Ok::<Arc<Database>, CapturedError>(Arc::new(db))
});

#[cfg(feature = "server")]
static CLIENT: Lazy<Arc<RwLock<AnnasArchiveClient>>> = Lazy::new(async move || {
    let db = DATABASE.clone();
    let settings = AppSettings::get(&db).map_err(CapturedError::from_display)?;

    Ok::<Arc<RwLock<AnnasArchiveClient>>, CapturedError>(Arc::new(RwLock::new(
        AnnasArchiveClient::new("annas-archive.gl".to_string(), settings.api_key),
    )))
});

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Search{},
    #[route("/book/:md5")]
    Book{ md5: String },
    #[route("/history")]
    History{},
    #[route("/admin")]
    Settings{},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn Navbar() -> Element {
    rsx! {
        div { id: "navbar",
            Link { to: Route::Search {}, "Home" }
            Link { to: Route::History {}, "History" }
            Link { to: Route::Settings {}, "Settings" }
        }

        Outlet::<Route> {}
    }
}
