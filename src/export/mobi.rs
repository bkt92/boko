//! MOBI 6 exporter.
//!
//! Creates MOBI 6 (legacy Kindle format) files from Book structures.

use std::collections::HashMap;
use std::io::{self, Seek, Write};
use std::path::Path;

use crate::mobi::index::{NcxBuildEntry, build_ncx_indx};
use crate::model::{AnchorTarget, Book, Metadata, TocEntry};

use super::{Exporter, resolve_cover_asset};

/// Find a safe UTF-8 boundary near `max_len` bytes from the start.
///
/// Walks backward from `max_len` to avoid splitting a multibyte character.
/// Returns the safe split position.
fn find_utf8_boundary(bytes: &[u8], max_len: usize) -> usize {
    let mut end = max_len.min(bytes.len());
    // A continuation byte has pattern 10xxxxxx (0x80-0xBF)
    while end > 0 && end < bytes.len() && (bytes[end] & 0xC0) == 0x80 {
        end -= 1;
    }
    end
}

/// Map BCP-47 language tag to MOBI language code.
fn language_code(lang: &str) -> u32 {
    let lang_lower = lang.to_lowercase();
    let primary = lang_lower.split('-').next().unwrap_or("");
    match primary {
        "en" => 0x09,
        "de" => 0x07,
        "fr" => 0x0C,
        "es" => 0x0A,
        "it" => 0x11,
        "ja" => 0x15,
        "zh" => 0x19,
        "ko" => 0x12,
        "pt" => 0x16,
        "ru" => 0x17,
        "nl" => 0x13,
        "sv" => 0x1A,
        "da" => 0x06,
        "fi" => 0x0B,
        "el" => 0x08,
        "cs" => 0x05,
        "pl" => 0x15,
        "tr" => 0x1F,
        "ar" => 0x01,
        "he" => 0x0D,
        "th" => 0x1D,
        "hu" => 0x0E,
        "no" => 0x14,
        _ => 0x09, // Default to English
    }
}

/// Find byte position of an element by its id attribute in HTML.
fn find_element_position(html: &str, element_id: &str) -> Option<u32> {
    let pattern = format!("id=\"{}\"", element_id);
    html.find(&pattern).map(|pos| pos as u32)
}

/// Flatten hierarchical TOC into a linear list of NcxBuildEntry for MOBI 6.
///
/// Walks TocEntry tree recursively, resolves byte positions in the HTML,
/// calculates section lengths, and sets parent/child indices for the binary NCX.
fn flatten_toc_for_mobi(
    entries: &[TocEntry],
    html_content: &str,
    chapter_positions: &HashMap<String, u32>,
    text_length: u32,
) -> Vec<NcxBuildEntry> {
    struct TempEntry {
        pos: u32,
        length: u32,
        label: String,
        depth: u32,
        parent: i32,
        children: Vec<usize>,
    }

    let mut result: Vec<TempEntry> = Vec::new();

    fn resolve_position(
        entry: &TocEntry,
        html_content: &str,
        chapter_positions: &HashMap<String, u32>,
    ) -> u32 {
        if let Some(ref target) = entry.target {
            match target {
                AnchorTarget::Chapter(chapter_id) => {
                    let key = format!("ChapterId({})", chapter_id.0);
                    if let Some(&pos) = chapter_positions.get(&key) {
                        return pos;
                    }
                }
                AnchorTarget::Internal(node_id) => {
                    let key = format!("ChapterId({})", node_id.chapter.0);
                    if let Some(&pos) = chapter_positions.get(&key) {
                        return pos;
                    }
                }
                _ => {}
            }
        }

        let fragment = if let Some(hash_pos) = entry.href.find('#') {
            &entry.href[hash_pos + 1..]
        } else {
            ""
        };

        if let Some(stripped) = fragment.strip_prefix("filepos") {
            if let Ok(pos) = stripped.parse::<u32>() {
                return pos;
            }
        } else if !fragment.is_empty()
            && let Some(pos) = find_element_position(html_content, fragment)
        {
            return pos;
        }

        let key = format!("ChapterId({})", entry.href);
        if let Some(&pos) = chapter_positions.get(&key) {
            return pos;
        }

        0
    }

    fn flatten_recursive(
        entries: &[TocEntry],
        depth: u32,
        parent_idx: i32,
        html_content: &str,
        chapter_positions: &HashMap<String, u32>,
        result: &mut Vec<TempEntry>,
    ) {
        for entry in entries {
            let current_idx = result.len();
            let pos = resolve_position(entry, html_content, chapter_positions);

            result.push(TempEntry {
                pos,
                length: 0,
                label: entry.title.clone(),
                depth,
                parent: parent_idx,
                children: Vec::new(),
            });

            if parent_idx >= 0 {
                result[parent_idx as usize].children.push(current_idx);
            }

            flatten_recursive(
                &entry.children,
                depth + 1,
                current_idx as i32,
                html_content,
                chapter_positions,
                result,
            );
        }
    }

    flatten_recursive(entries, 0, -1, html_content, chapter_positions, &mut result);

    // Sort by position, then calculate lengths
    let mut indexed: Vec<(usize, u32)> =
        result.iter().enumerate().map(|(i, e)| (i, e.pos)).collect();
    indexed.sort_by_key(|&(_, pos)| pos);

    let sorted_positions: Vec<u32> = indexed.iter().map(|&(_, pos)| pos).collect();
    for (rank, &(orig_idx, pos)) in indexed.iter().enumerate() {
        let next_pos = sorted_positions
            .get(rank + 1)
            .copied()
            .unwrap_or(text_length);
        result[orig_idx].length = next_pos.saturating_sub(pos);
    }

    // Remove entries with zero length (duplicates at same position)
    let mut keep = vec![true; result.len()];
    for i in 1..indexed.len() {
        if indexed[i].1 == indexed[i - 1].1 {
            keep[indexed[i].0] = false;
        }
    }

    let mut old_to_new: Vec<i32> = vec![-1; result.len()];
    for (i, should_keep) in keep.iter().enumerate() {
        if *should_keep {
            old_to_new[i] = old_to_new.len() as i32;
        }
    }

    result
        .into_iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, e)| NcxBuildEntry {
            pos: e.pos,
            length: e.length,
            label: e.label,
            depth: e.depth,
            parent: if e.parent >= 0 {
                old_to_new.get(e.parent as usize).copied().unwrap_or(-1)
            } else {
                -1
            },
            first_child: e
                .children
                .first()
                .and_then(|&c| old_to_new.get(c).copied())
                .unwrap_or(-1),
            last_child: e
                .children
                .last()
                .and_then(|&c| old_to_new.get(c).copied())
                .unwrap_or(-1),
        })
        .collect()
}

/// Encode a backward-encoded variable-width integer (bit 8 set on last byte).
fn bvwi(mut val: u32) -> Vec<u8> {
    let mut result = Vec::new();
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val > 0 {
            byte |= 0x80; // continuation bit
        }
        result.push(byte);
        if val == 0 {
            break;
        }
    }
    result
}

/// Build TBS (Trailing Byte Sequence) for a text record.
///
/// TBS tells the Kindle reader which TOC entries span this record,
/// enabling "current chapter" display. The format is:
/// - Backward-encoded size prefix
/// - For each spanning entry: backward-encoded (offset, length) pairs
fn build_tbs(
    record_start: u32,
    record_end: u32,
    toc_entries: &[NcxBuildEntry],
) -> Vec<u8> {
    // Find entries that overlap with this record
    let spanning: Vec<(u32, u32)> = toc_entries
        .iter()
        .filter(|e| e.pos < record_end && (e.pos + e.length) > record_start)
        .map(|e| {
            let overlap_start = e.pos.max(record_start) - record_start;
            let overlap_end = (e.pos + e.length).min(record_end) - record_start;
            (overlap_start, overlap_end - overlap_start)
        })
        .collect();

    if spanning.is_empty() {
        return Vec::new();
    }

    let mut data = Vec::new();
    for (offset, length) in &spanning {
        data.extend(bvwi(*offset));
        data.extend(bvwi(*length));
    }

    // Backward-encoded size prefix
    let mut size_bytes = bvwi(data.len() as u32);
    size_bytes.reverse(); // reverse for backward encoding

    let mut result = size_bytes;
    result.extend_from_slice(&data);
    result
}

/// Configuration for MOBI 6 export.
#[derive(Debug, Clone)]
pub struct MobiConfig {
    /// Collect warnings during export
    pub collect_warnings: bool,
    /// Maximum image dimensions (width, height)
    pub max_image_size: (u32, u32),
    /// Maximum image file size in bytes
    pub max_image_file_size: u64,
    /// Character encoding
    pub encoding: MobiEncoding,
}

impl Default for MobiConfig {
    fn default() -> Self {
        Self {
            collect_warnings: true,
            max_image_size: (2048, 2048),
            max_image_file_size: 10 * 1024 * 1024, // 10MB
            encoding: MobiEncoding::Utf8,
        }
    }
}

/// Character encoding for MOBI 6 files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobiEncoding {
    /// UTF-8 (modern Kindles)
    Utf8,
    /// CP1252 (legacy devices)
    Cp1252,
}

/// MOBI 6 format exporter.
///
/// Creates MOBI 6 files compatible with older Kindle devices.
pub struct MobiExporter {
    config: MobiConfig,
}

impl MobiExporter {
    /// Create a new exporter with default configuration.
    pub fn new() -> Self {
        Self {
            config: MobiConfig::default(),
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &MobiConfig {
        &self.config
    }

    /// Configure the exporter with custom settings.
    pub fn with_config(mut self, config: MobiConfig) -> Self {
        self.config = config;
        self
    }

    /// Export with warnings collected.
    pub fn export_with_warnings<W: Write + Seek>(
        &self,
        book: &mut Book,
        writer: &mut W,
    ) -> io::Result<MobiResult> {
        // For now, just call regular export
        self.export(book, writer)?;
        Ok(MobiResult {
            success: true,
            warnings: Vec::new(),
        })
    }

    /// Replace image src attributes with MOBI 6 recindex format.
    ///
    /// MOBI 6 uses `<img recindex="NNNNN">` where NNNNN is a 5-digit, 1-based
    /// index into the image records. This matches Calibre's writer2 output and
    /// KindleUnpack's reading conventions.
    fn update_image_references(&self, html: &str, builder: &MobiBuilder) -> String {
        let mut result = html.to_string();
        let mut replaced = 0;

        for (image_path, &record_index) in &builder.image_path_to_record {
            // MOBI 6 recindex: 5-digit, 1-based decimal index
            let recindex = format!("{:05}", record_index);

            // Try to match src attributes with various path formats
            let filename = if let Some(name) = Path::new(image_path).file_name() {
                name.to_string_lossy().to_string()
            } else {
                image_path.clone()
            };

            let patterns = [
                format!("src=\"{}\"", image_path),
                format!("src='{}'", image_path),
                format!("src=\"{}\"", filename),
                format!("src='{}'", filename),
            ];

            let replacement = format!("recindex=\"{}\"", recindex);

            for pattern in &patterns {
                if result.contains(pattern) {
                    result = result.replace(pattern, &replacement);
                    replaced += 1;
                    break;
                }
            }
        }

        eprintln!(
            "MOBI: Stored {} images, replaced {} src attributes with recindex",
            builder.image_path_to_record.len(),
            replaced
        );
        result
    }

    /// Remove cover image from body HTML to prevent dual cover display.
    ///
    /// MOBI stores the cover as a dedicated image record (EXTH record 201/202),
    /// which Kindles display as the book cover. If the cover image also appears
    /// inline in the HTML body (e.g., from a cover page), the reader sees it twice.
    /// This strips the `<img recindex="NNNNN">` tag for the cover image record.
    fn strip_cover_from_body(&self, html: &str, builder: &MobiBuilder) -> String {
        let Some(cover_idx) = builder.cover_record_index else {
            return html.to_string();
        };

        let recindex = format!("recindex=\"{:05}\"", cover_idx);

        // Find all <img> tags containing the cover recindex and collect their ranges
        let mut removals: Vec<(usize, usize)> = Vec::new();
        let mut search_from = 0;
        while let Some(rel_pos) = html[search_from..].find("<img") {
            let img_start = search_from + rel_pos;
            if let Some(tag_rel_end) = html[img_start..].find('>') {
                let tag_end = img_start + tag_rel_end + 1;
                let tag = &html[img_start..tag_end];
                if tag.contains(&recindex) {
                    removals.push((img_start, tag_end));
                }
                search_from = tag_end;
            } else {
                break;
            }
        }

        if removals.is_empty() {
            return html.to_string();
        }

        // Build result without the removed tags
        let mut result = String::with_capacity(html.len());
        let mut last_end = 0;
        for (start, end) in &removals {
            result.push_str(&html[last_end..*start]);
            last_end = *end;
        }
        result.push_str(&html[last_end..]);
        result
    }

    /// Build HTML-based TOC for pure MOBI 6.
    ///
    /// MOBI 6 does not use KF8 INDX/CNCX records. Instead, navigation is done
    /// via an HTML page with `<a filepos="NNNNN">` links embedded in the text.
    /// The TOC page is placed at the beginning of the book content.
    fn build_html_toc(
        &self,
        toc: &[TocEntry],
        chapter_positions: &HashMap<String, u32>,
    ) -> String {
        if toc.is_empty() {
            return String::new();
        }

        fn resolve_toc_position(entry: &TocEntry, chapter_positions: &HashMap<String, u32>) -> Option<u32> {
            if let Some(ref target) = entry.target {
                match target {
                    AnchorTarget::Chapter(chapter_id) => {
                        let key = format!("ChapterId({})", chapter_id.0);
                        chapter_positions.get(&key).copied()
                    }
                    AnchorTarget::Internal(node_id) => {
                        let key = format!("ChapterId({})", node_id.chapter.0);
                        chapter_positions.get(&key).copied()
                    }
                    _ => None,
                }
            } else {
                None
            }
        }

        fn build_entries(
            entries: &[TocEntry],
            chapter_positions: &HashMap<String, u32>,
            depth: usize,
        ) -> String {
            let mut html = String::new();
            for entry in entries {
                let pos = resolve_toc_position(entry, chapter_positions).unwrap_or(0);
                let indent = "&nbsp;".repeat(depth * 4);
                html.push_str(&format!(
                    "{}<a filepos=\"{}\">{}</a><br/>\n",
                    indent, pos, entry.title
                ));
                if !entry.children.is_empty() {
                    html.push_str(&build_entries(&entry.children, chapter_positions, depth + 1));
                }
            }
            html
        }

        let mut toc_html = String::from("<html><body>\n");
        toc_html.push_str(&build_entries(toc, chapter_positions, 0));
        toc_html.push_str("</body></html>");
        toc_html
    }

    fn collect_html_content(&self, book: &mut Book) -> io::Result<(String, HashMap<String, u32>)> {
        let mut html = String::new();
        let mut chapter_positions = HashMap::new();

        // Get spine (reading order)
        let spine_entries: Vec<_> = book.spine().to_vec();

        for (i, entry) in spine_entries.iter().enumerate() {
            match book.load_raw(entry.id) {
                Ok(data) => {
                    // Convert bytes to string, ignoring encoding issues
                    let chapter_html = String::from_utf8_lossy(&data);

                    // Add page break between chapters (not before first)
                    if i > 0 {
                        html.push_str("<mbp:pagebreak/>");
                    }

                    // Record position AFTER pagebreak, at the anchor/content start
                    let position = html.len() as u32;

                    // Insert anchor tag at chapter start for TOC linking
                    let anchor = format!("<a id=\"filepos{}\" />", position);
                    html.push_str(&anchor);

                    // Store position keyed by ChapterId (for TOC resolution)
                    let chapter_id = format!("{:?}", entry.id);
                    chapter_positions.insert(chapter_id, position);

                    // Also store by source path and its variants (for link resolution)
                    if let Some(source_path) = book.source_id(entry.id) {
                        chapter_positions.insert(source_path.to_string(), position);

                        // Store filename only (e.g., "chapter1.xhtml" from "OEBPS/text/chapter1.xhtml")
                        if let Some(fname) = Path::new(source_path).file_name() {
                            chapter_positions.insert(fname.to_string_lossy().to_string(), position);
                        }

                        // Store relative path without top-level dir
                        // "OEBPS/text/chapter1.xhtml" → "text/chapter1.xhtml"
                        let parts: Vec<_> = source_path.split('/').collect();
                        if parts.len() > 1 {
                            let relative = parts[1..].join("/");
                            chapter_positions.insert(relative, position);
                        }
                    }

                    html.push_str(&chapter_html);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load chapter {:?}: {}", entry.id, e);
                }
            }
        }

        Ok((html, chapter_positions))
    }

    /// Resolve internal links (href) to MOBI filepos references.
    ///
    /// Converts hrefs in the body HTML to MOBI `<a filepos="NNNNN">` format:
    /// - `href="chapter.xhtml"` → `filepos="NNNNN"` (chapter links)
    /// - `href="chapter.xhtml#section"` → `filepos="NNNNN"` (anchor links)
    /// - `href="#fileposNNNNN"` → `filepos="NNNNN"` (existing MOBI anchors)
    /// - `href="content.html#fileposNNNNN"` → `filepos="NNNNN"` (MOBI roundtrip)
    fn resolve_internal_links(
        &self,
        html: &str,
        chapter_positions: &HashMap<String, u32>,
    ) -> String {
        let mut result = html.to_string();

        // Phase 1: Convert chapter hrefs to filepos
        // For each known chapter path, replace href="path" and href="path#frag" with filepos
        let mut replacements: Vec<(String, String)> = Vec::new();
        for (path_key, &position) in chapter_positions {
            if path_key.starts_with("ChapterId(") {
                continue;
            }
            let filepos_attr = format!("filepos=\"{}\"", position);
            // Exact match: href="path_key" or href='path_key'
            replacements.push((format!("href=\"{}\"", path_key), filepos_attr.clone()));
            replacements.push((format!("href='{}'", path_key), filepos_attr.clone()));
        }

        // Sort replacements by length descending so longer paths match first
        replacements.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (pattern, replacement) in &replacements {
            result = result.replace(pattern, replacement);
        }

        // Phase 2: Convert href with #filepos fragments using string search
        // to avoid corrupting multibyte UTF-8 characters.
        // Handles: href="#fileposNNNNN", href="content.html#fileposNNNNN",
        // and any href containing #filepos followed by digits.
        let mut output = String::with_capacity(result.len());
        let mut search_from = 0;

        while let Some(idx) = result[search_from..].find("href=\"") {
            let abs_idx = search_from + idx;
            // Copy everything before href="
            output.push_str(&result[search_from..abs_idx]);

            let after_quote = abs_idx + 6; // skip 'href="'
            // Find closing quote
            if let Some(close_rel) = result[after_quote..].find('"') {
                let close = after_quote + close_rel;
                let href_content = &result[after_quote..close];

                if let Some(hash_pos) = href_content.find("#filepos") {
                    let after_filepos = &href_content[hash_pos + 8..];
                    if after_filepos.chars().all(|c| c.is_ascii_digit())
                        && !after_filepos.is_empty()
                    {
                        output.push_str("filepos=\"");
                        output.push_str(after_filepos);
                        output.push('"');
                        search_from = close + 1;
                        continue;
                    }
                }
                // Not a filepos href - copy as-is
                output.push_str("href=\"");
                output.push_str(href_content);
                output.push('"');
                search_from = close + 1;
            } else {
                // No closing quote - copy as-is and move on
                output.push_str("href=\"");
                search_from = after_quote;
            }
        }
        output.push_str(&result[search_from..]);

        output
    }
}

impl Default for MobiExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of MOBI export with warnings.
#[derive(Debug, Clone)]
pub struct MobiResult {
    pub success: bool,
    pub warnings: Vec<String>,
}

/// Internal builder for MOBI 6 file creation
struct MobiBuilder {
    /// Compressed text records (4KB each)
    text_records: Vec<Vec<u8>>,
    /// Uncompressed text length (for MOBI header)
    text_length: u32,
    /// Image records (PNG/JPEG data)
    image_records: Vec<Vec<u8>>,
    /// Image path -> record index mapping
    image_path_to_record: HashMap<String, u32>,
    /// Cover image record index (1-based)
    cover_record_index: Option<u32>,
    /// Start reading position (byte offset of first chapter body)
    start_reading_offset: u32,
    /// Book metadata
    metadata: Metadata,
    /// Table of contents
    toc: Vec<TocEntry>,
    /// Chapter ID to byte position mapping (for TOC resolution)
    chapter_positions: HashMap<String, u32>,
    /// Flattened NCX entries for binary TOC index and TBS generation
    ncx_entries: Vec<NcxBuildEntry>,
    /// Collected warnings
    warnings: Vec<String>,
    /// Configuration
    config: MobiConfig,
}

impl MobiBuilder {
    /// Create a new builder from a Book
    fn new(book: &mut Book, config: MobiConfig) -> io::Result<Self> {
        // Collect metadata
        let metadata = book.metadata().clone();

        // Collect TOC
        let toc = book.toc().to_vec();

        Ok(Self {
            text_records: Vec::new(),
            text_length: 0,
            image_records: Vec::new(),
            image_path_to_record: HashMap::new(),
            cover_record_index: None,
            start_reading_offset: 0,
            metadata,
            toc,
            chapter_positions: HashMap::new(),
            ncx_entries: Vec::new(),
            warnings: Vec::new(),
            config,
        })
    }

    /// Build PalmDB header (78 bytes)
    fn build_palmdb_header(&self, num_records: u16) -> Vec<u8> {
        let mut header = Vec::new();

        // Database name (32 bytes) - use sanitized title
        let title = self.sanitize_title(&self.metadata.title);
        let name_bytes = title.as_bytes();
        header.extend_from_slice(name_bytes);
        header.resize(32, 0); // Pad to 32 bytes

        // Attributes (2 bytes)
        header.extend_from_slice(&0u16.to_be_bytes());

        // Version (2 bytes)
        header.extend_from_slice(&0u16.to_be_bytes());

        // Timestamps (4 bytes each) - use current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        header.extend_from_slice(&now.to_be_bytes()); // Creation
        header.extend_from_slice(&now.to_be_bytes()); // Modification
        header.extend_from_slice(&0u32.to_be_bytes()); // Last backup

        // Modification number (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // App info ID (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Sort info ID (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Database type (4 bytes) + Creator (4 bytes) = 8 bytes
        // Should be "BOOKMOBI" for MOBI files
        header.extend_from_slice(b"BOOKMOBI");

        // Unique ID seed (4 bytes)
        let seed = (2 * num_records) as u32 - 1;
        header.extend_from_slice(&seed.to_be_bytes());

        // Next record list (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Number of records (2 bytes)
        header.extend_from_slice(&num_records.to_be_bytes());

        header
    }

    /// Sanitize title for PalmDB header (max 31 chars, null-terminated)
    fn sanitize_title(&self, title: &str) -> String {
        let title: String = title.chars().take(31).collect();
        title
    }

    /// Process images from book assets
    fn process_images(&mut self, book: &mut Book) -> io::Result<()> {
        // Collect image paths first to avoid borrow checker issues
        let image_paths: Vec<_> = book.list_assets().to_vec();

        // Resolve cover image index using shared resolver
        let cover_image_idx = resolve_cover_asset(
            self.metadata.cover_image.as_deref(),
            &image_paths,
        );

        let mut image_iter_idx = 0usize;
        for image_path in image_paths {
            // Filter to only process actual image files
            let path_str = image_path.to_string_lossy();
            let is_image = path_str.ends_with(".jpg")
                || path_str.ends_with(".jpeg")
                || path_str.ends_with(".png")
                || path_str.ends_with(".gif")
                || path_str.ends_with(".svg")
                || path_str.ends_with(".webp")
                || path_str.ends_with(".bmp")
                || path_str.ends_with(".tiff")
                || path_str.ends_with(".tif");

            if !is_image {
                continue; // Skip non-image files
            }

            // Load image data using Book::load_asset()
            let image_data = match book.load_asset(&image_path) {
                Ok(data) => data,
                Err(e) => {
                    self.warnings
                        .push(format!("Failed to load image {:?}: {}", image_path, e));
                    image_iter_idx += 1;
                    continue; // Skip this image
                }
            };

            // For MOBI, preserve original image data without reprocessing
            // This ensures compatibility with readers and prevents quality loss
            if !image_data.is_empty() {
                // Store original image data
                // kindle:embed indices are 1-based (Calibre convention: reader does idx-1)
                let record_index = self.image_records.len() as u32 + 1;
                self.image_records.push(image_data);

                // Detect cover image by index from shared resolver
                if self.cover_record_index.is_none()
                    && cover_image_idx.is_some_and(|ci| ci == image_iter_idx)
                {
                    self.cover_record_index = Some(record_index);
                }

                // Store the relative path for HTML replacement
                // HTML uses paths like "images/image_0015.jpg"
                // We need to match what's actually in the HTML
                let relative_path = if let Some(parent) = image_path.parent() {
                    if let Some(dirname) = parent.file_name() {
                        if let Some(filename) = image_path.file_name() {
                            format!(
                                "{}/{}",
                                dirname.to_string_lossy(),
                                filename.to_string_lossy()
                            )
                        } else {
                            path_str.to_string()
                        }
                    } else {
                        path_str.to_string()
                    }
                } else {
                    path_str.to_string()
                };

                self.image_path_to_record
                    .insert(relative_path, record_index);
            }

            image_iter_idx += 1;
        }

        Ok(())
    }

    /// Build compressed text records from HTML content, with TBS appended.
    fn build_text_records(&mut self, html_content: &str) -> io::Result<()> {
        use crate::mobi::palmdoc;

        const RECORD_SIZE: usize = 4096;

        let mut offset = 0;
        let html_bytes = html_content.as_bytes();

        while offset < html_bytes.len() {
            let proposed_end = (offset + RECORD_SIZE).min(html_bytes.len());
            let end = if proposed_end < html_bytes.len() {
                find_utf8_boundary(html_bytes, proposed_end)
            } else {
                proposed_end
            };
            let chunk = &html_bytes[offset..end];

            // Compress this chunk
            let mut compressed = palmdoc::compress(chunk);

            // Append TBS (Trailing Byte Sequence) for chapter context
            if !self.ncx_entries.is_empty() {
                let tbs = build_tbs(
                    offset as u32,
                    end as u32,
                    &self.ncx_entries,
                );
                compressed.extend_from_slice(&tbs);
            }

            self.text_records.push(compressed);
            offset = end;
        }

        if self.text_records.is_empty() {
            self.text_records.push(Vec::new());
        }

        self.text_length = html_content.len() as u32;

        Ok(())
    }

    /// Build EXTH metadata header
    fn build_exth_header(&self) -> Vec<u8> {
        let mut exth = Vec::new();

        // EXTH signature
        exth.extend_from_slice(b"EXTH");

        // Header length and record count - will update later
        let length_offset = exth.len();
        exth.extend_from_slice(&0u32.to_be_bytes()); // placeholder
        let count_offset = exth.len();
        exth.extend_from_slice(&0u32.to_be_bytes()); // placeholder

        let mut record_count = 0u32;

        // Helper to add a record
        let mut add_record = |record_type: u32, data: &[u8]| {
            exth.extend_from_slice(&record_type.to_be_bytes());
            let len = (8 + data.len()) as u32;
            exth.extend_from_slice(&len.to_be_bytes());
            exth.extend_from_slice(data);
            record_count += 1;
        };

        // Record 100: Author
        if !self.metadata.authors.is_empty() {
            let author = self.metadata.authors.join("; ");
            add_record(100, author.as_bytes());
        }

        // Record 101: Publisher
        if let Some(ref publisher) = self.metadata.publisher {
            add_record(101, publisher.as_bytes());
        }

        // Record 103: Description
        if let Some(ref description) = self.metadata.description {
            add_record(103, description.as_bytes());
        }

        // Record 105: Subjects (keywords)
        if !self.metadata.subjects.is_empty() {
            let subjects = self.metadata.subjects.join("; ");
            add_record(105, subjects.as_bytes());
        }

        // Record 106: Published date
        if let Some(ref date) = self.metadata.date {
            add_record(106, date.as_bytes());
        }

        // Record 112: Source (identifier)
        if !self.metadata.identifier.is_empty() {
            add_record(112, self.metadata.identifier.as_bytes());
        }

        // Record 113: ASIN/UUID (for Kindle identification)
        add_record(113, self.metadata.identifier.as_bytes());

        // Record 116: Start reading offset
        if self.start_reading_offset > 0 {
            add_record(116, &self.start_reading_offset.to_be_bytes());
        }

        // Record 201: Cover offset (record index of cover image)
        if let Some(cover_idx) = self.cover_record_index {
            add_record(201, &cover_idx.to_be_bytes());
        }

        // Record 202: Thumbnail offset (same as cover for MOBI 6)
        if let Some(cover_idx) = self.cover_record_index {
            add_record(202, &cover_idx.to_be_bytes());
        }

        // Record 501: CDE type = "EBOK" (e-book)
        add_record(501, b"EBOK");

        // Record 503: Title (in EXTH for better compatibility)
        add_record(503, self.metadata.title.as_bytes());

        // Record 524: Language
        if !self.metadata.language.is_empty() {
            add_record(524, self.metadata.language.as_bytes());
        }

        // Update header length and record count
        // Pad to 4-byte boundary
        while exth.len() % 4 != 0 {
            exth.push(0);
        }
        let exth_len_padded = exth.len() as u32;
        let len_bytes = exth_len_padded.to_be_bytes();
        exth[length_offset..length_offset + 4].copy_from_slice(&len_bytes);

        let count_bytes = record_count.to_be_bytes();
        exth[count_offset..count_offset + 4].copy_from_slice(&count_bytes);

        exth
    }

    /// Build MOBI 6 header (Record 0 content, after PalmDB header).
    /// Pure MOBI 6 uses a 232-byte header with no KF8 extensions.
    /// NCX index goes into extra record index [2] (offset 48).
    /// TBS is signalled via extra_data_flags at offset 240-243.
    fn build_mobi_header(&self, text_length: u32, index_record_count: usize) -> Vec<u8> {
        let mut header = Vec::new();

        // Offset 0: Compression type (2 bytes) - PalmDoc = 2
        header.extend_from_slice(&2u16.to_be_bytes());

        // Offset 2: Unused (2 bytes)
        header.extend_from_slice(&0u16.to_be_bytes());

        // Offset 4: Text length (4 bytes)
        header.extend_from_slice(&text_length.to_be_bytes());

        // Offset 8: Text record count (2 bytes)
        let record_count = self.text_records.len() as u16;
        header.extend_from_slice(&record_count.to_be_bytes());

        // Offset 10: Text record size (2 bytes) - 4096
        header.extend_from_slice(&4096u16.to_be_bytes());

        // Offset 12: Encryption type (2 bytes) - None = 0
        header.extend_from_slice(&0u16.to_be_bytes());

        // Offset 14: Unused (2 bytes)
        header.extend_from_slice(&0u16.to_be_bytes());

        // Offset 16: Ident (4 bytes) - "MOBI"
        header.extend_from_slice(b"MOBI");

        // Offset 20: Header length (4 bytes) - 232 for pure MOBI 6
        header.extend_from_slice(&232u32.to_be_bytes());

        // Offset 24: Book type (4 bytes) - 2 = standard book
        header.extend_from_slice(&2u32.to_be_bytes());

        // Offset 28: Text encoding (4 bytes)
        let codepage: u32 = match self.config.encoding {
            MobiEncoding::Utf8 => 65001,
            MobiEncoding::Cp1252 => 1252,
        };
        header.extend_from_slice(&codepage.to_be_bytes());

        // Offset 32: UID (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Offset 36: File version (4 bytes) - 6 for MOBI 6
        header.extend_from_slice(&6u32.to_be_bytes());

        // Offset 40-79: Extra record indices (10 x 4 bytes)
        let ncx_index = if index_record_count > 0 {
            1u32 + self.text_records.len() as u32
        } else {
            0xFFFFFFFF
        };

        header.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // [0] meta orth
        header.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // [1] meta infl
        header.extend_from_slice(&ncx_index.to_be_bytes()); // [2] NCX index
        for _ in 0..7 {
            header.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // [3-9]
        }

        // Offset 80: First non-text record (4 bytes)
        let first_non_text = (self.text_records.len() as u32) + 1;
        header.extend_from_slice(&first_non_text.to_be_bytes());

        // Offset 84: Title offset (4 bytes) - will update in build_record0
        header.extend_from_slice(&0u32.to_be_bytes());

        // Offset 88: Title length (4 bytes)
        let title_len = self.metadata.title.len() as u32;
        header.extend_from_slice(&title_len.to_be_bytes());

        // Offset 92: Language code (4 bytes)
        let lang = language_code(&self.metadata.language);
        header.extend_from_slice(&lang.to_be_bytes());

        // Offset 96-103: Dictionary in/out languages (0)
        header.extend_from_slice(&0u64.to_be_bytes());

        // Offset 104: Min version (4 bytes)
        header.extend_from_slice(&6u32.to_be_bytes());

        // Offset 108: First resource record (4 bytes)
        let first_resource = if self.image_records.is_empty() {
            0xFFFFFFFF
        } else {
            (self.text_records.len() as u32) + 1 + index_record_count as u32
        };
        header.extend_from_slice(&first_resource.to_be_bytes());

        // Offset 112-127: Huff/CDIC compression (all 0)
        for _ in 0..4 {
            header.extend_from_slice(&0u32.to_be_bytes());
        }

        // Offset 128: EXTH flags (4 bytes) - 0x50 = EXTH present
        header.extend_from_slice(&0x50u32.to_be_bytes());

        // Offset 132-163: Unknown (32 bytes of 0)
        while header.len() < 164 {
            header.push(0);
        }

        // Offset 164-167: Unknown index (NULL)
        header.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());

        // Offset 168-183: DRM (4 x 4 bytes)
        header.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());
        header.extend_from_slice(&0u32.to_be_bytes());
        header.extend_from_slice(&0u32.to_be_bytes());
        header.extend_from_slice(&0u32.to_be_bytes());

        // Offset 184-191: Unknown (8 bytes)
        header.extend_from_slice(&0u64.to_be_bytes());

        // Offset 192-199: Unknown (8 bytes of 0)
        header.extend_from_slice(&0u64.to_be_bytes());

        // Offset 200-207: Unknown (8 bytes of 0)
        header.extend_from_slice(&0u64.to_be_bytes());

        // Offset 208-215: Unknown (8 bytes of 0)
        header.extend_from_slice(&0u64.to_be_bytes());

        // Offset 216-223: Unknown (8 bytes of 0)
        header.extend_from_slice(&0u64.to_be_bytes());

        // Offset 224-231: Unknown (8 bytes of 0)
        header.extend_from_slice(&0u64.to_be_bytes());

        // Total: 232 bytes
        assert_eq!(header.len(), 232, "MOBI 6 header must be exactly 232 bytes");
        header
    }

    /// Build complete Record 0 with MOBI header, EXTH, and title
    fn build_record0(&self, text_length: u32, index_record_count: usize) -> Vec<u8> {
        let mut record0 = self.build_mobi_header(text_length, index_record_count);

        // Add EXTH metadata header (after MOBI header at offset 232)
        let exth = self.build_exth_header();
        if !exth.is_empty() {
            record0.extend_from_slice(&exth);
        }

        // Add full title after EXTH
        let title_bytes = self.metadata.title.as_bytes();
        record0.extend_from_slice(title_bytes);

        // Update title offset field (at offset 84) to point to title
        let title_offset = record0.len() as u32 - title_bytes.len() as u32;
        let offset_bytes = title_offset.to_be_bytes();
        record0[84..88].copy_from_slice(&offset_bytes);

        record0
    }

    /// Build binary TOC index records (INDX + CNCX) for the Kindle TOC panel.
    fn build_toc_index(&self) -> Option<(Vec<Vec<u8>>, Vec<u8>)> {
        if self.ncx_entries.is_empty() {
            return None;
        }

        eprintln!("MOBI TOC: {} entries with positions", self.ncx_entries.len());
        Some(build_ncx_indx(&self.ncx_entries))
    }

    /// Build EOF marker
    fn build_eof_marker(&self) -> Vec<u8> {
        vec![0xE9, 0x8E, 0x0D, 0x0A]
    }

    /// Write the complete PDB file
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> io::Result<()> {
        // Build binary TOC index (INDX + CNCX) for Kindle TOC panel
        let toc_index = self.build_toc_index();
        let (indx_records, cncx_data) = match &toc_index {
            Some((records, cncx)) => (records.clone(), cncx.clone()),
            None => (Vec::new(), Vec::new()),
        };
        let index_record_count = indx_records.len() + if cncx_data.is_empty() { 0 } else { 1 };

        let eof_marker = self.build_eof_marker();

        // Record layout: Record 0 + text records + INDX/CNCX + image records + EOF
        let num_records = 1 + self.text_records.len() + index_record_count
            + self.image_records.len() + 1;

        let pdb_header = self.build_palmdb_header(num_records as u16);
        let record0 = self.build_record0(self.text_length, index_record_count);

        // Calculate record offsets
        let mut offsets = Vec::new();
        let mut offset = pdb_header.len() + 8 * num_records + 2; // + gap

        // Record 0
        offsets.push(offset);
        offset += record0.len();

        // Text records
        for record in &self.text_records {
            offsets.push(offset);
            offset += record.len();
        }

        // INDX records
        for record in &indx_records {
            offsets.push(offset);
            offset += record.len();
        }

        // CNCX record
        if !cncx_data.is_empty() {
            offsets.push(offset);
            offset += cncx_data.len();
        }

        // Image records
        for record in &self.image_records {
            offsets.push(offset);
            offset += record.len();
        }

        // EOF marker
        offsets.push(offset);

        // Write PalmDB header
        writer.write_all(&pdb_header)?;

        // Write record info list (8 bytes per record)
        for (i, &record_offset) in offsets.iter().enumerate() {
            writer.write_all(&(record_offset as u32).to_be_bytes())?;
            writer.write_all(&[0x00])?;
            let unique_id = (2 * i) as u32;
            writer.write_all(&unique_id.to_be_bytes()[1..4])?;
        }

        // Write gap
        writer.write_all(&[0x00, 0x00])?;

        // Write Record 0
        writer.write_all(&record0)?;

        // Write text records
        for record in &self.text_records {
            writer.write_all(record)?;
        }

        // Write INDX/CNCX records
        for record in &indx_records {
            writer.write_all(record)?;
        }
        if !cncx_data.is_empty() {
            writer.write_all(&cncx_data)?;
        }

        // Write image records
        for record in &self.image_records {
            writer.write_all(record)?;
        }

        // Write EOF marker
        writer.write_all(&eof_marker)?;

        Ok(())
    }
}

// Exporter trait implementation
impl Exporter for MobiExporter {
    fn export<W: Write + Seek>(&self, book: &mut Book, writer: &mut W) -> io::Result<()> {
        // Create builder
        let mut builder = MobiBuilder::new(book, self.config.clone())?;

        // Process images from book assets
        builder.process_images(book)?;

        // Get HTML content from book and track chapter positions
        let (html_content, chapter_positions) = self.collect_html_content(book)?;
        builder.chapter_positions = chapter_positions.clone();

        // Build HTML TOC and prepend to content (pure MOBI 6 navigation)
        let toc = builder.toc.clone();
        let html_content = if toc.is_empty() {
            html_content
        } else {
            let toc_html = self.build_html_toc(&toc, &chapter_positions);
            let toc_len = toc_html.len() as u32;

            // Shift all chapter positions by TOC length
            let mut shifted_positions = HashMap::new();
            for (key, &pos) in &chapter_positions {
                shifted_positions.insert(key.clone(), pos + toc_len);
            }
            builder.chapter_positions = shifted_positions;

            // Update start reading offset to skip TOC
            builder.start_reading_offset = toc_len;

            // Prepend TOC to body content
            let mut full_html = toc_html;
            full_html.push_str(&html_content);
            full_html
        };

        // Set start reading position if not already set by TOC
        if builder.start_reading_offset == 0
            && let Some(first_pos) = chapter_positions.values().min()
        {
            builder.start_reading_offset = *first_pos;
        }

        // Update image references in HTML to use MOBI record indices
        let html_content = self.update_image_references(&html_content, &builder);

        // Remove cover image from body HTML to avoid dual cover
        let html_content = self.strip_cover_from_body(&html_content, &builder);

        // Resolve internal links to MOBI filepos references
        let html_content = self.resolve_internal_links(&html_content, &builder.chapter_positions);

        // Flatten TOC into binary NCX entries (for INDX/CNCX records + TBS)
        if !builder.toc.is_empty() {
            builder.ncx_entries = flatten_toc_for_mobi(
                &builder.toc,
                &html_content,
                &builder.chapter_positions,
                html_content.len() as u32,
            );
        }

        // Build text records (with TBS appended for chapter context)
        builder.build_text_records(&html_content)?;

        // Write file
        builder.write(writer)?;

        // Emit warnings if any
        for warning in &builder.warnings {
            eprintln!("MOBI export warning: {}", warning);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobi_exporter_new() {
        let exporter = MobiExporter::new();
        // Should create successfully
        assert_eq!(exporter.config().max_image_size, (2048, 2048));
    }

    #[test]
    fn test_mobi_config_default() {
        let config = MobiConfig::default();
        assert_eq!(config.max_image_size, (2048, 2048));
        assert_eq!(config.max_image_file_size, 10 * 1024 * 1024);
        assert!(matches!(config.encoding, MobiEncoding::Utf8));
    }

    #[test]
    fn test_builder_creation() {
        // Verify the struct compiles and config works
        let config = MobiConfig::default();
        assert_eq!(config.max_image_size, (2048, 2048));
    }

    #[test]
    fn test_mobi_exporter_creation() {
        // Test that MobiExporter can be created
        let exporter = MobiExporter::new();
        assert!(matches!(exporter.config().encoding, MobiEncoding::Utf8));
    }
}
