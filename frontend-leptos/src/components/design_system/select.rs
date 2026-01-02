use leptos::ev;
use leptos::prelude::*;

/// A styled select dropdown component
#[component]
pub fn Select(
    /// Current selected value
    #[prop(into)]
    value: String,
    /// Change handler
    #[prop(into, optional)]
    on_change: Option<Callback<String>>,
    /// Whether the select is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Select options
    children: Children,
) -> impl IntoView {
    let base_class = "w-full bg-zinc-800 border border-zinc-700 rounded p-3 text-white focus:outline-none focus:ring-2 focus:ring-purple-500/50 focus:border-purple-500";
    let full_class = format!("{base_class} {class}");

    let handle_change = move |evt: ev::Event| {
        if let Some(ref callback) = on_change {
            let target = event_target::<web_sys::HtmlSelectElement>(&evt);
            callback.run(target.value());
        }
    };

    view! {
        <select
            class=full_class
            disabled=disabled
            on:change=handle_change
            prop:value=value
        >
            {children()}
        </select>
    }
}
