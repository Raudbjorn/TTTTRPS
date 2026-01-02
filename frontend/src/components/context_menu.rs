use dioxus::prelude::*;

#[component]
pub fn ContextMenu(
    x: f64,
    y: f64,
    on_close: EventHandler<()>,
    children: Element
) -> Element {
    rsx! {
        // Backdrop
        div {
            class: "fixed inset-0 z-40",
            onclick: move |_| on_close.call(()),
        }
        // Menu
        div {
            class: "fixed z-50 bg-zinc-900 border border-zinc-700 rounded-lg shadow-xl p-1 w-48 flex flex-col gap-1 text-sm text-zinc-300 animate-in fade-in zoom-in-95 duration-100",
            style: "left: {x}px; top: {y}px;",
            {children}
        }
    }
}

#[component]
pub fn ContextMenuItem(
    icon: Option<&'static str>,
    label: String,
    onclick: EventHandler<()>,
    danger: Option<bool>
) -> Element {
    let base_class = "flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer transition-colors";
    let color_class = if danger.unwrap_or(false) {
        "text-red-400 hover:bg-red-900/20 hover:text-red-200"
    } else {
        "hover:bg-zinc-800 hover:text-white"
    };

    rsx! {
        button {
            class: "{base_class} {color_class}",
            onclick: move |_| onclick.call(()),
            if let Some(ic) = icon {
                 span { class: "opacity-70", "{ic}" }
            }
            span { "{label}" }
        }
    }
}
