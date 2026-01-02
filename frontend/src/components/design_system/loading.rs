use leptos::prelude::*;

/// A loading spinner component
#[component]
pub fn LoadingSpinner(
    /// Size: "sm", "md", or "lg"
    #[prop(default = "md")]
    size: &'static str,
) -> impl IntoView {
    let size_class = match size {
        "sm" => "w-4 h-4",
        "lg" => "w-8 h-8",
        _ => "w-6 h-6",
    };

    view! {
        <div class=format!("{} animate-spin rounded-full border-2 border-gray-600 border-t-blue-500", size_class)></div>
    }
}

/// A typing indicator component (three bouncing dots)
#[component]
pub fn TypingIndicator() -> impl IntoView {
    view! {
        <div class="flex items-center gap-1">
            <div class="w-2 h-2 rounded-full bg-zinc-500 animate-bounce" style="animation-delay: 0ms"></div>
            <div class="w-2 h-2 rounded-full bg-zinc-500 animate-bounce" style="animation-delay: 150ms"></div>
            <div class="w-2 h-2 rounded-full bg-zinc-500 animate-bounce" style="animation-delay: 300ms"></div>
        </div>
    }
}
