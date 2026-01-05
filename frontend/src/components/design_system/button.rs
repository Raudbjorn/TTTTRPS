use leptos::prelude::*;
use leptos::ev;
use super::loading::LoadingSpinner;

/// Button variant styles
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
    Ghost,
    Outline,
}

impl ButtonVariant {
    fn class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => {
                "bg-blue-600 hover:bg-blue-500 text-white shadow-lg shadow-blue-900/50 border border-transparent"
            }
            ButtonVariant::Secondary => {
                "bg-gray-700 hover:bg-gray-600 text-gray-200 border border-gray-600"
            }
            ButtonVariant::Danger => {
                "bg-red-600 hover:bg-red-500 text-white shadow-lg shadow-red-900/50 border border-transparent"
            }
            ButtonVariant::Ghost => {
                "bg-transparent hover:bg-white/10 text-gray-400 hover:text-white border border-transparent"
            }
            ButtonVariant::Outline => {
                "bg-transparent border border-gray-500 text-gray-300 hover:border-gray-300 hover:text-white"
            }
        }
    }
}

/// A styled button component with multiple variants
#[component]

pub fn Button<F>(
    /// The visual variant of the button
    #[prop(default = ButtonVariant::Primary)]
    variant: ButtonVariant,
    /// Click handler - accepts any closure taking MouseEvent
    #[prop(optional)]
    on_click: Option<F>,
    /// Whether the button is disabled
    #[prop(into, default = false.into())]
    disabled: MaybeSignal<bool>,
    /// Whether to show a loading spinner
    #[prop(into, default = false.into())]
    loading: MaybeSignal<bool>,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Title/tooltip text
    #[prop(into, optional)]
    title: String,
    /// Button content
    children: Children,
) -> impl IntoView
where
    F: Fn(ev::MouseEvent) + 'static,
{
    let base_class = "px-4 py-2 rounded transition-all duration-200 flex items-center justify-center gap-2 font-medium focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-900 focus:ring-blue-500";
    let variant_class = variant.class();

    let is_disabled = move || disabled.get() || loading.get();

    let state_class = move || {
        if is_disabled() {
            "opacity-50 cursor-not-allowed transform-none"
        } else {
            "cursor-pointer active:scale-95"
        }
    };

    let full_class = move || format!("{base_class} {variant_class} {} {class}", state_class());

    let handle_click = move |evt: ev::MouseEvent| {
        if !is_disabled() {
            if let Some(ref callback) = on_click {
                callback(evt);
            }
        }
    };

    view! {
        <button
            class=full_class
            on:click=handle_click
            disabled=is_disabled
            title=title
        >
            {move || {
                if loading.get() {
                    Some(view! { <LoadingSpinner size="sm" /> })
                } else {
                    None
                }
            }}
            {children()}
        </button>
    }
}
