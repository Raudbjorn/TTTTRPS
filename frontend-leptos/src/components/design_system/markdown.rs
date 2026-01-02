use leptos::prelude::*;
use pulldown_cmark::{html, Options, Parser};

/// CSS styles for markdown content
const MARKDOWN_STYLES: &str = r#"
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

/// Render markdown content to HTML using pulldown-cmark
fn render_markdown(content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// A markdown renderer component using pulldown-cmark
#[component]
pub fn Markdown(
    /// The markdown content to render
    #[prop(into)]
    content: String,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
) -> impl IntoView {
    let html_content = render_markdown(&content);
    let full_class = format!("markdown-content text-gray-200 {class}");

    view! {
        <style>{MARKDOWN_STYLES}</style>
        <div class=full_class inner_html=html_content />
    }
}
