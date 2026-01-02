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
    placeholder: String,
    /// Input change handler (called with the new value)
    #[prop(into, optional)]
    on_input: Option<Callback<String>>,
    /// Keydown event handler
    #[prop(into, optional)]
    on_keydown: Option<Callback<ev::KeyboardEvent>>,
    /// Whether the input is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Input type (text, password, email, etc.)
    #[prop(into, optional)]
    r#type: Option<String>,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
) -> impl IntoView {
    let input_type = r#type.unwrap_or_else(|| "text".to_string());

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
            type=input_type
            prop:value=move || value.get()
            placeholder=placeholder
            disabled=disabled
            on:input=handle_input
            on:keydown=handle_keydown
        />
    }
}
