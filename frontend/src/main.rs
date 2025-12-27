#![allow(non_snake_case)]

use dioxus::prelude::*;

mod components {
    pub mod chat;
    pub mod settings;
    pub mod library;
}
use components::chat::Chat;
use components::settings::Settings;
use components::library::Library;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Chat {},
    #[route("/settings")]
    Settings {},
    #[route("/library")]
    Library {},
}

fn main() {
    tracing_wasm::set_as_global_default();
    tracing::info!("Starting TTRPG Assistant Frontend");
    launch(App);
}

fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}
