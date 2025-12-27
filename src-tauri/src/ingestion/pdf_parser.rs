use std::path::Path;
use std::error::Error;
use lopdf::Document;

pub struct PDFParser;

impl PDFParser {
    pub fn extract_text(path: &Path) -> Result<String, Box<dyn Error>> {
        let doc = Document::load(path)?;
        // Simple extraction for now.
        // A proper implementation would iterate pages and extract text content.
        let mut text = String::new();
        for (page_num, page_id) in doc.get_pages() {
             let content = doc.extract_text(&[page_num])?;
             text.push_str(&content);
             text.push('\n');
        }
        Ok(text)
    }
}
