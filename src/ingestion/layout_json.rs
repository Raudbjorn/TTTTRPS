//! Layout JSON Importer
//!
//! Imports pre-extracted document layout JSON files (Anthropic format).
//! This format contains structured page and element data from document extraction.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

use super::kreuzberg_extractor::Page;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum LayoutJsonError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, LayoutJsonError>;

// ============================================================================
// Layout JSON Schema Types
// ============================================================================

/// Root layout document structure (Anthropic format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutDocument {
    /// Schema version (e.g., "1.0.0")
    pub version: String,

    /// Document metadata
    pub metadata: LayoutMetadata,

    /// Array of pages with elements
    pub pages: Vec<LayoutPage>,
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutMetadata {
    /// Document title
    #[serde(default)]
    pub title: Option<String>,

    /// Producer/generator info
    #[serde(default)]
    pub producer: Option<String>,

    /// Total page count
    #[serde(default)]
    pub page_count: Option<usize>,

    /// Total possible pages (may differ from actual extracted pages)
    #[serde(default)]
    pub total_possible_pages: Option<usize>,

    /// Additional metadata fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A single page in the layout document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPage {
    /// Page number (1-indexed)
    pub page_number: usize,

    /// Page width in points
    #[serde(default)]
    pub width: f64,

    /// Page height in points
    #[serde(default)]
    pub height: f64,

    /// Page rotation in degrees
    #[serde(default)]
    pub rotation: i32,

    /// Page region information
    #[serde(default)]
    pub regions: Option<PageRegions>,

    /// Elements on this page
    #[serde(default)]
    pub elements: Vec<LayoutElement>,

    /// Reading order (optional)
    #[serde(default)]
    pub reading_order: Vec<String>,

    /// Page metrics
    #[serde(default)]
    pub metrics: Option<PageMetrics>,

    /// Whether OCR was applied to this page
    #[serde(default)]
    pub ocr_applied: bool,
}

/// Page region/margin information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageRegions {
    pub left_margin: f64,
    pub right_margin: f64,
    pub top_margin: f64,
    pub bottom_margin: f64,
    pub column_count: u32,
}

/// Page metrics and statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageMetrics {
    pub text_block_count: u32,
    pub table_count: u32,
    pub list_count: u32,
    pub figure_count: u32,
    pub image_count: u32,
    pub heading_count: u32,
    pub median_confidence: f32,
    pub min_confidence: f32,
    pub column_count: u32,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A content element on a page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutElement {
    /// Element type (e.g., "TextBlock", "Table", "Image")
    #[serde(rename = "type")]
    pub element_type: String,

    /// Unique element ID
    #[serde(default)]
    pub id: Option<String>,

    /// Bounding box
    #[serde(default)]
    pub bounding_box: Option<BoundingBox>,

    /// Semantic role (e.g., "title", "heading1", "body", "caption")
    #[serde(default)]
    pub role: Option<String>,

    /// Role confidence score (0.0-1.0)
    #[serde(default)]
    pub role_confidence: Option<f32>,

    /// Text content
    #[serde(default)]
    pub content: String,

    /// Reading order index
    #[serde(default)]
    pub reading_order_index: Option<usize>,

    /// Whether this element continues from the previous page
    #[serde(default)]
    pub continues_from_previous: bool,

    /// Whether this element continues to the next page
    #[serde(default)]
    pub continues_to_next: bool,

    /// Extraction confidence
    #[serde(default)]
    pub confidence: Option<f32>,

    /// Chunk boundary hint
    #[serde(default)]
    pub chunk_boundary: Option<String>,

    /// Additional element properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Element bounding box
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

// ============================================================================
// Implementation
// ============================================================================

impl LayoutDocument {
    /// Check if a file appears to be a layout JSON document.
    ///
    /// Performs a quick check for the presence of "version" and "pages" keys.
    pub fn is_layout_json(path: &Path) -> bool {
        // Check extension first
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.to_lowercase() != "json" {
            return false;
        }

        // Quick content check - read first ~500 bytes
        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };

        let reader = BufReader::new(file);
        let mut header = String::new();
        let mut bytes_read = 0;

        for line in reader.lines().take(20).flatten() {
            header.push_str(&line);
            bytes_read += line.len();
            if bytes_read > 500 {
                break;
            }
        }

        // Check for characteristic fields
        header.contains("\"version\"")
            && header.contains("\"pages\"")
            && (header.contains("\"metadata\"") || header.contains("\"elements\""))
    }

    /// Load and parse a layout JSON file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let doc: LayoutDocument = serde_json::from_str(&content)?;

        // Validate basic structure
        if doc.pages.is_empty() {
            return Err(LayoutJsonError::InvalidSchema(
                "Document has no pages".to_string(),
            ));
        }

        Ok(doc)
    }

    /// Parse layout JSON from a string.
    pub fn from_str(content: &str) -> Result<Self> {
        let doc: LayoutDocument = serde_json::from_str(content)?;

        if doc.pages.is_empty() {
            return Err(LayoutJsonError::InvalidSchema(
                "Document has no pages".to_string(),
            ));
        }

        Ok(doc)
    }

    /// Get the document title.
    pub fn title(&self) -> Option<&str> {
        self.metadata.title.as_deref()
    }

    /// Get the total page count.
    pub fn page_count(&self) -> usize {
        self.metadata
            .page_count
            .unwrap_or(self.pages.len())
    }

    /// Convert layout pages to simple Page structs for the ingestion pipeline.
    ///
    /// Concatenates element content in reading order.
    pub fn to_pages(&self) -> Vec<Page> {
        self.pages
            .iter()
            .map(|lp| {
                let content = lp.to_text_content();
                Page {
                    page_number: lp.page_number,
                    content,
                }
            })
            .collect()
    }

    /// Get pages with their full element data preserved.
    pub fn pages_with_elements(&self) -> &[LayoutPage] {
        &self.pages
    }
}

impl LayoutPage {
    /// Convert page elements to concatenated text content.
    ///
    /// Elements are sorted by reading_order_index if available,
    /// otherwise by their array order.
    pub fn to_text_content(&self) -> String {
        let mut elements: Vec<_> = self.elements.iter().collect();

        // Sort by reading order if available
        elements.sort_by_key(|e| e.reading_order_index.unwrap_or(usize::MAX));

        let mut content = String::new();
        let mut prev_role: Option<&str> = None;

        for element in elements {
            if element.content.trim().is_empty() {
                continue;
            }

            // Add appropriate spacing based on role changes
            if !content.is_empty() {
                let current_role = element.role.as_deref();

                // Add extra spacing for major role transitions
                if matches!(
                    current_role,
                    Some("title") | Some("heading1") | Some("heading2") | Some("heading3")
                ) || matches!(
                    prev_role,
                    Some("title") | Some("heading1") | Some("heading2") | Some("heading3")
                ) {
                    content.push_str("\n\n");
                } else {
                    content.push('\n');
                }
            }

            content.push_str(element.content.trim());
            prev_role = element.role.as_deref();
        }

        content
    }

    /// Get elements filtered by role.
    pub fn elements_by_role(&self, role: &str) -> Vec<&LayoutElement> {
        self.elements
            .iter()
            .filter(|e| e.role.as_deref() == Some(role))
            .collect()
    }

    /// Get the main heading/title for this page.
    pub fn heading(&self) -> Option<&str> {
        self.elements
            .iter()
            .find(|e| matches!(e.role.as_deref(), Some("title") | Some("heading1")))
            .map(|e| e.content.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"{
        "version": "1.0.0",
        "metadata": {
            "title": "Test Document",
            "page_count": 2
        },
        "pages": [
            {
                "page_number": 1,
                "elements": [
                    {"type": "TextBlock", "role": "title", "content": "Chapter 1"},
                    {"type": "TextBlock", "role": "body", "content": "First paragraph."}
                ]
            },
            {
                "page_number": 2,
                "elements": [
                    {"type": "TextBlock", "role": "body", "content": "Second page content."}
                ]
            }
        ]
    }"#;

    #[test]
    fn test_parse_layout_json() {
        let doc = LayoutDocument::from_str(SAMPLE_JSON).unwrap();

        assert_eq!(doc.version, "1.0.0");
        assert_eq!(doc.title(), Some("Test Document"));
        assert_eq!(doc.page_count(), 2);
        assert_eq!(doc.pages.len(), 2);
    }

    #[test]
    fn test_to_pages() {
        let doc = LayoutDocument::from_str(SAMPLE_JSON).unwrap();
        let pages = doc.to_pages();

        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].page_number, 1);
        assert!(pages[0].content.contains("Chapter 1"));
        assert!(pages[0].content.contains("First paragraph"));
        assert_eq!(pages[1].page_number, 2);
        assert!(pages[1].content.contains("Second page content"));
    }

    #[test]
    fn test_page_to_text_content() {
        let doc = LayoutDocument::from_str(SAMPLE_JSON).unwrap();
        let content = doc.pages[0].to_text_content();

        // Title should have extra spacing
        assert!(content.contains("Chapter 1\n\nFirst paragraph"));
    }

    #[test]
    fn test_page_heading() {
        let doc = LayoutDocument::from_str(SAMPLE_JSON).unwrap();

        assert_eq!(doc.pages[0].heading(), Some("Chapter 1"));
        assert_eq!(doc.pages[1].heading(), None);
    }

    #[test]
    fn test_empty_pages_error() {
        let empty_json = r#"{"version": "1.0.0", "metadata": {}, "pages": []}"#;
        let result = LayoutDocument::from_str(empty_json);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LayoutJsonError::InvalidSchema(_)
        ));
    }
}
