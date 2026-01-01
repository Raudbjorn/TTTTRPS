#![allow(non_snake_case)]
use dioxus::prelude::*;
use pulldown_cmark::{Parser, Options, html};

// ============================================================================
// Button Component
// ============================================================================

#[derive(PartialEq, Clone, Copy)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
    Outline,
}

#[derive(PartialEq, Clone, Props)]
pub struct ButtonProps {
    #[props(default = ButtonVariant::Primary)]
    variant: ButtonVariant,
    #[props(default)]
    onclick: EventHandler<MouseEvent>,
    #[props(default)]
    disabled: bool,
    #[props(default)]
    loading: bool,
    #[props(default)]
    class: String,
    #[props(default)]
    title: String,
    children: Element,
}

#[component]
pub fn Button(props: ButtonProps) -> Element {
    let base_class = "px-4 py-2 rounded transition-all duration-200 flex items-center justify-center gap-2 font-medium focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-900 focus:ring-blue-500";

    let variant_class = match props.variant {
        ButtonVariant::Primary => "bg-blue-600 hover:bg-blue-500 text-white shadow-lg shadow-blue-900/50 border border-transparent",
        ButtonVariant::Secondary => "bg-gray-700 hover:bg-gray-600 text-gray-200 border border-gray-600",
        ButtonVariant::Danger => "bg-red-600 hover:bg-red-500 text-white shadow-lg shadow-red-900/50 border border-transparent",
        ButtonVariant::Ghost => "bg-transparent hover:bg-white/10 text-gray-400 hover:text-white border border-transparent",
        ButtonVariant::Outline => "bg-transparent border border-gray-500 text-gray-300 hover:border-gray-300 hover:text-white",
    };

    let state_class = if props.disabled || props.loading {
        "opacity-50 cursor-not-allowed transform-none"
    } else {
        "cursor-pointer active:scale-95"
    };

    rsx! {
        button {
            class: "{base_class} {variant_class} {state_class} {props.class}",
            onclick: move |evt| if !props.disabled && !props.loading { props.onclick.call(evt) },
            disabled: props.disabled || props.loading,
            title: "{props.title}",
            if props.loading {
                LoadingSpinner { size: "sm" }
            }
            {props.children}
        }
    }
}

// ============================================================================
// Input Component
// ============================================================================

#[derive(PartialEq, Clone, Props)]
pub struct InputProps {
    #[props(default)]
    value: String,
    #[props(default)]
    placeholder: String,
    #[props(default)]
    oninput: EventHandler<String>,
    #[props(default)]
    onkeydown: EventHandler<KeyboardEvent>,
    #[props(default)]
    disabled: bool,
    #[props(default = "text".to_string())]
    r#type: String,
    #[props(default)]
    class: String,
}

#[component]
pub fn Input(props: InputProps) -> Element {
    rsx! {
        input {
            class: "w-full p-2 rounded bg-gray-900 text-white border border-gray-700 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors placeholder-gray-500 disabled:opacity-50 disabled:cursor-not-allowed {props.class}",
            r#type: "{props.r#type}",
            value: "{props.value}",
            placeholder: "{props.placeholder}",
            disabled: props.disabled,
            oninput: move |evt| props.oninput.call(evt.value()),
            onkeydown: move |evt| props.onkeydown.call(evt)
        }
    }
}

// ============================================================================
// Card Component
// ============================================================================

#[derive(PartialEq, Clone, Props)]
pub struct CardProps {
    #[props(default)]
    class: String,
    children: Element,
}

#[component]
pub fn Card(props: CardProps) -> Element {
    rsx! {
        div {
            class: "bg-gray-800 border border-gray-700 rounded-lg shadow-md overflow-hidden {props.class}",
            {props.children}
        }
    }
}

#[component]
pub fn CardHeader(props: CardProps) -> Element {
    rsx! {
        div {
            class: "px-4 py-3 bg-gray-800/50 border-b border-gray-700 flex justify-between items-center {props.class}",
            {props.children}
        }
    }
}

#[component]
pub fn CardBody(props: CardProps) -> Element {
    rsx! {
        div {
            class: "p-4 {props.class}",
            {props.children}
        }
    }
}

// ============================================================================
// Badge Component
// ============================================================================

#[derive(PartialEq, Clone, Copy)]
pub enum BadgeVariant {
    Default,
    Success,
    Warning,
    Error,
    Info,
    Outline,
}

#[derive(PartialEq, Clone, Props)]
pub struct BadgeProps {
    #[props(default = BadgeVariant::Default)]
    variant: BadgeVariant,
    children: Element,
}

#[component]
pub fn Badge(props: BadgeProps) -> Element {
    let color_class = match props.variant {
        BadgeVariant::Default => "bg-gray-700 text-gray-300",
        BadgeVariant::Success => "bg-green-900/50 text-green-300 border border-green-800",
        BadgeVariant::Warning => "bg-yellow-900/50 text-yellow-300 border border-yellow-800",
        BadgeVariant::Error => "bg-red-900/50 text-red-300 border border-red-800",
        BadgeVariant::Info => "bg-blue-900/50 text-blue-300 border border-blue-800",
        BadgeVariant::Outline => "bg-transparent border border-gray-500 text-gray-300",
    };

    rsx! {
        span {
            class: "px-2 py-0.5 rounded text-xs font-medium inline-flex items-center gap-1 {color_class}",
            {props.children}
        }
    }
}

// ============================================================================
// Loading Spinner
// ============================================================================

#[derive(PartialEq, Clone, Props)]
pub struct SpinnerProps {
    #[props(default = "md".to_string())]
    size: String,
    #[props(default)]
    class: String,
}

#[component]
pub fn LoadingSpinner(props: SpinnerProps) -> Element {
    let size_class = match props.size.as_str() {
        "sm" => "w-4 h-4",
        "lg" => "w-8 h-8",
        _ => "w-6 h-6",
    };

    rsx! {
        div {
            class: "animate-spin rounded-full border-2 border-gray-600 border-t-blue-500 {size_class} {props.class}"
        }
    }
}

// ============================================================================
// Modal Component
// ============================================================================

#[derive(PartialEq, Clone, Props)]
pub struct ModalProps {
    is_open: bool,
    onclose: EventHandler<()>,
    title: String,
    children: Element,
}

#[component]
pub fn Modal(props: ModalProps) -> Element {
    if !props.is_open {
        return rsx! {};
    }

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fade-in",
            onclick: move |_| props.onclose.call(()),
            div {
                class: "bg-gray-800 border border-gray-600 rounded-xl shadow-2xl w-full max-w-lg transform transition-all animate-scale-in overflow-hidden",
                onclick: |evt| evt.stop_propagation(), // Prevent click from closing modal

                // Header
                div {
                    class: "flex justify-between items-center p-4 border-b border-gray-700 bg-gray-850",
                    h3 { class: "text-lg font-semibold text-white", "{props.title}" }
                    button {
                        class: "text-gray-400 hover:text-white hover:bg-gray-700 rounded-full p-1 transition-colors",
                        onclick: move |_| props.onclose.call(()),
                        svg { class: "w-5 h-5", view_box: "0 0 20 20", fill: "currentColor",
                            path { "fill-rule": "evenodd", d: "M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z", "clip-rule": "evenodd" }
                        }
                    }
                }

                // Content
                div {
                    class: "p-6",
                    {props.children}
                }
            }
        }
    }
}

// ============================================================================
// Markdown Component
// ============================================================================

#[derive(PartialEq, Clone, Props)]
pub struct MarkdownProps {
    content: String,
    #[props(default)]
    class: String,
}

#[component]
pub fn Markdown(props: MarkdownProps) -> Element {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(&props.content, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    let style = r#"
        .markdown-content h1 { font-size: 1.5em; font-weight: bold; margin-top: 1em; margin-bottom: 0.5em; color: #e5e7eb; }
        .markdown-content h2 { font-size: 1.25em; font-weight: bold; margin-top: 1em; margin-bottom: 0.5em; color: #d1d5db; }
        .markdown-content h3 { font-size: 1.1em; font-weight: bold; margin-top: 1em; margin-bottom: 0.5em; color: #d1d5db; }
        .markdown-content p { margin-bottom: 0.8em; line-height: 1.6; }
        .markdown-content ul { list-style-type: disc; padding-left: 1.5em; margin-bottom: 1em; }
        .markdown-content ol { list-style-type: decimal; padding-left: 1.5em; margin-bottom: 1em; }
        .markdown-content li { margin-bottom: 0.25em; }
        .markdown-content code { background-color: #374151; padding: 0.2em 0.4em; border-radius: 0.25em; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 0.9em; color: #e5e7eb; }
        .markdown-content pre { background-color: #1f2937; padding: 1em; overflow-x: auto; border-radius: 0.5em; margin-bottom: 1em; border: 1px solid #374151; }
        .markdown-content pre code { background-color: transparent; padding: 0; color: #e5e7eb; border-radius: 0; }
        .markdown-content blockquote { border-left: 4px solid #4b5563; padding-left: 1em; color: #9ca3af; margin-left: 0; margin-right: 0; font-style: italic; }
        .markdown-content a { color: #60a5fa; text-decoration: underline; }
        .markdown-content strong { font-weight: bold; color: #f3f4f6; }
        .markdown-content em { font-style: italic; }
        .markdown-content table { border-collapse: collapse; width: 100%; margin-bottom: 1em; }
        .markdown-content th, .markdown-content td { border: 1px solid #4b5563; padding: 0.5em; text-align: left; }
        .markdown-content th { background-color: #374151; font-weight: bold; }
        .markdown-content tr:nth-child(even) { background-color: #262f3d; }
    "#;

    rsx! {
        style { "{style}" }
        div {
            class: "markdown-content text-gray-200 {props.class}",
            dangerous_inner_html: "{html_output}"
        }
    }
}

// ============================================================================
// Select Component
// ============================================================================

#[derive(PartialEq, Clone, Props)]
pub struct SelectProps {
    #[props(default)]
    value: String,
    #[props(default)]
    onchange: EventHandler<String>,
    #[props(default)]
    disabled: bool,
    #[props(default)]
    class: String,
    children: Element,
}

#[component]
pub fn Select(props: SelectProps) -> Element {
    rsx! {
        div {
            class: "relative {props.class}",
            select {
                class: "appearance-none w-full p-2 pr-8 rounded bg-gray-900 text-white border border-gray-700 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                value: "{props.value}",
                disabled: props.disabled,
                onchange: move |evt| props.onchange.call(evt.value()),
                {props.children}
            }
            // Chevron Icon
            div {
                class: "absolute right-2 top-1/2 transform -translate-y-1/2 pointer-events-none text-gray-500",
                svg { class: "w-4 h-4", view_box: "0 0 20 20", fill: "currentColor",
                    path { "fill-rule": "evenodd", d: "M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z", "clip-rule": "evenodd" }
                }
            }
        }
    }
}

// ============================================================================
// Typing Indicator
// ============================================================================

#[component]
pub fn TypingIndicator() -> Element {
    rsx! {
        div {
            class: "flex space-x-1 items-center p-2 h-8",
            div { class: "w-2 h-2 bg-gray-500 rounded-full animate-bounce", style: "animation-delay: -0.3s" }
            div { class: "w-2 h-2 bg-gray-500 rounded-full animate-bounce", style: "animation-delay: -0.15s" }
            div { class: "w-2 h-2 bg-gray-500 rounded-full animate-bounce" }
        }
    }
}
