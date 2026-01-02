use leptos::prelude::*;

/// A styled card container component
#[component]
pub fn Card(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Card content
    children: Children,
) -> impl IntoView {
    let base_class = "bg-gray-800 border border-gray-700 rounded-lg shadow-md overflow-hidden";
    let full_class = format!("{base_class} {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

/// Card header section with distinct background
#[component]
pub fn CardHeader(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Header content
    children: Children,
) -> impl IntoView {
    let base_class =
        "px-4 py-3 bg-gray-800/50 border-b border-gray-700 flex justify-between items-center";
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
    let base_class = "p-4";
    let full_class = format!("{base_class} {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}
