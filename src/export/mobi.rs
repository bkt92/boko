//! MOBI 6 exporter.
//!
//! Creates MOBI 6 (legacy Kindle format) files from Book structures.

use std::io::{self, Seek, Write};

use crate::model::Book;

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
}
