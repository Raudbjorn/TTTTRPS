pub mod chat_message;
// Placeholder for Chat component if it was missing or moved
use dioxus::prelude::*;

#[component]
pub fn Chat() -> Element {
    rsx! {
        div { class: "p-4", "Chat View Placeholder" }
    }
}
