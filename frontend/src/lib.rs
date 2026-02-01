#![allow(non_snake_case)]

pub mod bindings;
pub mod components;
pub mod services;

mod app;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    // Initialize console error panic hook for better error messages
    console_error_panic_hook::set_once();

    // Simple console logging for WASM
    web_sys::console::log_1(&"Starting TTRPG Assistant Frontend (Leptos)".into());

    // Remove loading spinner
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(loader) = document.get_element_by_id("app-loading") {
                loader.remove();
            }
        }
    }



    leptos::mount::mount_to_body(app::App);
}
