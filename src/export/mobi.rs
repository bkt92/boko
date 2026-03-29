//! MOBI 6 exporter.
//!
//! Creates MOBI 6 (legacy Kindle format) files from Book structures.

use std::collections::HashMap;
use std::io::{self, Seek, Write};

use crate::model::{Book, Metadata, TocEntry};

use super::Exporter;

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

    /// Collect HTML content from book chapters
    /// Update image references in HTML to use MOBI record indices
    fn update_image_references(&self, html: &str, builder: &MobiBuilder) -> String {
        let mut result = html.to_string();

        // Calculate first image record index
        // Images come after: Record 0 + text records + INDX/CNCX records (3)
        let first_image_record = (builder.text_records.len() + 1 + 3) as u32;

        // Update each image reference
        for (image_path, image_index) in &builder.image_path_to_record {
            let mobi_record_index = first_image_record + image_index;
            let mobi_img_tag = format!(r#"<img recindex="{}"/>"#, mobi_record_index);

            // Get filename from path (handle both forward and backslash)
            let filename = match image_path.rsplit('\\').next() {
                Some(name) if name.contains('/') => {
                    name.rsplit('/').next().unwrap_or(name)
                }
                Some(name) => name,
                None => image_path,
            };

            // Try multiple image reference patterns
            let patterns = vec![
                format!(r#"src="{}""#, image_path.replace('\\', "/")),
                format!(r#"src='{}'"#, image_path.replace('\\', "/")),
                format!(r#"src="../images/{}""#, filename),
                format!(r#"src='../images/{}'"#, filename),
                format!(r#"src="images/{}""#, filename),
                format!(r#"src='images/{}'"#, filename),
                format!(r#"src="{}""#, filename),
                format!(r#"src='{}'"#, filename),
            ];

            for pattern in patterns {
                result = result.replace(&pattern, &mobi_img_tag);
            }
        }

        // Log the number of images processed
        if !builder.image_path_to_record.is_empty() {
            eprintln!(
                "MOBI: Processed {} images with recindex format (starting at record {})",
                builder.image_path_to_record.len(),
                first_image_record
            );
        }

        result
    }

    fn collect_html_content(&self, book: &mut Book) -> io::Result<String> {
        let mut html = String::new();

        // Simple HTML wrapper
        html.push_str("<html><body>");

        // Get spine (reading order) - collect entries to avoid borrow checker
        let spine_entries: Vec<_> = book.spine().to_vec();

        for entry in spine_entries {
            match book.load_raw(entry.id) {
                Ok(data) => {
                    // Convert bytes to string, ignoring encoding issues
                    let chapter_html = String::from_utf8_lossy(&data);
                    html.push_str(&chapter_html);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load chapter {:?}: {}", entry.id, e);
                }
            }
        }

        html.push_str("</body></html>");

        Ok(html)
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
    /// Image records (PNG/JPEG data)
    image_records: Vec<Vec<u8>>,
    /// Image path -> record index mapping
    image_path_to_record: HashMap<String, u32>,
    /// Book metadata
    metadata: Metadata,
    /// Table of contents
    #[allow(dead_code)] // TODO: Use when build_ncx_index is fully implemented
    toc: Vec<TocEntry>,
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
            image_records: Vec::new(),
            image_path_to_record: HashMap::new(),
            metadata,
            toc,
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
                    continue; // Skip this image
                }
            };

            // Process with shared image module
            use crate::image::convert::{ImageConfig, ImageFormat, process_image};

            let config = ImageConfig {
                max_dimensions: self.config.max_image_size,
                max_file_size: self.config.max_image_file_size,
                output_format: ImageFormat::Auto,
                jpeg_quality: 85,
                png_compression: 6,
            };

            match process_image(&image_data, &config) {
                Ok((data, warn)) => {
                    self.warnings.extend(warn);
                    if !data.is_empty() {
                        // Store processed image
                        let record_index = self.image_records.len() as u32;
                        self.image_records.push(data);

                        // Map path -> record index for HTML filtering
                        let path_str = image_path.to_string_lossy().to_string();
                        self.image_path_to_record.insert(path_str, record_index);
                    }
                }
                Err(e) => {
                    self.warnings
                        .push(format!("Failed to process image {:?}: {}", image_path, e));
                }
            };
        }

        Ok(())
    }

    /// Build compressed text records from HTML content
    fn build_text_records(&mut self, html_content: &str) -> io::Result<()> {
        use crate::mobi::palmdoc;

        // Split HTML into chunks, then compress each chunk independently
        // This ensures PalmDoc back-references don't span record boundaries
        const RECORD_SIZE: usize = 4096;

        // Split HTML into chunks and compress each one
        let mut offset = 0;
        let html_bytes = html_content.as_bytes();

        while offset < html_bytes.len() {
            let end = (offset + RECORD_SIZE).min(html_bytes.len());
            let chunk = &html_bytes[offset..end];

            // Compress this chunk
            let compressed = palmdoc::compress(chunk);
            self.text_records.push(compressed);

            offset = end;
        }

        // Add at least one record even if empty
        if self.text_records.is_empty() {
            self.text_records.push(Vec::new());
        }

        Ok(())
    }

    /// Build MOBI 6 header (Record 0 content, after PalmDB header)
    fn build_mobi_header(&self, text_length: u32) -> Vec<u8> {
        let mut header = Vec::new();

        // Compression type (2 bytes) - PalmDoc = 2
        header.extend_from_slice(&2u16.to_be_bytes());

        // Reserved (2 bytes)
        header.extend_from_slice(&0u16.to_be_bytes());

        // Text length (4 bytes)
        header.extend_from_slice(&text_length.to_be_bytes());

        // Text record count (2 bytes)
        let record_count = self.text_records.len() as u16;
        header.extend_from_slice(&record_count.to_be_bytes());

        // Text record size (2 bytes) - 4096
        header.extend_from_slice(&4096u16.to_be_bytes());

        // Encryption (2 bytes) - None = 0
        header.extend_from_slice(&0u16.to_be_bytes());

        // Reserved (2 bytes)
        header.extend_from_slice(&0u16.to_be_bytes());

        // Unknown (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // MOBI header length (4 bytes) - will update at end
        let header_length_offset = header.len();
        header.extend_from_slice(&0u32.to_be_bytes());

        // MOBI type (4 bytes) - 2 = standard book
        header.extend_from_slice(&2u32.to_be_bytes());

        // Codepage (4 bytes) - 65001 = UTF-8
        let codepage: u32 = match self.config.encoding {
            MobiEncoding::Utf8 => 65001,
            MobiEncoding::Cp1252 => 1252,
        };
        header.extend_from_slice(&codepage.to_be_bytes());

        // Unknown (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Skip to title offset (0x54)
        while header.len() < 0x54 {
            header.push(0);
        }

        // Title offset (4 bytes)
        let title_offset = 232u32; // After header
        header.extend_from_slice(&title_offset.to_be_bytes());

        // Title length (4 bytes)
        let title_len = self.metadata.title.len() as u32;
        header.extend_from_slice(&title_len.to_be_bytes());

        // Language (4 bytes) - 0x09 = English
        header.extend_from_slice(&0x09u32.to_be_bytes());

        // Unknown (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Skip to MOBI version (0x68)
        while header.len() < 0x68 {
            header.push(0);
        }

        // MOBI version (4 bytes) - 6 for MOBI 6
        header.extend_from_slice(&6u32.to_be_bytes());

        // First image index (4 bytes)
        // Images come after: Record 0 + text records + INDX/CNCX records (3)
        let first_image = if self.image_records.is_empty() {
            0xFFFFFFFF // NULL_INDEX
        } else {
            // Record 0 (MOBI header) + text records + INDX/CNCX (3 records)
            (self.text_records.len() as u32) + 1 + 3
        };
        header.extend_from_slice(&first_image.to_be_bytes());

        // Unknown (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Skip to EXTH flags (0x80)
        while header.len() < 0x80 {
            header.push(0);
        }

        // EXTH flags (4 bytes) - 0x40 = EXTH present
        header.extend_from_slice(&0x40u32.to_be_bytes());

        // Update header length at stored offset
        let header_len = header.len() as u32;
        let len_bytes = header_len.to_be_bytes();
        header[header_length_offset..header_length_offset + 4].copy_from_slice(&len_bytes);

        // Add title after header
        let title_bytes = self.metadata.title.as_bytes();
        header.extend_from_slice(title_bytes);

        header
    }

    /// Build INDX header record (Record 117 in reference file)
    fn build_indx_header(&self) -> Vec<u8> {
        let mut indx = Vec::new();

        // Magic (4 bytes)
        indx.extend_from_slice(b"INDX");

        // Header length (4 bytes) - 192 bytes
        indx.extend_from_slice(&192u32.to_be_bytes());

        // Header type (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 2
        indx.extend_from_slice(&2u32.to_be_bytes());

        // IDXT start (4 bytes) - offset to index entries (232)
        indx.extend_from_slice(&232u32.to_be_bytes());

        // Entry count (4 bytes) - number of entries in INDX (1)
        indx.extend_from_slice(&1u32.to_be_bytes());

        // Encoding (4 bytes) - UTF-8 (65001)
        indx.extend_from_slice(&65001u32.to_be_bytes());

        // Unknown (4 bytes) - 0xFFFFFFFF
        indx.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());

        // Total entries (4 bytes) - total number of TOC entries (12)
        indx.extend_from_slice(&(self.toc.len() as u32).to_be_bytes());

        // ORDT offset (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // LIGT offset (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Num LIGT (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Num CNCX (4 bytes) - 1 (one CNCX record)
        indx.extend_from_slice(&1u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Padding to offset 180 (TAGX offset)
        while indx.len() < 180 {
            indx.push(0);
        }

        // TAGX offset (4 bytes) - 192 (right after header)
        indx.extend_from_slice(&192u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Should be at 192 bytes now
        assert_eq!(indx.len(), 192);

        // TAGX section (starts at offset 192)
        indx.extend_from_slice(b"TAGX");

        // First entry offset (4 bytes) - 32
        indx.extend_from_slice(&32u32.to_be_bytes());

        // Control byte count (4 bytes) - 1
        indx.extend_from_slice(&1u32.to_be_bytes());

        // TAGX entries (4 bytes each)
        // Tag 1: Name offset, 1 value, bitmask 0x01
        indx.extend_from_slice(&[1, 1, 1, 0]);
        // Tag 2: Offset, 1 value, bitmask 0x02
        indx.extend_from_slice(&[2, 1, 2, 1]);
        // Tag 3: Level, 1 value, bitmask 0x04
        indx.extend_from_slice(&[3, 1, 4, 1]);
        // Tag 4: Parent offset, 1 value, bitmask 0x08
        indx.extend_from_slice(&[4, 1, 8, 1]);
        // Tag 5: First child offset, 1 value, bitmask 0x10
        indx.extend_from_slice(&[5, 1, 16, 1]);
        // Tag 6: Last child offset, 1 value, bitmask 0x20
        indx.extend_from_slice(&[6, 1, 32, 1]);

        // IDXT section (starts at offset 232)
        // Index entry 1 (placeholder - points to record 118)
        indx.extend_from_slice(&1u32.to_be_bytes()); // Record 118

        indx
    }

    /// Build INDX entries record (Record 118 in reference file)
    fn build_indx_entries(&self) -> Vec<u8> {
        let mut indx = Vec::new();

        // Magic (4 bytes)
        indx.extend_from_slice(b"INDX");

        // Header length (4 bytes) - 192 bytes
        indx.extend_from_slice(&192u32.to_be_bytes());

        // Header type (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 1
        indx.extend_from_slice(&1u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // IDXT start (4 bytes) - 340 (offset to actual entries)
        indx.extend_from_slice(&340u32.to_be_bytes());

        // Entry count (4 bytes) - number of TOC entries
        indx.extend_from_slice(&(self.toc.len() as u32).to_be_bytes());

        // Encoding (4 bytes) - 0xFFFFFFFF
        indx.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());

        // Unknown (4 bytes) - 0xFFFFFFFF
        indx.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());

        // Unknown (4 bytes) - 0
        indx.extend_from_slice(&0u32.to_be_bytes());

        // Padding to 192 bytes
        while indx.len() < 192 {
            indx.push(0);
        }

        // IDXT section (starts at offset 340)
        // Build index entries from TOC
        for (i, entry) in self.toc.iter().enumerate() {
            // Control byte (1 byte) - all tags present
            indx.push(0x3F); // 0b00111111 = tags 1-6 present

            // Tag 1: Name offset (variable-width int)
            // For now, use entry index as placeholder
            let name_offset = (i * 4) as u32;
            indx.extend_from_slice(&name_offset.to_be_bytes());

            // Tag 2: File position (4 bytes) - placeholder
            indx.extend_from_slice(&0u32.to_be_bytes());

            // Tag 3: Level (1 byte) - use placeholder (TocEntry doesn't have level field)
            indx.push(0u8);

            // Tag 4: Parent offset (1 byte) - placeholder
            indx.push(0xFF);

            // Tag 5: First child offset (1 byte) - placeholder
            indx.push(0xFF);

            // Tag 6: Last child offset (1 byte) - placeholder
            indx.push(0xFF);
        }

        indx
    }

    /// Build CNCX strings record (Record 119 in reference file)
    fn build_cncx_strings(&self) -> Vec<u8> {
        let mut cncx = Vec::new();

        // Collect all TOC entry titles
        for entry in &self.toc {
            cncx.extend_from_slice(entry.title.as_bytes());
            cncx.push(0); // Null terminator
        }

        cncx
    }

    /// Build FLIS record (Record 142 in reference file)
    fn build_flis_record(&self) -> Vec<u8> {
        let mut flis = Vec::new();

        // Magic (4 bytes)
        flis.extend_from_slice(b"FLIS");

        // Header length (4 bytes) - 8
        flis.extend_from_slice(&8u32.to_be_bytes());

        // Unknown (1 byte) - 0x41
        flis.push(0x41);

        // Unknown (3 bytes) - 0
        flis.extend_from_slice(&[0, 0, 0]);

        // Unknown (4 bytes) - 0xFFFFFFFF
        flis.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000001
        flis.extend_from_slice(&1u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000003
        flis.extend_from_slice(&3u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000003
        flis.extend_from_slice(&3u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000001
        flis.extend_from_slice(&1u32.to_be_bytes());

        // Unknown (4 bytes) - 0xFFFFFFFF
        flis.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());

        flis
    }

    /// Build FCIS record (Record 143 in reference file)
    fn build_fcis_record(&self) -> Vec<u8> {
        let mut fcis = Vec::new();

        // Magic (4 bytes)
        fcis.extend_from_slice(b"FCIS");

        // Header length (4 bytes) - 20 (0x14)
        fcis.extend_from_slice(&20u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000010
        fcis.extend_from_slice(&16u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000001
        fcis.extend_from_slice(&1u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        fcis.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000007
        fcis.extend_from_slice(&7u32.to_be_bytes());

        // Unknown (4 bytes) - 0x396F0000
        fcis.extend_from_slice(&0x396F0000u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        fcis.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0
        fcis.extend_from_slice(&0u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000020
        fcis.extend_from_slice(&32u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000008
        fcis.extend_from_slice(&8u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000001
        fcis.extend_from_slice(&1u32.to_be_bytes());

        // Unknown (4 bytes) - 0x00000001
        fcis.extend_from_slice(&1u32.to_be_bytes());

        fcis
    }

    /// Build EOF marker (Record 144 in reference file)
    fn build_eof_marker(&self) -> Vec<u8> {
        vec![0xE9, 0x8E, 0x0D, 0x0A]
    }

    /// Write the complete PDB file
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> io::Result<()> {
        // Build navigation/index records
        let indx_header = self.build_indx_header();
        let indx_entries = self.build_indx_entries();
        let cncx_strings = self.build_cncx_strings();
        let flis_record = self.build_flis_record();
        let fcis_record = self.build_fcis_record();
        let eof_marker = self.build_eof_marker();

        // Calculate total number of records
        // Record 0 (headers) + text records + INDX records (3) + image records + FLIS/FCIS/EOF (3)
        let num_records = 1 + self.text_records.len() + 3 + self.image_records.len() + 3;

        // Build PalmDB header
        let pdb_header = self.build_palmdb_header(num_records as u16);

        // Build MOBI header (Record 0 content)
        let text_length: u32 = self.text_records.iter().map(|r| r.len()).sum::<usize>() as u32; // Uncompressed length estimate
        let mobi_header = self.build_mobi_header(text_length);

        // Calculate record offsets
        let mut offsets = Vec::new();
        let mut offset = pdb_header.len() + 8 * num_records + 2; // + gap

        // Record 0 offset
        offsets.push(offset);
        offset += mobi_header.len();

        // Text record offsets
        for record in &self.text_records {
            offsets.push(offset);
            offset += record.len();
        }

        // INDX/CNCX record offsets (records 117-119)
        offsets.push(offset);
        offset += indx_header.len();

        offsets.push(offset);
        offset += indx_entries.len();

        offsets.push(offset);
        offset += cncx_strings.len();

        // Image record offsets
        for record in &self.image_records {
            offsets.push(offset);
            offset += record.len();
        }

        // FLIS/FCIS/EOF record offsets (last 3 records)
        offsets.push(offset);
        offset += flis_record.len();

        offsets.push(offset);
        offset += fcis_record.len();

        offsets.push(offset);
        offset += eof_marker.len();

        // Write PalmDB header
        writer.write_all(&pdb_header)?;

        // Write record info list (8 bytes per record)
        for (i, &record_offset) in offsets.iter().enumerate() {
            // Offset (4 bytes) - must be u32 for PalmDB format
            writer.write_all(&(record_offset as u32).to_be_bytes())?;

            // Attributes (1 byte)
            writer.write_all(&[0x00])?;

            // Unique ID (3 bytes) - 2*i
            let unique_id = (2 * i) as u32;
            writer.write_all(&unique_id.to_be_bytes()[1..4])?;
        }

        // Write gap (2 bytes)
        writer.write_all(&[0x00, 0x00])?;

        // Write Record 0 (MOBI header + title)
        writer.write_all(&mobi_header)?;

        // Write text records
        for record in &self.text_records {
            writer.write_all(record)?;
        }

        // Write INDX/CNCX records
        writer.write_all(&indx_header)?;
        writer.write_all(&indx_entries)?;
        writer.write_all(&cncx_strings)?;

        // Write image records
        for record in &self.image_records {
            writer.write_all(record)?;
        }

        // Write FLIS/FCIS/EOF records
        writer.write_all(&flis_record)?;
        writer.write_all(&fcis_record)?;
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

        // Get HTML content from book
        let html_content = self.collect_html_content(book)?;

        // Update image references in HTML to use MOBI record indices
        let html_content = self.update_image_references(&html_content, &builder);

        // Build text records
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
