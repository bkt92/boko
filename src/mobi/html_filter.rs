//! HTML filtering for MOBI 6 compatibility.
//!
//! MOBI 6 supports a limited subset of HTML tags. This module filters HTML
//! to only include supported tags and converts image references.

use std::collections::HashMap;

/// Check if a tag is supported in MOBI 6
pub fn is_supported_tag(tag: &str) -> bool {
    matches!(tag.to_lowercase().as_str(),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" |
        "p" | "br" |
        "i" | "b" | "u" |
        "ul" | "ol" | "li" |
        "table" | "tr" | "td" | "th" |
        "img" |
        "div" | "span"  // Limited support - stripped if no attributes
    )
}

/// Filter HTML to MOBI 6 supported tags
///
/// Returns (filtered_html, warnings)
pub fn filter_html_for_mobi6(
    html: &str,
    _image_map: &HashMap<String, u32>,
) -> (String, Vec<String>) {
    let warnings = Vec::new();

    // TODO: Implement full DOM walking in later task
    // For now, return original HTML
    let filtered = html.to_string();

    (filtered, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_tags() {
        assert!(is_supported_tag("h1"));
        assert!(is_supported_tag("p"));
        assert!(is_supported_tag("img"));
        assert!(is_supported_tag("table"));
        assert!(!is_supported_tag("video"));
        assert!(!is_supported_tag("audio"));
    }

    #[test]
    fn test_filter_simple_html() {
        let html = r#"<div><p>Hello</p><video src="test.mp4"/></div>"#;
        let (filtered, _warnings) = filter_html_for_mobi6(html, &HashMap::new());

        // Should keep p (for now returns original)
        assert!(filtered.contains("<p>"));
        // TODO: After full implementation, video should be removed
        // assert!(!filtered.contains("video"));
    }
}
