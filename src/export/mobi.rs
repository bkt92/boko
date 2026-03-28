//! MOBI 6 exporter.
//!
//! Creates MOBI 6 (legacy Kindle format) files from Book structures.

use std::io::{self, Seek, Write};
use std::collections::HashMap;

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
        header.extend_from_slice(&now.to_be_bytes());  // Creation
        header.extend_from_slice(&now.to_be_bytes());  // Modification
        header.extend_from_slice(&0u32.to_be_bytes());  // Last backup

        // Modification number (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // App info ID (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Sort info ID (4 bytes)
        header.extend_from_slice(&0u32.to_be_bytes());

        // Database type (4 bytes)
        header.extend_from_slice(b"MOBI\0");

        // Creator (4 bytes)
        header.extend_from_slice(b"MOBI\0");

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
        let title: String = title.chars()
            .take(31)
            .collect();
        title
    }

    /// Process images from book assets
    fn process_images(&mut self, book: &mut Book) -> io::Result<()> {
        // Collect image paths first to avoid borrow checker issues
        let image_paths: Vec<_> = book.list_assets().to_vec();

        for image_path in image_paths {
            // Load image data using Book::load_asset()
            let image_data = match book.load_asset(&image_path) {
                Ok(data) => data,
                Err(e) => {
                    self.warnings.push(format!(
                        "Failed to load image {:?}: {}",
                        image_path, e
                    ));
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
                    self.warnings.push(format!(
                        "Failed to process image {:?}: {}",
                        image_path, e
                    ));
                }
            };
        }

        Ok(())
    }

    /// Build compressed text records from HTML content
    fn build_text_records(&mut self, html_content: &str) -> io::Result<()> {
        use crate::mobi::palmdoc;

        // Compress the HTML content
        let compressed = palmdoc::compress(html_content.as_bytes());

        // Split into 4KB records
        const RECORD_SIZE: usize = 4096;
        let mut offset = 0;

        while offset < compressed.len() {
            let end = (offset + RECORD_SIZE).min(compressed.len());
            let record_data = compressed[offset..end].to_vec();
            self.text_records.push(record_data);
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
        let first_image = if self.image_records.is_empty() {
            0xFFFFFFFF // NULL_INDEX
        } else {
            self.text_records.len() as u32
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
        header[header_length_offset..header_length_offset + 4]
            .copy_from_slice(&len_bytes);

        // Add title after header
        let title_bytes = self.metadata.title.as_bytes();
        header.extend_from_slice(title_bytes);

        header
    }

    /// Build NCX index from TOC entries
    fn build_ncx_index(&self) -> io::Result<Vec<u8>> {
        // For Phase 3, create simplified INDX record
        let mut indx = Vec::new();

        // INDX header
        indx.extend_from_slice(b"INDX");
        indx.extend_from_slice(&0xC0u32.to_be_bytes()); // Header length = 192
        indx.extend_from_slice(&0u32.to_be_bytes());  // Unknown
        indx.resize(192, 0); // Pad to 192 bytes

        // For MOBI 6, we'll use a simple flat index
        // TODO: Full implementation in later phase
        // This is a placeholder that creates a minimal valid INDX

        Ok(indx)
    }

    /// Write the complete PDB file
    fn write<W: Write + Seek>(&self, _writer: &mut W) -> io::Result<()> {
        // TODO: Implement in subsequent tasks
        Ok(())
    }
}

// Exporter trait will be implemented in a later task
impl Exporter for MobiExporter {
    fn export<W: Write + Seek>(&self, _book: &mut Book, _writer: &mut W) -> io::Result<()> {
        // TODO: Implement in Phase 3
        // For now, return unsupported to prevent crashes
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "MOBI 6 export not yet implemented",
        ))
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
        // This test will need a Book, which we'll mock for now
        // Full implementation in subsequent tasks
        // For now, just verify the struct compiles
        let config = MobiConfig::default();
        assert_eq!(config.max_image_size, (2048, 2048));
    }
}
