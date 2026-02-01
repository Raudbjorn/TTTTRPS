use leptos::prelude::*;

/// A styled card container component inspired by Shadcn-UI
#[component]
pub fn Card(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Card content
    children: Children,
) -> impl IntoView {
    // Shadcn Card: rounded-xl border bg-card text-card-foreground shadow
    let base_class = "rounded-xl border border-zinc-200 bg-white text-zinc-950 shadow dark:border-zinc-800 dark:bg-zinc-950 dark:text-zinc-50";
    let full_class = format!("{base_class} {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

/// Card header section
#[component]
pub fn CardHeader(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Header content
    children: Children,
) -> impl IntoView {
    // Shadcn Header: flex flex-col space-y-1.5 p-6
    // Note: The original had border-b and bg color. Shadcn is usually clean.
    // We will keep flex-col but allow overrides via class for existing usages that expect flex-row.
    let base_class = "flex flex-col space-y-1.5 p-6"; 
    let full_class = format!("{base_class} {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

/// Card body section with padding
#[component]
pub fn CardBody(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Body content
    children: Children,
) -> impl IntoView {
    // Shadcn Content: p-6 pt-0
    let base_class = "p-6 pt-0";
    let full_class = format!("{base_class} {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

/// Card title component
#[component]
pub fn CardTitle(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content
    children: Children,
) -> impl IntoView {
    let base_class = "font-semibold leading-none tracking-tight";
    let full_class = format!("{base_class} {class}");

    view! {
        <h3 class=full_class>
            {children()}
        </h3>
    }
}

/// Card description component
#[component]
pub fn CardDescription(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content
    children: Children,
) -> impl IntoView {
    let base_class = "text-sm text-zinc-500 dark:text-zinc-400";
    let full_class = format!("{base_class} {class}");

    view! {
        <p class=full_class>
            {children()}
        </p>
    }
}
