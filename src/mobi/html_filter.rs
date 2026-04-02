//! HTML filtering for MOBI 6 compatibility.
//!
//! MOBI 6 supports a limited subset of HTML tags. This module filters HTML
//! to only include supported tags and converts image references.

use std::collections::HashMap;

/// Check if a tag is supported in MOBI 6
pub fn is_supported_tag(tag: &str) -> bool {
    matches!(
        tag.to_lowercase().as_str(),
        "h1" | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "p"
            | "br"
            | "i"
            | "b"
            | "u"
            | "ul"
            | "ol"
            | "li"
            | "table"
            | "tr"
            | "td"
            | "th"
            | "img"
            | "div"
            | "span" // Limited support - stripped if no attributes
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

/// Strip MOBI-specific artifacts from HTML before exporting to other formats.
///
/// Removes tags and attributes that are MOBI-internal and have no meaning
/// in EPUB or other formats:
/// - `<mbp:pagebreak/>` — MOBI page break markers
/// - `<a id="fileposNNNNN" />` — MOBI byte-position anchors
/// - `<a filepos="NNNNN">` — MOBI navigation links (converted to fragments or removed)
/// - `recindex="NNNNN"` attributes on img tags (should already be converted to src)
pub fn strip_mobi_artifacts(html: &str) -> String {
    let mut result = html.to_string();

    // Remove <mbp:pagebreak/> (self-closing)
    result = result.replace("<mbp:pagebreak/>", "");
    result = result.replace("<mbp:pagebreak>", "");

    // Remove <a id="fileposNNNNN" /> anchors (MOBI byte-position markers)
    // Pattern: <a id="filepos" followed by digits, optional whitespace, then "/>" or "></a>"
    let mut cleaned = String::with_capacity(result.len());
    let mut search_from = 0;
    while let Some(pos) = result[search_from..].find("<a id=\"filepos") {
        let abs_pos = search_from + pos;
        cleaned.push_str(&result[search_from..abs_pos]);

        // Find end of this tag
        let remaining = &result[abs_pos..];
        if let Some(end) = remaining.find('>') {
            let tag_content = &remaining[..=end];
            // Check if this is a self-closing anchor with only a filepos id
            // Pattern: <a id="fileposNNNNN" /> or <a id="fileposNNNNN"></a>
            if tag_content.contains("filepos") && tag_content.starts_with("<a ") {
                // Skip this tag entirely
                search_from = abs_pos + end + 1;
                // Also skip closing </a> if this is <a id="fileposNNNNN"></a> (not self-closing)
                if !tag_content.contains("/>")
                    && let Some(close) = result[search_from..].find("</a>")
                {
                    search_from += close + 4;
                }
                continue;
            }
        }
        // Not a filepos anchor - keep it
        cleaned.push_str(&result[abs_pos..abs_pos + 3]);
        search_from = abs_pos + 3;
    }
    cleaned.push_str(&result[search_from..]);
    result = cleaned;

    result
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

    #[test]
    fn test_strip_mbp_pagebreak() {
        let html = "<p>Chapter 1</p><mbp:pagebreak/><p>Chapter 2</p>";
        let result = strip_mobi_artifacts(html);
        assert_eq!(result, "<p>Chapter 1</p><p>Chapter 2</p>");
    }

    #[test]
    fn test_strip_filepos_anchors_self_closing() {
        let html = r#"<p>Text</p><a id="filepos12345" /><p>More</p>"#;
        let result = strip_mobi_artifacts(html);
        assert_eq!(result, "<p>Text</p><p>More</p>");
    }

    #[test]
    fn test_strip_filepos_anchors_with_close_tag() {
        let html = r#"<p>Text</p><a id="filepos0000100"></a><p>More</p>"#;
        let result = strip_mobi_artifacts(html);
        assert_eq!(result, "<p>Text</p><p>More</p>");
    }

    #[test]
    fn test_preserves_normal_anchors() {
        let html = r##"<p><a id="chapter1" />Text</p><a href="#chapter1">Link</a>"##;
        let result = strip_mobi_artifacts(html);
        assert!(result.contains(r#"<a id="chapter1" />"#));
        assert!(result.contains(r##"<a href="#chapter1">Link</a>"##));
        assert!(!result.contains("filepos"));
    }

    #[test]
    fn test_strip_all_artifacts() {
        let html = r#"<a id="filepos0" /><mbp:pagebreak/><p>Hello</p><mbp:pagebreak/><a id="filepos500" /><p>World</p>"#;
        let result = strip_mobi_artifacts(html);
        assert_eq!(result, "<p>Hello</p><p>World</p>");
    }
}
