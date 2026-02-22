//! Markdown Page Parser
//!
//! Detects page boundaries in Markdown documents and splits content into pages.
//! Supports explicit `*Page N*` markers and fallback size-based splitting.

use once_cell::sync::Lazy;
use regex::Regex;

/// Matches *Page N* or *page N* patterns on their own line
/// Examples: *Page 2*, *page 15*, *Page 123*
static PAGE_MARKER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?mi)^\s*\*[Pp]age\s+(\d+)\*\s*$").expect("Invalid page marker regex")
});

/// Default characters per synthetic page when no markers are found
pub const DEFAULT_CHARS_PER_PAGE: usize = 3000;

/// Default minimum content length to be considered a valid page.
///
/// Pages shorter than this threshold are silently dropped to filter out
/// noise like page numbers or headers extracted in isolation. However,
/// this may also drop legitimate short pages such as title pages,
/// dedication pages, or section dividers. Increase this value to be
/// more aggressive about filtering noise, or decrease it (even to 0)
/// to preserve all pages including very short ones.
pub const DEFAULT_MIN_PAGE_CONTENT_LENGTH: usize = 50;

/// Markdown page parser for detecting and splitting page boundaries
pub struct MarkdownPageParser;

impl MarkdownPageParser {
    /// Check if the content contains page markers
    pub fn has_page_markers(content: &str) -> bool {
        PAGE_MARKER.is_match(content)
    }

    /// Split Markdown content by explicit `*Page N*` markers.
    ///
    /// Returns a vector of (page_number, content) tuples.
    /// Content before the first marker is treated as page 1.
    ///
    /// # Arguments
    /// * `content` - The markdown content to split
    /// * `min_page_length` - Minimum content length for a page to be included.
    ///   If `None`, uses [`DEFAULT_MIN_PAGE_CONTENT_LENGTH`].
    pub fn split_by_page_markers(
        content: &str,
        min_page_length: Option<usize>,
    ) -> Vec<(usize, String)> {
        let min_len = min_page_length.unwrap_or(DEFAULT_MIN_PAGE_CONTENT_LENGTH);
        let mut pages = Vec::new();
        let mut current_page_num: usize = 1;
        let mut current_content = String::new();
        let mut found_first_marker = false;

        for line in content.lines() {
            if let Some(caps) = PAGE_MARKER.captures(line) {
                // Save previous page content if non-empty
                let trimmed = current_content.trim();
                if !trimmed.is_empty() && trimmed.len() >= min_len {
                    pages.push((current_page_num, current_content.trim().to_string()));
                }

                // Start new page
                if let Ok(num) = caps.get(1).unwrap().as_str().parse::<usize>() {
                    current_page_num = num;
                    found_first_marker = true;
                }
                current_content = String::new();
            } else {
                // Accumulate content
                if !current_content.is_empty() {
                    current_content.push('\n');
                }
                current_content.push_str(line);
            }
        }

        // Don't forget the last page
        let trimmed = current_content.trim();
        if !trimmed.is_empty() && trimmed.len() >= min_len {
            pages.push((current_page_num, current_content.trim().to_string()));
        }

        // If no markers were found but there's content, return as single page
        if pages.is_empty() && !content.trim().is_empty() {
            pages.push((1, content.trim().to_string()));
        }

        // If we found markers but first content was before any marker, it's page 1
        if !found_first_marker && !pages.is_empty() {
            // Content is already correctly assigned to page 1
        }

        pages
    }

    /// Split content by approximate character count (fallback when no markers).
    ///
    /// Tries to split at paragraph boundaries near the target size.
    /// Returns a vector of (page_number, content) tuples starting from page 1.
    ///
    /// # Arguments
    /// * `content` - The markdown content to split
    /// * `chars_per_page` - Target characters per page (uses [`DEFAULT_CHARS_PER_PAGE`] if 0)
    /// * `min_page_length` - Minimum content length for a page to be included.
    ///   If `None`, uses [`DEFAULT_MIN_PAGE_CONTENT_LENGTH`].
    pub fn split_by_size(
        content: &str,
        chars_per_page: usize,
        min_page_length: Option<usize>,
    ) -> Vec<(usize, String)> {
        let chars_per_page = if chars_per_page == 0 {
            DEFAULT_CHARS_PER_PAGE
        } else {
            chars_per_page
        };
        let min_len = min_page_length.unwrap_or(DEFAULT_MIN_PAGE_CONTENT_LENGTH);

        let mut pages = Vec::new();
        let mut current_content = String::new();
        let mut page_num = 1usize;

        for para in content.split("\n\n") {
            let para = para.trim();
            if para.is_empty() {
                continue;
            }

            // If adding this paragraph would exceed the limit, finalize current page
            if !current_content.is_empty()
                && current_content.len() + para.len() + 2 > chars_per_page
            {
                if current_content.len() >= min_len {
                    pages.push((page_num, current_content.trim().to_string()));
                    page_num += 1;
                }
                current_content = String::new();
            }

            // Add paragraph to current page
            if !current_content.is_empty() {
                current_content.push_str("\n\n");
            }
            current_content.push_str(para);
        }

        // Don't forget the last page
        if !current_content.is_empty() && current_content.len() >= min_len {
            pages.push((page_num, current_content.trim().to_string()));
        }

        // Handle edge case: if content is too short for even one page, still return it
        if pages.is_empty() && !content.trim().is_empty() {
            pages.push((1, content.trim().to_string()));
        }

        pages
    }

    /// Parse Markdown content into pages.
    ///
    /// First attempts to detect `*Page N*` markers. If none are found,
    /// falls back to size-based splitting.
    ///
    /// # Arguments
    /// * `content` - The markdown content to parse
    /// * `fallback_chars_per_page` - Target characters per page when no markers found
    /// * `min_page_length` - Minimum content length for a page to be included.
    ///   If `None`, uses [`DEFAULT_MIN_PAGE_CONTENT_LENGTH`].
    pub fn parse(
        content: &str,
        fallback_chars_per_page: Option<usize>,
        min_page_length: Option<usize>,
    ) -> Vec<(usize, String)> {
        if Self::has_page_markers(content) {
            Self::split_by_page_markers(content, min_page_length)
        } else {
            Self::split_by_size(
                content,
                fallback_chars_per_page.unwrap_or(DEFAULT_CHARS_PER_PAGE),
                min_page_length,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_page_markers() {
        assert!(MarkdownPageParser::has_page_markers("Some text\n*Page 2*\nMore text"));
        assert!(MarkdownPageParser::has_page_markers("*page 1*\nContent"));
        assert!(MarkdownPageParser::has_page_markers("Content\n*Page 123*"));
        assert!(!MarkdownPageParser::has_page_markers("No markers here"));
        assert!(!MarkdownPageParser::has_page_markers("Page 2 without asterisks"));
    }

    #[test]
    fn test_split_by_page_markers() {
        let content = r#"# Title

Some intro content here that is long enough to be a valid page with plenty of text.

---
*Page 2*

This is page 2 content with enough text to be considered valid content for testing purposes.

---
*Page 3*

This is page 3 content also with sufficient length to pass the minimum content threshold.
"#;

        let pages = MarkdownPageParser::split_by_page_markers(content, None);

        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].0, 1); // First content before markers is page 1
        assert_eq!(pages[1].0, 2);
        assert_eq!(pages[2].0, 3);
        assert!(pages[0].1.contains("intro content"));
        assert!(pages[1].1.contains("page 2 content"));
        assert!(pages[2].1.contains("page 3 content"));
    }

    #[test]
    fn test_split_by_size() {
        let content = r#"Paragraph one with some content.

Paragraph two with more content here.

Paragraph three with even more content to add.

Paragraph four continues the document.

Paragraph five has additional text."#;

        // Very small page size to force multiple pages
        let pages = MarkdownPageParser::split_by_size(content, 100, None);

        assert!(pages.len() >= 2);
        assert_eq!(pages[0].0, 1);
        assert_eq!(pages[1].0, 2);
    }

    #[test]
    fn test_parse_with_markers() {
        let content = "*Page 1*\n\nFirst page with enough content to meet minimum length requirements.\n\n*Page 2*\n\nSecond page also with sufficient content for testing.";
        let pages = MarkdownPageParser::parse(content, None, None);

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].0, 1);
        assert_eq!(pages[1].0, 2);
    }

    #[test]
    fn test_parse_without_markers() {
        let content = "Just some content without any page markers but with enough text to be meaningful.";
        let pages = MarkdownPageParser::parse(content, Some(50), None);

        assert!(!pages.is_empty());
        assert_eq!(pages[0].0, 1);
    }

    #[test]
    fn test_empty_content() {
        let pages = MarkdownPageParser::parse("", None, None);
        assert!(pages.is_empty());

        let pages = MarkdownPageParser::split_by_page_markers("", None);
        assert!(pages.is_empty());

        let pages = MarkdownPageParser::split_by_size("", 1000, None);
        assert!(pages.is_empty());
    }
}
