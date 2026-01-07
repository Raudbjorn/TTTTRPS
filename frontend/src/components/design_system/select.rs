use leptos::ev;
use leptos::prelude::*;

/// Standard select styling - use this for consistent dropdowns across the app
pub const SELECT_CLASS: &str = "w-full p-3 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] transition-colors";

/// Standard option styling
pub const OPTION_CLASS: &str = "bg-[var(--bg-elevated)] text-[var(--text-primary)]";

/// A styled select dropdown component that directly sets an RwSignal
/// Use this when you want automatic signal updates (like Slider)
#[component]
pub fn SelectRw(
    /// Current selected value (reactive signal - directly set on change)
    value: RwSignal<String>,
    /// Optional change handler for additional processing
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
    let full_class = format!("{SELECT_CLASS} {class}");

    let handle_change = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&evt);
        let new_value = target.value();
        // Directly set the signal (like Slider does)
        value.set(new_value.clone());
        // Also call callback for any additional handling
        if let Some(ref callback) = on_change {
            callback.run(new_value);
        }
    };

    view! {
        <select
            class=full_class
            style="color-scheme: dark;"
            disabled=disabled
            on:change=handle_change
            prop:value=move || value.get()
        >
            {children()}
        </select>
    }
}

/// A styled select dropdown component (legacy - uses callback for updates)
/// For new code, prefer SelectRw which directly sets the signal
#[component]
pub fn Select(
    /// Current selected value (reactive - accepts signals or static strings)
    #[prop(into)]
    value: MaybeSignal<String>,
    /// Change handler - required to update the value
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
    let full_class = format!("{SELECT_CLASS} {class}");

    let handle_change = move |evt: ev::Event| {
        if let Some(ref callback) = on_change {
            let target = event_target::<web_sys::HtmlSelectElement>(&evt);
            callback.run(target.value());
        }
    };

    view! {
        <select
            class=full_class
            style="color-scheme: dark;"
            disabled=disabled
            on:change=handle_change
            prop:value=move || value.get()
        >
            {children()}
        </select>
    }
}

/// A styled option for use within Select
#[component]
pub fn SelectOption(
    /// The option value
    #[prop(into)]
    value: String,
    /// Display text (if different from value)
    #[prop(into, optional)]
    label: Option<String>,
) -> impl IntoView {
    let display = label.unwrap_or_else(|| value.clone());
    view! {
        <option value=value class=OPTION_CLASS>{display}</option>
    }
}
