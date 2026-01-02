use leptos::prelude::*;

/// Badge variant styles
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum BadgeVariant {
    #[default]
    Default,
    Success,
    Warning,
    Danger,
    Info,
}

impl BadgeVariant {
    fn class(&self) -> &'static str {
        match self {
            BadgeVariant::Default => "bg-zinc-700 text-zinc-200",
            BadgeVariant::Success => "bg-green-900/50 text-green-400 border-green-500/30",
            BadgeVariant::Warning => "bg-yellow-900/50 text-yellow-400 border-yellow-500/30",
            BadgeVariant::Danger => "bg-red-900/50 text-red-400 border-red-500/30",
            BadgeVariant::Info => "bg-blue-900/50 text-blue-400 border-blue-500/30",
        }
    }
}

/// A styled badge/tag component
#[component]
pub fn Badge(
    /// The visual variant of the badge
    #[prop(default = BadgeVariant::Default)]
    variant: BadgeVariant,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Badge content
    children: Children,
) -> impl IntoView {
    let base_class = "px-2 py-0.5 text-xs font-medium rounded-full border";
    let variant_class = variant.class();
    let full_class = format!("{base_class} {variant_class} {class}");

    view! {
        <span class=full_class>
            {children()}
        </span>
    }
}
