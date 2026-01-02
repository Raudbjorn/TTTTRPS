use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::layout::main_shell::MainShell;
use crate::components::command_palette::CommandPalette;
use crate::components::chat::Chat;
use crate::components::settings::Settings;
use crate::components::library::Library;
use crate::components::campaigns::Campaigns;
use crate::components::session::Session;
use crate::components::character::CharacterCreator;
use crate::services::layout_service::provide_layout_state;
use crate::services::theme_service::{ThemeState, provide_theme_state};

#[component]
pub fn App() -> impl IntoView {
    // Provide global services
    provide_theme_state();
    provide_layout_state();

    let theme_state = expect_context::<ThemeState>();

    // Effect to update body styles when theme changes
    Effect::new(move |_| {
        let css = theme_state.get_css();
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                // Create or update the theme style element
                let style_id = "dynamic-theme-styles";
                let style_el = document.get_element_by_id(style_id).unwrap_or_else(|| {
                    let el = document.create_element("style").unwrap();
                    el.set_id(style_id);
                    if let Some(head) = document.head() {
                        let _ = head.append_child(&el);
                    }
                    el.into()
                });
                style_el.set_text_content(Some(&css));

                // Also set the preset name as data attribute if available
                if let Some(preset) = theme_state.current_preset.get() {
                    if let Some(body) = document.body() {
                        let _ = body.set_attribute("data-theme", &preset);
                    }
                }
            }
        }
    });

    view! {
        <Router>
            // Global Command Palette (Ctrl+K)
            <CommandPalette />

            <MainShell
                sidebar=|| view! {
                    <div class="p-4 text-sm text-[var(--text-muted)]">"Sidebar Context"</div>
                }.into_any()
                info_panel=|| view! {
                    <div class="p-4 text-sm text-[var(--text-muted)]">"Info Panel"</div>
                }.into_any()
            >
                <Routes fallback=|| view! { <div>"404 - Page Not Found"</div> }>
                    <Route path=path!("/") view=Chat />
                    <Route path=path!("/settings") view=Settings />
                    <Route path=path!("/library") view=Library />
                    <Route path=path!("/campaigns") view=Campaigns />
                    <Route path=path!("/session/:campaign_id") view=Session />
                    <Route path=path!("/character") view=CharacterCreator />
                </Routes>
            </MainShell>
        </Router>
    }
}
