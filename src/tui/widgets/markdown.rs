//! Markdown → ratatui Lines renderer.
//!
//! Converts markdown text to `Vec<Line<'static>>` for display in ratatui widgets.
//! Reuses syntect resources from `core::logging` to avoid double-loading.

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::tui::theme;
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;

use crate::core::logging::{get_syntax_set, get_theme_set};

/// Convert markdown text to ratatui Lines with syntax highlighting.
pub fn markdown_to_lines(md: &str) -> Vec<Line<'static>> {
    let parser = Parser::new(md);
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Current line being built
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    // Style stack for nested formatting
    let mut style_stack: Vec<Style> = vec![Style::default()];

    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buffer = String::new();
    let mut list_depth: usize = 0;
    let mut in_heading = false;

    for event in parser {
        match event {
            // ── Headings ─────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                flush_line(&mut current_spans, &mut lines);
                let style = match level {
                    pulldown_cmark::HeadingLevel::H1 => {
                        Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
                    }
                    pulldown_cmark::HeadingLevel::H2 => {
                        Style::default().fg(theme::PRIMARY_LIGHT).add_modifier(Modifier::BOLD)
                    }
                    pulldown_cmark::HeadingLevel::H3 => Style::default().fg(theme::SUCCESS),
                    _ => Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
                };
                style_stack.push(style);
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                flush_line(&mut current_spans, &mut lines);
                in_heading = false;
            }

            // ── Bold / Italic ────────────────────────────────────
            Event::Start(Tag::Strong) => {
                let base = current_style(&style_stack);
                style_stack.push(base.add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                let base = current_style(&style_stack);
                style_stack.push(base.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }

            // ── Inline code ──────────────────────────────────────
            Event::Code(code) => {
                current_spans.push(Span::styled(
                    format!(" {} ", code),
                    Style::default().fg(theme::TEXT).bg(theme::BG_SURFACE),
                ));
            }

            // ── Fenced code blocks ───────────────────────────────
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                flush_line(&mut current_spans, &mut lines);
                in_code_block = true;
                code_lang = lang.to_string();
                code_buffer.clear();
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                flush_line(&mut current_spans, &mut lines);
                in_code_block = true;
                code_lang.clear();
                code_buffer.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                if !code_buffer.is_empty() {
                    render_code_block(&code_buffer, &code_lang, &mut lines);
                }
                in_code_block = false;
            }

            // ── Lists ────────────────────────────────────────────
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                if list_depth == 0 {
                    // Blank line after top-level list
                    lines.push(Line::raw(""));
                }
            }
            Event::Start(Tag::Item) => {
                flush_line(&mut current_spans, &mut lines);
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                current_spans.push(Span::styled(
                    format!("{indent}• "),
                    Style::default().fg(theme::PRIMARY_LIGHT),
                ));
            }
            Event::End(TagEnd::Item) => {
                flush_line(&mut current_spans, &mut lines);
            }

            // ── Links ────────────────────────────────────────────
            Event::Start(Tag::Link { .. }) => {
                let style = Style::default()
                    .fg(theme::INFO)
                    .add_modifier(Modifier::UNDERLINED);
                style_stack.push(style);
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
            }

            // ── Paragraphs ───────────────────────────────────────
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut current_spans, &mut lines);
                if !in_heading {
                    lines.push(Line::raw(""));
                }
            }

            // ── Text content ─────────────────────────────────────
            Event::Text(text) => {
                if in_code_block {
                    code_buffer.push_str(&text);
                } else {
                    let style = current_style(&style_stack);
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }

            // ── Breaks ───────────────────────────────────────────
            Event::SoftBreak => {
                if !in_code_block {
                    current_spans.push(Span::raw(" "));
                }
            }
            Event::HardBreak => {
                flush_line(&mut current_spans, &mut lines);
            }

            // ── Horizontal rule ──────────────────────────────────
            Event::Rule => {
                flush_line(&mut current_spans, &mut lines);
                lines.push(Line::styled(
                    "─".repeat(40),
                    Style::default().fg(theme::TEXT_DIM),
                ));
                lines.push(Line::raw(""));
            }

            // ── Block quote ──────────────────────────────────────
            Event::Start(Tag::BlockQuote) => {
                flush_line(&mut current_spans, &mut lines);
                let base = current_style(&style_stack);
                style_stack.push(base.fg(theme::TEXT_MUTED).add_modifier(Modifier::ITALIC));
                current_spans.push(Span::styled("│ ", Style::default().fg(theme::TEXT_DIM)));
            }
            Event::End(TagEnd::BlockQuote) => {
                flush_line(&mut current_spans, &mut lines);
                style_stack.pop();
            }

            _ => {}
        }
    }

    // Flush any remaining spans
    flush_line(&mut current_spans, &mut lines);

    // Trim trailing empty lines
    while lines.last().is_some_and(|l| l.spans.is_empty() || l.to_string().is_empty()) {
        lines.pop();
    }

    lines
}

fn current_style(stack: &[Style]) -> Style {
    stack.last().copied().unwrap_or_default()
}

fn flush_line(spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
    }
}

/// Render a code block with syntect highlighting.
fn render_code_block(code: &str, lang: &str, lines: &mut Vec<Line<'static>>) {
    let ss = get_syntax_set();
    let ts = get_theme_set();

    let syntax = if lang.is_empty() {
        ss.find_syntax_plain_text()
    } else {
        ss.find_syntax_by_token(lang)
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    };

    let theme = &ts.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    for line_str in LinesWithEndings::from(code) {
        match highlighter.highlight_line(line_str, ss) {
            Ok(ranges) => {
                let spans: Vec<Span<'static>> = ranges
                    .into_iter()
                    .map(|(style, text)| {
                        let fg = style.foreground;
                        Span::styled(
                            text.to_string(),
                            Style::default()
                                .fg(Color::Rgb(fg.r, fg.g, fg.b))
                                .bg(Color::Rgb(43, 48, 59)),
                        )
                    })
                    .collect();
                lines.push(Line::from(spans));
            }
            Err(_) => {
                lines.push(Line::styled(
                    line_str.to_string(),
                    Style::default().fg(theme::TEXT).bg(Color::Rgb(43, 48, 59)),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let lines = markdown_to_lines("Hello world");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].to_string().contains("Hello world"));
    }

    #[test]
    fn test_bold_text() {
        let lines = markdown_to_lines("**bold text**");
        assert!(!lines.is_empty());
        let line = &lines[0];
        // Should have bold modifier
        assert!(line.spans.iter().any(|s| s
            .style
            .add_modifier
            .contains(Modifier::BOLD)));
    }

    #[test]
    fn test_italic_text() {
        let lines = markdown_to_lines("*italic text*");
        assert!(!lines.is_empty());
        let line = &lines[0];
        assert!(line.spans.iter().any(|s| s
            .style
            .add_modifier
            .contains(Modifier::ITALIC)));
    }

    #[test]
    fn test_headings() {
        let lines = markdown_to_lines("# Heading 1\n## Heading 2\n### Heading 3");
        assert!(lines.len() >= 3);
        // H1 should be accent + bold
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.fg == Some(theme::ACCENT)));
        // H2 should be primary_light + bold
        assert!(lines[1]
            .spans
            .iter()
            .any(|s| s.style.fg == Some(theme::PRIMARY_LIGHT)));
        // H3 should be success
        assert!(lines[2]
            .spans
            .iter()
            .any(|s| s.style.fg == Some(theme::SUCCESS)));
    }

    #[test]
    fn test_inline_code() {
        let lines = markdown_to_lines("Use `cargo test` to run");
        assert!(!lines.is_empty());
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.bg == Some(theme::BG_SURFACE)));
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let lines = markdown_to_lines(md);
        assert!(!lines.is_empty());
        // Code block lines should have Rgb background
        assert!(lines.iter().any(|l| l
            .spans
            .iter()
            .any(|s| matches!(s.style.bg, Some(Color::Rgb(43, 48, 59))))));
    }

    #[test]
    fn test_list() {
        let md = "- item one\n- item two\n- item three";
        let lines = markdown_to_lines(md);
        assert!(lines.len() >= 3);
        let text: String = lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("•"));
    }

    #[test]
    fn test_nested_formatting() {
        let lines = markdown_to_lines("**bold *and italic***");
        assert!(!lines.is_empty());
        // Should have both bold and italic spans
        let has_bold = lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD));
        assert!(has_bold);
    }

    #[test]
    fn test_empty_input() {
        let lines = markdown_to_lines("");
        assert!(lines.is_empty());
    }
}
