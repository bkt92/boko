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

/// Strip MOBI/Kindle-specific artifacts from HTML before exporting to other formats.
///
/// Removes tags and attributes that are MOBI/Kindle-internal and have no meaning
/// in EPUB or other formats:
/// - `<mbp:pagebreak/>` — MOBI page break markers
/// - `<a id="fileposNNNNN" />` — MOBI byte-position anchors
/// - `aid="..."` attributes — Amazon anchor IDs
/// - `data-Amzn*` / `data-amzn*` attributes — Amazon tracking attributes
/// - `recindex="NNNNN"` attributes on img tags (should already be converted to src)
pub fn strip_mobi_artifacts(html: &str) -> String {
    // Phase 1: Strip Kindle/Amazon-specific attributes (aid, data-Amzn*)
    // Uses the existing high-performance byte-level stripper from transform.rs
    let cleaned_bytes = super::transform::strip_kindle_attributes_fast(html.as_bytes());
    let mut result = String::from_utf8_lossy(&cleaned_bytes).to_string();

    // Phase 2: Remove <mbp:pagebreak/> tags
    result = result.replace("<mbp:pagebreak/>", "");
    result = result.replace("<mbp:pagebreak>", "");

    // Phase 3: Remove <a id="fileposNNNNN" /> anchors (MOBI byte-position markers)
    let mut cleaned = String::with_capacity(result.len());
    let mut search_from = 0;
    while let Some(pos) = result[search_from..].find("<a id=\"filepos") {
        let abs_pos = search_from + pos;
        cleaned.push_str(&result[search_from..abs_pos]);

        let remaining = &result[abs_pos..];
        if let Some(end) = remaining.find('>') {
            let tag_content = &remaining[..=end];
            if tag_content.contains("filepos") && tag_content.starts_with("<a ") {
                search_from = abs_pos + end + 1;
                if !tag_content.contains("/>")
                    && let Some(close) = result[search_from..].find("</a>")
                {
                    search_from += close + 4;
                }
                continue;
            }
        }
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

    #[test]
    fn test_strip_aid_attribute() {
        let html = r#"<p aid="0001">Hello</p><div aid="00AB">World</div>"#;
        let result = strip_mobi_artifacts(html);
        assert!(
            !result.contains("aid="),
            "aid should be removed: {}",
            result
        );
        assert!(
            result.contains("Hello</p>"),
            "content preserved: {}",
            result
        );
        assert!(
            result.contains("World</div>"),
            "content preserved: {}",
            result
        );
    }

    #[test]
    fn test_strip_amzn_data_attributes() {
        let html = r#"<p data-AmznRemoved="true">Text</p><span data-amzn-track="1">More</span>"#;
        let result = strip_mobi_artifacts(html);
        assert!(
            !result.contains("data-Amzn"),
            "data-Amzn should be removed: {}",
            result
        );
        assert!(
            !result.contains("data-amzn"),
            "data-amzn should be removed: {}",
            result
        );
        assert!(result.contains("Text</p>"));
        assert!(result.contains("More</span>"));
    }

    #[test]
    fn test_combined_mobi_artifacts() {
        let html = r#"<a id="filepos100" /><mbp:pagebreak/><p aid="0050">Chapter</p><img src="cover.jpg" data-AmznPageBreak="true"/>"#;
        let result = strip_mobi_artifacts(html);
        assert!(!result.contains("filepos"), "filepos anchors removed");
        assert!(!result.contains("mbp:"), "mbp tags removed");
        assert!(!result.contains("aid="), "aid attributes removed");
        assert!(!result.contains("data-Amzn"), "data-Amzn removed");
        assert!(result.contains("Chapter</p>"), "content preserved");
        assert!(
            result.contains(r#"src="cover.jpg""#),
            "img src preserved without amzn attr"
        );
    }
}
