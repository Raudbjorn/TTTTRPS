//! DOCX Parser Module
//!
//! Extracts text from Microsoft Word DOCX files for ingestion.
//! DOCX files are ZIP archives containing XML content.

use std::io::{Read, BufReader};
use std::path::Path;
use thiserror::Error;
use quick_xml::events::Event;
use quick_xml::Reader;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum DOCXError {
    #[error("Failed to open DOCX file: {0}")]
    OpenError(String),

    #[error("Failed to read document.xml: {0}")]
    ReadError(String),

    #[error("XML parsing error: {0}")]
    XmlError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    ZipError(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, DOCXError>;

// ============================================================================
// Extracted Content Types
// ============================================================================

/// Extracted DOCX content
#[derive(Debug, Clone)]
pub struct ExtractedDOCX {
    /// Source file path
    pub source_path: String,
    /// Full extracted text
    pub text: String,
    /// Extracted paragraphs
    pub paragraphs: Vec<String>,
}

// ============================================================================
// DOCX Parser
// ============================================================================

pub struct DOCXParser;

impl DOCXParser {
    /// Extract text from a DOCX file
    pub fn extract_text(path: &Path) -> Result<String> {
        let extracted = Self::extract_structured(path)?;
        Ok(extracted.text)
    }

    /// Extract structured content from a DOCX file
    pub fn extract_structured(path: &Path) -> Result<ExtractedDOCX> {
        let file = std::fs::File::open(path)
            .map_err(|e| DOCXError::OpenError(e.to_string()))?;

        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;

        // Try to read word/document.xml (main document content)
        let document_xml = Self::read_document_xml(&mut archive)?;

        // Parse XML and extract text
        let (text, paragraphs) = Self::parse_document_xml(&document_xml)?;

        Ok(ExtractedDOCX {
            source_path: path.to_string_lossy().to_string(),
            text,
            paragraphs,
        })
    }

    /// Read the document.xml file from the DOCX archive
    fn read_document_xml<R: Read + std::io::Seek>(archive: &mut zip::ZipArchive<R>) -> Result<String> {
        // DOCX standard location for main document
        let mut document_file = archive.by_name("word/document.xml")
            .map_err(|e| DOCXError::ReadError(format!("Cannot find word/document.xml: {}", e)))?;

        let mut xml_content = String::new();
        document_file.read_to_string(&mut xml_content)
            .map_err(|e| DOCXError::ReadError(e.to_string()))?;

        Ok(xml_content)
    }

    /// Parse the document.xml and extract text content
    fn parse_document_xml(xml: &str) -> Result<(String, Vec<String>)> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut text_parts = Vec::new();
        let mut paragraphs = Vec::new();
        let mut current_paragraph = String::new();
        let mut in_text = false;
        let mut in_paragraph = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    match e.name().as_ref() {
                        b"w:t" => in_text = true,
                        b"w:p" => {
                            in_paragraph = true;
                            current_paragraph.clear();
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    match e.name().as_ref() {
                        b"w:t" => in_text = false,
                        b"w:p" => {
                            in_paragraph = false;
                            let para = current_paragraph.trim().to_string();
                            if !para.is_empty() {
                                paragraphs.push(para.clone());
                                text_parts.push(para);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_text {
                        let text = e.unescape()
                            .map_err(|e| DOCXError::XmlError(e.to_string()))?;
                        if in_paragraph {
                            current_paragraph.push_str(&text);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(DOCXError::XmlError(format!(
                    "Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))),
                _ => {}
            }
        }

        let full_text = text_parts.join("\n\n");
        Ok((full_text, paragraphs))
    }

    /// Get paragraph count without extracting full content
    pub fn get_paragraph_count(path: &Path) -> Result<usize> {
        let extracted = Self::extract_structured(path)?;
        Ok(extracted.paragraphs.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_xml() {
        let xml = r#"<?xml version="1.0"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:t>Hello World</w:t>
                    </w:r>
                </w:p>
                <w:p>
                    <w:r>
                        <w:t>Second paragraph</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let result = DOCXParser::parse_document_xml(xml);
        assert!(result.is_ok());

        let (text, paragraphs) = result.unwrap();
        assert_eq!(paragraphs.len(), 2);
        assert!(text.contains("Hello World"));
        assert!(text.contains("Second paragraph"));
    }
}
