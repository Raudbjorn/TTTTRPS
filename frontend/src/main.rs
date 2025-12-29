#![allow(non_snake_case)]

use dioxus::prelude::*;

pub mod bindings;

pub mod bindings;
pub mod services;
pub mod components;

use components::chat::Chat;
use components::settings::Settings;
use components::library::Library;
use components::campaigns::Campaigns;
use components::session::Session;
use components::character::CharacterCreator;
use components::layout::main_shell::MainShell;
use services::layout_service::LayoutState;

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
    // Initialize services
    use_context_provider(|| Signal::new("fantasy".to_string()));
    use_context_provider(|| LayoutState::new());

    let theme_sig = use_context::<ThemeSignal>();

    // Effect to update body attribute
    use_effect(move || {
        let current_theme = theme_sig.read();
        let _ = document::eval(&format!("document.body.setAttribute('data-theme', '{}')", current_theme));
    });

    rsx! {
        // We wrap the entire app in the MainShell
        MainShell {
            sidebar: rsx! {
                div { class: "p-4 text-sm text-[var(--text-muted)]", "Sidebar Context" }
            },
            info_panel: rsx! {
                 div { class: "p-4 text-sm text-[var(--text-muted)]", "Info Panel" }
            },

            // The Router Outlet goes into the Main Content slot
            Router::<Route> {}
        }
    }
}
