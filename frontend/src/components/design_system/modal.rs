use leptos::ev;
use leptos::prelude::*;

/// A modal dialog component
/// Note: Due to Leptos Children semantics, the modal content is always rendered
/// but the modal container is conditionally shown/hidden via CSS.
#[component]
pub fn Modal(
    /// Whether the modal is visible
    is_open: RwSignal<bool>,
    /// Optional title for the modal header
    #[prop(into, optional)]
    title: String,
    /// Additional CSS classes for the modal content
    #[prop(into, optional)]
    class: String,
    /// Modal content
    children: Children,
) -> impl IntoView {
    let handle_backdrop_click = move |_| {
        is_open.set(false);
    };

    let handle_content_click = move |evt: ev::MouseEvent| {
        evt.stop_propagation();
    };

    let has_title = !title.is_empty();

    // Use CSS to show/hide instead of conditional rendering
    // This avoids the Children + Show issue in Leptos
    view! {
        <div
            class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 transition-opacity duration-200"
            style:display=move || if is_open.get() { "flex" } else { "none" }
            on:click=handle_backdrop_click
        >
            <div
                class=format!("bg-zinc-900 rounded-xl border border-zinc-800 shadow-2xl overflow-hidden {class}")
                on:click=handle_content_click
            >
                {if has_title {
                    Some(view! {
                        <div class="h-16 bg-gradient-to-br from-purple-900 to-zinc-900 p-4 flex items-center border-b border-zinc-800">
                            <h2 class="text-xl font-bold text-white">{title.clone()}</h2>
                        </div>
                    })
                } else {
                    None
                }}
                {children()}
            </div>
        </div>
    }
}
