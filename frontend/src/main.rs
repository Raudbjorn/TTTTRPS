#![allow(non_snake_case)]

use dioxus::prelude::*;

pub mod bindings;

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

// Global Theme Signal
pub type ThemeSignal = Signal<String>;

fn App() -> Element {
    // Initialize theme signal
    use_context_provider(|| Signal::new("fantasy".to_string()));
    let theme_sig = use_context::<ThemeSignal>();

    // Effect to update body attribute
    use_effect(move || {
        let current_theme = theme_sig.read();
        let _ = document::eval(&format!("document.body.setAttribute('data-theme', '{}')", current_theme));
    });

    rsx! {
        Router::<Route> {}
    }
}
