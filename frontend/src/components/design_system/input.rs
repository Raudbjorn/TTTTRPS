use leptos::ev;
use leptos::prelude::*;

/// A styled text input component
#[component]
pub fn Input(
    /// The current value (two-way binding signal)
    #[prop(into)]
    value: RwSignal<String>,
    /// Placeholder text
    #[prop(into, optional)]
    placeholder: Signal<String>,
    /// Input change handler (called with the new value)
    #[prop(into, optional)]
    on_input: Option<Callback<String>>,
    /// Keydown event handler
    #[prop(into, optional)]
    on_keydown: Option<Callback<ev::KeyboardEvent>>,
    /// Whether the input is disabled
    #[prop(into, default = Signal::derive(|| false))]
    disabled: Signal<bool>,
    /// Input type (text, password, email, etc.)
    #[prop(into, optional)]
    r#type: Signal<String>,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// List attribute for datalist
    #[prop(into, optional)]
    list: Option<String>,
) -> impl IntoView {
    let input_type = Signal::derive(move || {
        let t = r#type.get();
        if t.is_empty() {
            "text".to_string()
        } else {
            t
        }
    });

    let base_class = "w-full p-2 rounded bg-gray-900 text-white border border-gray-700 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors placeholder-gray-500 disabled:opacity-50 disabled:cursor-not-allowed";

    let full_class = format!("{base_class} {class}");

    let handle_input = move |evt: ev::Event| {
        let new_value = event_target_value(&evt);
        value.set(new_value.clone());
        if let Some(ref callback) = on_input {
            callback.run(new_value);
        }
    };

    let handle_keydown = move |evt: ev::KeyboardEvent| {
        if let Some(ref callback) = on_keydown {
            callback.run(evt);
        }
    };

    view! {
        <input
            class=full_class
            type=move || input_type.get()
            prop:value=move || value.get()
            placeholder=move || placeholder.get()
            disabled=move || disabled.get()
            list=list
            on:input=handle_input
            on:keydown=handle_keydown
        />
    }
}
