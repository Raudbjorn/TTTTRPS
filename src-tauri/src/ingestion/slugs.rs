//! Slug Generation Utilities
//!
//! This module provides deterministic, filesystem-safe slug generation
//! for Meilisearch index names. Each document gets a unique slug that
//! is used as the base for its indexes:
//! - `<slug>-raw` for raw page-level documents
//! - `<slug>` for semantic chunks

use std::path::Path;

/// Maximum length for generated slugs (Meilisearch index name limit is 400)
pub const MAX_SLUG_LENGTH: usize = 64;

/// Generate a deterministic, filesystem-safe slug from a file path.
///
/// The slug is used as the base name for Meilisearch indexes:
/// - `<slug>-raw` for raw page-level documents
/// - `<slug>` for semantic chunks
///
/// # Rules
/// - Lowercase alphanumeric characters and hyphens only
/// - No consecutive hyphens
/// - No leading/trailing hyphens
/// - Deterministic: same input always produces same output
/// - Truncated to MAX_SLUG_LENGTH characters
///
/// # Examples
/// ```
/// use std::path::Path;
/// use ttrpg_assistant::ingestion::slugs::generate_source_slug;
///
/// let slug = generate_source_slug(Path::new("Delta Green - Handler's Guide.pdf"), None);
/// assert_eq!(slug, "delta-green-handlers-guide");
/// ```
pub fn generate_source_slug(path: &Path, title_override: Option<&str>) -> String {
    // Use title override if provided, otherwise extract from filename
    let base_name = title_override
        .map(|s| s.to_string())
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unnamed".to_string());

    slugify(&base_name)
}

/// Convert any string to a clean slug.
///
/// Handles Unicode by attempting transliteration of common characters,
/// then falling back to stripping non-ASCII.
pub fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_was_hyphen = true; // Start true to avoid leading hyphen

    for c in input.chars() {
        match c {
            // Direct passthrough for lowercase alphanumeric
            'a'..='z' | '0'..='9' => {
                slug.push(c);
                last_was_hyphen = false;
            }
            // Convert uppercase to lowercase
            'A'..='Z' => {
                slug.push(c.to_ascii_lowercase());
                last_was_hyphen = false;
            }
            // Common separators become hyphens
            ' ' | '-' | '_' | '.' | '/' | '\\' | ':' | ',' | ';' | '(' | ')' | '[' | ']' => {
                if !last_was_hyphen {
                    slug.push('-');
                    last_was_hyphen = true;
                }
            }
            // Transliterate common Unicode characters (lowercase and uppercase)
            'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' | 'Á' | 'À' | 'Ä' | 'Â' | 'Ã' | 'Å' => {
                slug.push('a');
                last_was_hyphen = false;
            }
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => {
                slug.push('e');
                last_was_hyphen = false;
            }
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => {
                slug.push('i');
                last_was_hyphen = false;
            }
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'ø' | 'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' | 'Ø' => {
                slug.push('o');
                last_was_hyphen = false;
            }
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => {
                slug.push('u');
                last_was_hyphen = false;
            }
            'ñ' | 'Ñ' => {
                slug.push('n');
                last_was_hyphen = false;
            }
            'ç' | 'Ç' => {
                slug.push('c');
                last_was_hyphen = false;
            }
            'ß' => {
                slug.push_str("ss");
                last_was_hyphen = false;
            }
            'æ' | 'Æ' => {
                slug.push_str("ae");
                last_was_hyphen = false;
            }
            'œ' | 'Œ' => {
                slug.push_str("oe");
                last_was_hyphen = false;
            }
            'þ' | 'Þ' => {
                slug.push_str("th");
                last_was_hyphen = false;
            }
            'ð' | 'Ð' => {
                slug.push('d');
                last_was_hyphen = false;
            }
            // Apostrophes and quotes are dropped (don't become hyphens)
            // ASCII quotes
            '\'' | '"' | '`' => {}
            // Unicode curly quotes (using escape sequences)
            '\u{2018}' | '\u{2019}' | '\u{201C}' | '\u{201D}' => {}
            // Skip other characters
            _ => {}
        }
    }

    // Remove trailing hyphen
    while slug.ends_with('-') {
        slug.pop();
    }

    // Truncate to max length at word boundary if possible
    if slug.len() > MAX_SLUG_LENGTH {
        // Find last hyphen before limit
        if let Some(pos) = slug[..MAX_SLUG_LENGTH].rfind('-') {
            slug.truncate(pos);
        } else {
            slug.truncate(MAX_SLUG_LENGTH);
        }
    }

    // Fallback for empty slugs
    if slug.is_empty() {
        slug = "unnamed".to_string();
    }

    slug
}

/// Generate the raw index name for a source slug
pub fn raw_index_name(slug: &str) -> String {
    format!("{}-raw", slug)
}

/// Generate the chunks index name for a source slug (same as slug)
pub fn chunks_index_name(slug: &str) -> String {
    slug.to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("simple"), "simple");
        assert_eq!(slugify("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_slugify_special_characters() {
        assert_eq!(
            slugify("Delta Green - Handler's Guide"),
            "delta-green-handlers-guide"
        );
        assert_eq!(slugify("D&D 5th Edition"), "dd-5th-edition");
        assert_eq!(
            slugify("Call of Cthulhu (7th Ed)"),
            "call-of-cthulhu-7th-ed"
        );
        assert_eq!(
            slugify("Monster Manual: Expanded"),
            "monster-manual-expanded"
        );
    }

    #[test]
    fn test_slugify_unicode() {
        assert_eq!(slugify("Über"), "uber");
        assert_eq!(slugify("naïve"), "naive");
        assert_eq!(slugify("café"), "cafe");
        assert_eq!(slugify("Müller"), "muller");
        assert_eq!(slugify("Ægis"), "aegis");
        assert_eq!(slugify("Þórr"), "thorr");
    }

    #[test]
    fn test_slugify_consecutive_separators() {
        assert_eq!(slugify("hello   world"), "hello-world");
        assert_eq!(slugify("hello---world"), "hello-world");
        assert_eq!(slugify("hello___world"), "hello-world");
        assert_eq!(slugify("hello - world"), "hello-world");
    }

    #[test]
    fn test_slugify_leading_trailing() {
        assert_eq!(slugify("  hello  "), "hello");
        assert_eq!(slugify("--hello--"), "hello");
        assert_eq!(slugify("123_test"), "123-test");
    }

    #[test]
    fn test_slugify_empty_and_special() {
        assert_eq!(slugify(""), "unnamed");
        assert_eq!(slugify("!!!"), "unnamed");
        assert_eq!(slugify("'\""), "unnamed");
    }

    #[test]
    fn test_slugify_long_input() {
        let long_input = "This Is A Very Long Title That Exceeds The Maximum Slug Length And Should Be Truncated At A Word Boundary";
        let slug = slugify(long_input);
        assert!(slug.len() <= MAX_SLUG_LENGTH);
        assert!(!slug.ends_with('-'));
    }

    #[test]
    fn test_slugify_numbers() {
        assert_eq!(slugify("5th Edition"), "5th-edition");
        assert_eq!(slugify("2024"), "2024");
        assert_eq!(slugify("D&D 3.5"), "dd-3-5");
    }

    #[test]
    fn test_generate_source_slug_from_path() {
        let path = Path::new("/home/user/rpg/Delta Green - Handler's Guide.pdf");
        assert_eq!(
            generate_source_slug(path, None),
            "delta-green-handlers-guide"
        );

        let path = Path::new("Monster_Manual_5e.pdf");
        assert_eq!(generate_source_slug(path, None), "monster-manual-5e");
    }

    #[test]
    fn test_generate_source_slug_with_override() {
        let path = Path::new("file123.pdf");
        assert_eq!(
            generate_source_slug(path, Some("Player's Handbook")),
            "players-handbook"
        );
    }

    #[test]
    fn test_index_name_helpers() {
        assert_eq!(raw_index_name("delta-green"), "delta-green-raw");
        assert_eq!(chunks_index_name("delta-green"), "delta-green");
    }

    #[test]
    fn test_slugify_deterministic() {
        // Same input should always produce same output
        let input = "Delta Green: Handler's Guide (2nd Printing)";
        let slug1 = slugify(input);
        let slug2 = slugify(input);
        let slug3 = slugify(input);
        assert_eq!(slug1, slug2);
        assert_eq!(slug2, slug3);
    }
}
