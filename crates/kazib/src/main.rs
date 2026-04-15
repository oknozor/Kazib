use dioxus::prelude::*;
use views::{Book, History, Search, Settings};

mod model;
mod views;

#[cfg(feature = "server")]
mod server;

use crate::model::AppSettings;

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
