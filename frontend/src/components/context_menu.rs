//! Context Menu component for Leptos
//! Right-click menu with items

use leptos::prelude::*;

#[component]
pub fn ContextMenu(
    x: f64,
    y: f64,
    on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    view! {
        // Backdrop
        <div
            class="fixed inset-0 z-40"
            on:click=move |_| on_close.run(())
        />
        // Menu
        <div
            class="fixed z-50 bg-zinc-900 border border-zinc-700 rounded-lg shadow-xl p-1 w-48 flex flex-col gap-1 text-sm text-zinc-300 animate-in fade-in zoom-in-95 duration-100"
            style=move || format!("left: {}px; top: {}px;", x, y)
        >
            {children()}
        </div>
    }
}

#[component]
pub fn ContextMenuItem(
    #[prop(optional)]
    icon: Option<&'static str>,
    label: String,
    on_click: Callback<()>,
    #[prop(optional)]
    danger: Option<bool>,
) -> impl IntoView {
    let base_class = "flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer transition-colors";
    let color_class = if danger.unwrap_or(false) {
        "text-red-400 hover:bg-red-900/20 hover:text-red-200"
    } else {
        "hover:bg-zinc-800 hover:text-white"
    };

    view! {
        <button
            class=format!("{} {}", base_class, color_class)
            on:click=move |_| on_click.run(())
        >
            {icon.map(|ic| view! { <span class="opacity-70">{ic}</span> })}
            <span>{label}</span>
        </button>
    }
}
