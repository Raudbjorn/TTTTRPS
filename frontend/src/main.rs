#![allow(non_snake_case)]

use dioxus::prelude::*;

pub mod bindings;
pub mod theme;

mod components {
    pub mod chat;
    pub mod settings;
    pub mod library;
    pub mod campaigns;
    pub mod session;
    pub mod character;
    pub mod design_system;
    pub mod campaign_details;
}
use components::chat::Chat;
use components::settings::Settings;
use components::library::Library;
use components::campaigns::Campaigns;
use components::session::Session;
use components::character::CharacterCreator;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Chat {},
    #[route("/settings")]
    Settings {},
    #[route("/library")]
    Library {},
    #[route("/campaigns")]
    Campaigns {},
    #[route("/session/:campaign_id")]
    Session { campaign_id: String },
    #[route("/character")]
    CharacterCreator {},
}

fn main() {
    tracing_wasm::set_as_global_default();
    tracing::info!("Starting TTRPG Assistant Frontend");
    launch(App);
}

fn App() -> Element {
    // Theme is now applied via CSS classes on individual components
    // (see session.rs for dynamic theme class selection based on campaign system)
    rsx! {
        Router::<Route> {}
    }
}
