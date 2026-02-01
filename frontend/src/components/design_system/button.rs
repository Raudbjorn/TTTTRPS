use leptos::prelude::*;
use leptos::ev;
use super::loading::LoadingSpinner;

/// Button variant styles
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Destructive,
    Ghost,
    Outline,
    Link,
}

/// Button size variants
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    #[default]
    Default,
    Sm,
    Lg,
    Icon,
}

impl ButtonVariant {
    fn class(&self) -> &'static str {
        match self {
            // Shadcn Primary: bg-primary text-primary-foreground shadow hover:bg-primary/90
            ButtonVariant::Primary => {
                "bg-zinc-900 text-zinc-50 shadow hover:bg-zinc-900/90 dark:bg-zinc-50 dark:text-zinc-900 dark:hover:bg-zinc-50/90"
            }
            // Shadcn Secondary: bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80
            ButtonVariant::Secondary => {
                "bg-zinc-100 text-zinc-900 shadow-sm hover:bg-zinc-100/80 dark:bg-zinc-800 dark:text-zinc-50 dark:hover:bg-zinc-800/80"
            }
            // Shadcn Destructive: bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90
            ButtonVariant::Destructive => {
                "bg-red-500 text-zinc-50 shadow-sm hover:bg-red-500/90 dark:bg-red-900 dark:text-zinc-50 dark:hover:bg-red-900/90"
            }
            // Shadcn Ghost: hover:bg-accent hover:text-accent-foreground
            ButtonVariant::Ghost => {
                "hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-zinc-800 dark:hover:text-zinc-50"
            }
            // Shadcn Outline: border border-input bg-background shadow-sm hover:bg-accent hover:text-accent-foreground
            ButtonVariant::Outline => {
                "border border-zinc-200 bg-white shadow-sm hover:bg-zinc-100 hover:text-zinc-900 dark:border-zinc-800 dark:bg-zinc-950 dark:hover:bg-zinc-800 dark:hover:text-zinc-50"
            }
            // Shadcn Link: text-primary underline-offset-4 hover:underline
            ButtonVariant::Link => {
                "text-zinc-900 underline-offset-4 hover:underline dark:text-zinc-50"
            }
        }
    }
}

impl ButtonSize {
    fn class(&self) -> &'static str {
        match self {
            ButtonSize::Default => "h-9 px-4 py-2",
            ButtonSize::Sm => "h-8 rounded-md px-3 text-xs",
            ButtonSize::Lg => "h-10 rounded-md px-8",
            ButtonSize::Icon => "h-9 w-9",
        }
    }
}

/// A styled button component inspired by Shadcn-UI
#[component]
pub fn Button<F>(
    /// The visual variant of the button
    #[prop(default = ButtonVariant::Primary)]
    variant: ButtonVariant,
    /// The size of the button
    #[prop(default = ButtonSize::Default)]
    size: ButtonSize,
    /// Click handler - accepts any closure taking MouseEvent
    #[prop(optional)]
    on_click: Option<F>,
    /// Whether the button is disabled
    #[prop(into, default = false.into())]
    disabled: Signal<bool>,
    /// Whether to show a loading spinner
    #[prop(into, default = false.into())]
    loading: Signal<bool>,
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
    // Shadcn Base Styles
    let base_class = "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-zinc-950 disabled:pointer-events-none disabled:opacity-50 dark:focus-visible:ring-zinc-300";
    
    let variant_class = variant.class();
    let size_class = size.class();

    let is_disabled = move || disabled.get() || loading.get();

    let full_class = move || format!(
        "{} {} {} {}", 
        base_class, 
        variant_class, 
        size_class,
        class
    );

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
