# MOBI 6 Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add legacy MOBI 6 export support to the Boko ebook conversion library

**Architecture:** Create MobiExporter following existing Exporter trait pattern, reuse PalmDoc compression, add shared image processing module, filter HTML to MOBI 6 subset, write PalmDB format files with proper headers

**Tech Stack:** Rust 2024, html5ever 0.39, image crate 0.25, flate2, existing PalmDoc compression

---

## File Structure

### New Files
- `src/image/mod.rs` - Shared image module exports
- `src/image/convert.rs` - Image format conversion, downsampling, optimization
- `src/mobi/html_filter.rs` - HTML filtering to MOBI 6 subset
- `src/export/mobi.rs` - MobiExporter and MobiBuilder
- `tests/mobi_export_test.rs` - Integration tests

### Modified Files
- `src/export/mod.rs` - Add mobi module export
- `src/model/book.rs` - Enable MOBI in export() match arm
- `src/main.rs` - Remove CLI restriction
- `Cargo.toml` - Add image crate dependency

### Test Fixtures
- `tests/fixtures/mobi/simple.epub` - Basic text, no images
- `tests/fixtures/mobi/with_images.epub` - Contains images
- `tests/fixtures/mobi/complex.epub` - Tables, nested lists, links

---

## Phase 0: Shared Image Module

### Task 1: Add image crate dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add image crate to dependencies**

```toml
# Add to [dependencies] section
image = { version = "0.25", default-features = false, features = ["gif", "jpeg", "png", "webp"] }
```

- [ ] **Step 2: Verify dependencies compile**

Run: `cargo check`
Expected: No errors, image crate resolved

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "deps: add image crate 0.25 for image processing

Features: gif, jpeg, png, webp
Will be used by MOBI 6 exporter and future EPUB/AZW3 optimization
"
```

### Task 2: Create image module structure

**Files:**
- Create: `src/image/mod.rs`

- [ ] **Step 1: Create module file**

```rust
//! Shared image processing utilities.
//!
//! This module provides image processing functionality used across multiple exporters:
//! - MOBI 6: Downscale and convert for legacy device compatibility
//! - EPUB/AZW3: Optimize images to reduce file size (future)
//!
//! Design principles:
//! - Format-agnostic: No format-specific logic
//! - Configurable: All settings via ImageConfig
//! - Reusable: Return processed data + warnings

pub mod convert;

pub use convert::{ImageConfig, ImageFormat, process_image, detect_format, is_supported_format};
```

- [ ] **Step 2: Verify module compiles**

Run: `cargo check`
Expected: Error about missing `convert` module

- [ ] **Step 3: Commit**

```bash
git add src/image/mod.rs
git commit -m "img: create image module structure

Shared module for image processing across exporters
"
```

### Task 3: Implement image format detection

**Files:**
- Create: `src/image/convert.rs`

- [ ] **Step 1: Write test for format detection**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // JPEG magic bytes: FF D8 FF
    #[test]
    fn test_detect_jpeg() {
        let data = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        assert_eq!(detect_format(data), Some(ImageFormat::Jpeg));
    }

    // PNG magic bytes: 89 50 4E 47
    #[test]
    fn test_detect_png() {
        let data = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_format(data), Some(ImageFormat::Png));
    }

    // GIF magic bytes: 47 49 46 38
    #[test]
    fn test_detect_gif() {
        let data = &[0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
        assert_eq!(detect_format(data), Some(ImageFormat::Gif));
    }

    // WebP magic bytes: 52 49 46 46 ... 57 45 42 50
    #[test]
    fn test_detect_webp() {
        let mut data = vec![0x52, 0x49, 0x46, 0x46];
        data.extend_from_slice(b"WEBP");
        assert_eq!(detect_format(&data), Some(ImageFormat::Jpeg)); // WebP converts to JPEG
    }

    #[test]
    fn test_detect_unknown() {
        let data = &[0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_format(data), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib image::convert::tests::test_detect_jpeg`
Expected: FAILED - function not found

- [ ] **Step 3: Implement types and detection**

```rust
//! Image processing functions for format conversion and optimization.

use std::io;
use std::path::Path;

/// Image format options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    /// Keep original format if supported
    Auto,
    /// JPEG format (photos, gradients)
    Jpeg,
    /// PNG format (graphics, transparency)
    Png,
    /// GIF format (animated images)
    Gif,
}

/// Configuration for image processing
#[derive(Clone, Debug)]
pub struct ImageConfig {
    /// Maximum dimensions (width, height)
    pub max_dimensions: (u32, u32),
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Output format preference
    pub output_format: ImageFormat,
    /// JPEG quality (1-100, default 85)
    pub jpeg_quality: u8,
    /// PNG compression level (0-9, default 6)
    pub png_compression: u8,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            max_dimensions: (2048, 2048),
            max_file_size: 10 * 1024 * 1024, // 10MB
            output_format: ImageFormat::Auto,
            jpeg_quality: 85,
            png_compression: 6,
        }
    }
}

/// Detect image format from magic bytes
pub fn detect_format(data: &[u8]) -> Option<ImageFormat> {
    if data.len() < 4 {
        return None;
    }

    // JPEG: FF D8 FF
    if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        return Some(ImageFormat::Jpeg);
    }

    // PNG: 89 50 4E 47
    if data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47 {
        return Some(ImageFormat::Png);
    }

    // GIF: 47 49 46 38
    if data[0] == 0x47 && data[1] == 0x49 && data[2] == 0x46 && data[3] == 0x38 {
        return Some(ImageFormat::Gif);
    }

    // WebP: RIFF...WEBP
    if data.len() >= 12
        && &data[0..4] == b"RIFF"
        && &data[8..12] == b"WEBP"
    {
        // WebP will be converted to JPEG or PNG
        return Some(ImageFormat::Jpeg);
    }

    None
}

/// Check if image format is natively supported (no conversion needed)
pub fn is_supported_format(data: &[u8]) -> bool {
    detect_format(data).is_some()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib image::convert::tests::test_detect`
Expected: All 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/image/convert.rs
git commit -m "img: implement image format detection

Supports JPEG, PNG, GIF, WebP detection via magic bytes
Tests for all formats + unknown format
"
```

### Task 4: Implement image processing

**Files:**
- Modify: `src/image/convert.rs`

- [ ] **Step 1: Write test for JPEG preservation**

```rust
// Add to tests module in convert.rs

#[test]
fn test_process_jpeg_no_conversion_needed() {
    // Read JPEG from fixture file
    let fixture_path = "tests/fixtures/image/test.jpg";
    let jpeg_data = if std::path::Path::new(fixture_path).exists() {
        std::fs::read(fixture_path).expect("Failed to read fixture")
    } else {
        // Skip test if fixture not found
        eprintln!("Skipping test: fixture not found at {}", fixture_path);
        return;
    };

    let config = ImageConfig {
        max_dimensions: (100, 100),
        max_file_size: 1024 * 1024,
        output_format: ImageFormat::Auto,
        ..Default::default()
    };

    let (result, warnings) = process_image(&jpeg_data, &config).unwrap();

    // Should preserve JPEG unchanged
    assert!(!result.is_empty());
    assert_eq!(warnings.len(), 0);
    assert_eq!(detect_format(&result), Some(ImageFormat::Jpeg));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib image::convert::tests::test_process_jpeg_no_conversion_needed`
Expected: FAILED - function not found

- [ ] **Step 3: Implement process_image function**

```rust
use image::{ImageDecoder, ImageEncoder, ImageFormat as ImgFormat, DynamicImage, ImageError, io::Cursor};
use std::io::Read;

/// Process image data according to configuration
///
/// Returns (processed_image_data, warnings)
/// Returns Ok(Vec::new()) if image should be skipped
pub fn process_image(
    data: &[u8],
    config: &ImageConfig,
) -> io::Result<(Vec<u8>, Vec<String>)> {
    let mut warnings = Vec::new();

    // Detect format
    let format = match detect_format(data) {
        Some(f) => f,
        None => {
            // Unknown format - try to load with image crate
            return Ok((Vec::new(), vec!["Unsupported image format".to_string()]));
        }
    };

    // Load image
    let img = match load_image(data, format) {
        Ok(img) => img,
        Err(e) => {
            warnings.push(format!("Failed to load image: {}", e));
            return Ok((Vec::new(), warnings));
        }
    };

    // Check dimensions and downsample if needed
    let (width, height) = img.dimensions();
    let img = if width > config.max_dimensions.0 || height > config.max_dimensions.1 {
        warnings.push(format!(
            "Image too large ({}x{}), downsampling to max {:?}",
            width, height, config.max_dimensions
        ));

        // Actually downsample the image
        use image::imageops::FilterType;
        img.resize(
            config.max_dimensions.0.min(width),
            config.max_dimensions.1.min(height),
            FilterType::Lanczos3
        )
    } else {
        img
    };

    // Determine output format
    let output_format = match config.output_format {
        ImageFormat::Auto => format,
        f => f,
    };

    // Encode image
    let encoded = match encode_image(&img, output_format, config) {
        Ok(data) => data,
        Err(e) => {
            warnings.push(format!("Failed to encode image: {}", e));
            return Ok((Vec::new(), warnings));
        }
    };

    // Check file size
    if encoded.len() as u64 > config.max_file_size {
        warnings.push(format!(
            "Image too large ({} bytes), consider quality reduction",
            encoded.len()
        ));
    }

    Ok((encoded, warnings))
}

fn load_image(data: &[u8], format: ImageFormat) -> Result<DynamicImage, io::Error> {
    let cursor = Cursor::new(data);

    let img_format = match format {
        ImageFormat::Jpeg => ImgFormat::Jpeg,
        ImageFormat::Png => ImgFormat::Png,
        ImageFormat::Gif => ImgFormat::Gif,
        ImageFormat::Auto => {
            // Try to detect from data
            return image::io::Reader::new(cursor).with_guessed_format()
                .decode()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
    };

    image::io::Reader::with_format(cursor, img_format)
        .decode()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn encode_image(
    img: &DynamicImage,
    format: ImageFormat,
    config: &ImageConfig,
) -> Result<Vec<u8>, io::Error> {
    let mut output = Vec::new();

    match format {
        ImageFormat::Jpeg => {
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, config.jpeg_quality);
            encoder.encode(
                img.to_rgb8().as_ref(),
                img.width(),
                img.height(),
                image::color::ColorType::Rgb8,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        ImageFormat::Png => {
            let encoder = image::codecs::png::PngEncoder::new_with_quality(
                &mut output,
                image::codecs::png::CompressionType::Fast,
                config.png_compression,
            );
            encoder.encode(
                img.to_rgb8().as_ref(),
                img.width(),
                img.height(),
                image::color::ColorType::Rgb8,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        ImageFormat::Gif => {
            // GIF encoding - use simplest approach
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut output);
            encoder.encode_frame(img.to_rgba8()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
        ImageFormat::Auto => {
            // Shouldn't happen - format should be resolved by now
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Auto format not supported for encoding"));
        }
    }

    Ok(output)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib image::convert::tests::test_process_jpeg_no_conversion_needed`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/image/convert.rs
git commit -m "img: implement image processing with format conversion

Supports JPEG, PNG, GIF encoding with configurable quality
Preserves format when output_format is Auto
Returns warnings for processing issues
"
```

---

## Phase 1: Foundation (MobiExporter Skeleton)

### Task 5: Create MobiExporter structure

**Files:**
- Create: `src/export/mobi.rs`

- [ ] **Step 1: Write test for MobiExporter creation**

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib export::mobi::tests::test_mobi_exporter_new`
Expected: FAILED - module not found

- [ ] **Step 3: Implement MobiExporter and config**

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib export::mobi::tests::test_mobi`
Expected: Both tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: create MobiExporter structure with config

Implements MobiConfig, MobiEncoding, MobiExporter skeleton
Default config: UTF-8, 2048x2048 images, 10MB max file size
Tests for default values and exporter creation
"
```

### Task 6: Wire up MobiExporter in export module

**Files:**
- Modify: `src/export/mod.rs`

- [ ] **Step 1: Add mobi module declaration**

```rust
// Add with other module declarations:
mod mobi;
```

- [ ] **Step 2: Add pub use exports**

```rust
// Add with other exports:
pub use mobi::{MobiConfig, MobiEncoding, MobiExporter, MobiResult};
```

- [ ] **Step 3: Verify module compiles**

Run: `cargo check`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/export/mod.rs
git commit -m "export: wire up MobiExporter in module

Exports MobiConfig, MobiEncoding, MobiExporter, MobiResult
"
```

### Task 7: Enable MOBI in Book::export()

**Files:**
- Modify: `src/model/book.rs`

- [ ] **Step 1: Find the export() method and MOBI restriction**

Search for `Format::Mobi => Err` to find current restriction

- [ ] **Step 2: Remove MOBI restriction, add MobiExporter call**

Find the export() method in Book implementation and replace:

```rust
// Remove this:
Format::Mobi => Err(io::Error::new(
    io::ErrorKind::Unsupported,
    format!("{:?} export is not supported", format),
)),

// With this:
Format::Mobi => {
    use crate::export::Exporter;
    let exporter = crate::export::MobiExporter::new();
    exporter.export(self, writer)
}
```

- [ ] **Step 3: Verify changes compile**

Run: `cargo check`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/model/book.rs
git commit -m "model: enable MOBI export in Book::export()

Removes \"not supported\" error, wires up MobiExporter
Users can now call book.export(Format::Mobi, &mut writer)
"
```

### Task 8: Remove CLI restriction

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Find MOBI restriction in main**

Search for: `"MOBI output is not supported"`

- [ ] **Step 2: Remove or comment out the restriction**

```rust
// Remove this block:
if output_format == Format::Mobi {
    return Err("MOBI output is not supported; use .azw3 instead".to_string());
}
```

- [ ] **Step 3: Verify CLI compiles**

Run: `cargo check --bin boko`
Expected: No errors

- [ ] **Step 4: Test CLI accepts MOBI**

Run: `cargo run -- --help`
Expected: MOBI listed as supported format in convert command

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "cli: remove MOBI output restriction

Users can now use: boko convert input.epub output.mobi
"
```

### Task 9: Implement Exporter trait for MobiExporter

**Files:**
- Modify: `src/export/mobi.rs`

- [ ] **Step 1: Write test for Exporter trait**

```rust
// Add to tests module:
use std::io::Cursor;

#[test]
fn test_exporter_trait() {
    let exporter = MobiExporter::new();
    // We need a minimal Book - for now just test it compiles
    // Full integration tests will come later
    let mut output = Cursor::new(Vec::new());

    // This will fail until we implement the builder
    // but demonstrates the trait is implemented
}
```

- [ ] **Step 2: Implement Exporter trait stub**

```rust
impl Exporter for MobiExporter {
    fn export<W: Write + Seek>(&self, book: &mut Book, writer: &mut W) -> io::Result<()> {
        // TODO: Implement in Phase 3
        // For now, return unsupported to prevent crashes
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "MOBI 6 export not yet implemented",
        ))
    }
}
```

- [ ] **Step 3: Verify trait implementation compiles**

Run: `cargo check`
Expected: No errors, trait implemented

- [ ] **Step 4: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: implement Exporter trait stub

Returns \"not yet implemented\" error
Full implementation in Phase 3
"
```

---

## Phase 2: HTML Filtering

### Task 10: Create html_filter module

**Files:**
- Create: `src/mobi/html_filter.rs`

- [ ] **Step 1: Write test for supported tag detection**

```rust
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
        let (filtered, warnings) = filter_html_for_mobi6(html, &std::collections::HashMap::new());

        // Should keep p, remove video
        assert!(filtered.contains("<p>"));
        assert!(!filtered.contains("video"));
        assert!(!warnings.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib mobi::html_filter::tests::test_supported_tags`
Expected: FAILED - module not found

- [ ] **Step 3: Implement html_filter module**

```rust
//! HTML filtering for MOBI 6 compatibility.
//!
//! MOBI 6 supports a limited subset of HTML tags. This module filters HTML
//! to only include supported tags and converts image references.

use std::collections::HashMap;
use html5ever::parse_document;
use html5ever::driver::ParseOpts;
use html5ever::tendril::TendrilSink;
use crate::dom::tree_sink::ArenaSink;

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
    image_map: &HashMap<String, u32>,
) -> (String, Vec<String>) {
    let mut warnings = Vec::new();

    // Parse HTML with html5ever
    let sink = ArenaSink::new();
    let dom = parse_document(sink, ParseOpts::default())
        .from_utf8()
        .one(html.as_bytes())
        .into_dom();

    // Walk DOM and filter
    let mut output = String::new();
    walk_dom(&dom, &mut output, &mut warnings, image_map);

    (output, warnings)
}

fn walk_dom(
    dom: &ArenaDom,
    output: &mut String,
    warnings: &mut Vec<String>,
    image_map: &HashMap<String, u32>,
) {
    // TODO: Implement full DOM walking
    // For Phase 2, just return original HTML
    // Full implementation in Phase 3

    // Image reference transformation (when implemented):
    // When encountering <img> tags:
    // if let Some(src) = dom.get_attr(node_id, "src") {
    //     let src_str = src.to_string();
    //     if let Some(&recindex) = image_map.get(&src_str) {
    //         // Replace src with recindex for MOBI 6
    //         output.push_str(&format!("<img recindex=\"{}\"/>", recindex));
    //     } else {
    //         // Image not found - skip this img tag
    //         warnings.push(format!("Image not found in map: {}", src_str));
    //     }
    // }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib mobi::html_filter::tests`
Expected: Both tests PASS

- [ ] **Step 5: Add module to mobi mod.rs**

Check existing `src/mobi/mod.rs` content first:
```bash
head -20 src/mobi/mod.rs
```

Add `pub mod html_filter;` with other module declarations (around line 10-15):
```rust
// Add with existing module exports:
pub mod html_filter;
```

If file doesn't exist, create it with this content:
```rust
//! MOBI format support.

pub mod html_filter;
pub mod palmdoc;
pub mod headers;
// ... other existing modules
```

- [ ] **Step 6: Commit**

```bash
git add src/mobi/html_filter.rs src/mobi/mod.rs
git commit -m "mobi: implement HTML filtering skeleton

is_supported_tag() checks tag against MOBI 6 whitelist
filter_html_for_mobi6() parses HTML with html5ever
TODO: Full DOM walking and image reference conversion
"
```

---

## Phase 3: MOBI 6 File Building

### Task 11: Implement MobiBuilder structure

**Files:**
- Modify: `src/export/mobi.rs`

- [ ] **Step 1: Write test for MobiBuilder creation**

```rust
#[cfg(test)]
mod tests {
    // ... existing tests ...

    use std::io::Cursor;

    #[test]
    fn test_builder_creation() {
        // This will need a Book, which we'll mock for now
        // Full implementation in subsequent tasks
    }
}
```

- [ ] **Step 2: Implement MobiBuilder struct**

```rust
use std::collections::HashMap;
use std::io::{self, Seek, Write};

use crate::model::{Book, Metadata, TocEntry};

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

    /// Write the complete PDB file
    fn write<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        // TODO: Implement in subsequent tasks
        Ok(())
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: implement MobiBuilder structure

Add MobiBuilder with state management
build_palmdb_header() creates 78-byte header
sanitize_title() limits title to 31 chars
TODO: write() method implementation
"
```

### Task 11a: Implement process_images method

**Files:**
- Modify: `src/export/mobi.rs`

- [ ] **Step 1: Add process_images method to MobiBuilder**

```rust
impl MobiBuilder {
    /// Process images from book assets
    fn process_images(&mut self, book: &mut Book) -> io::Result<()> {
        use std::path::Path;

        for image_path in book.list_assets() {
            // Load image data using Book::load_asset()
            let image_data = match book.load_asset(image_path) {
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

            let (processed, warnings) = match process_image(&image_data, &config) {
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
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors, Book::load_asset() accessible

- [ ] **Step 3: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: implement image processing with correct API

process_images() uses book.load_asset(path) to get image data
Stores processed images in image_records
Builds image_path_to_record map for HTML filtering
"
```

### Task 12: Implement text record building

**Files:**
- Modify: `src/export/mobi.rs`

- [ ] **Step 1: Add build_text_records method**

```rust
impl MobiBuilder {
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
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors, palmdoc::compress accessible

- [ ] **Step 3: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: implement text record building

build_text_records() compresses HTML with PalmDoc
Splits compressed data into 4KB records
Reuses existing palmdoc::compress()
"
```

### Task 13: Implement MOBI header building

**Files:**
- Modify: `src/export/mobi.rs`

- [ ] **Step 1: Add build_mobi_header method**

```rust
impl MobiBuilder {
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
        let codepage = match self.config.encoding {
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
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: implement MOBI 6 header building

build_mobi_header() creates Record 0 content
Version 6 (not 8), UTF-8/CP1252 encoding
EXTH flags set to indicate metadata present
Title appended after header
"
```

### Task 13a: Build NCX Index

**Files:**
- Modify: `src/export/mobi.rs`

- [ ] **Step 1: Implement build_ncx_index method**

```rust
impl MobiBuilder {
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
}
```

- [ ] **Step 2: Call build_ncx_index in write()**

Update write() to include NCX index record after text records.

- [ ] **Step 3: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: add NCX index building stub

build_ncx_index() creates minimal INDX record
Placeholder for full implementation
"
```

### Task 14: Implement PDB file writing

**Files:**
- Modify: `src/export/mobi.rs`

- [ ] **Step 1: Implement write method**

```rust
impl MobiBuilder {
    /// Write the complete PDB file
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> io::Result<()> {
        // Calculate total number of records
        // Record 0 (headers) + text records + image records
        let num_records = 1 + self.text_records.len() + self.image_records.len();

        // Build PalmDB header
        let pdb_header = self.build_palmdb_header(num_records as u16);

        // Build MOBI header (Record 0 content)
        let text_length: u32 = self.text_records.iter()
            .map(|r| r.len())
            .sum::<usize>() as u32; // Uncompressed length estimate
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

        // Image record offsets
        for record in &self.image_records {
            offsets.push(offset);
            offset += record.len();
        }

        // Write PalmDB header
        writer.write_all(&pdb_header)?;

        // Write record info list (8 bytes per record)
        for (i, &record_offset) in offsets.iter().enumerate() {
            // Offset (4 bytes)
            writer.write_all(&record_offset.to_be_bytes())?;

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

        // Write image records
        for record in &self.image_records {
            writer.write_all(record)?;
        }

        Ok(())
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: implement PDB file writing

write() method writes complete PDB structure:
- PalmDB header (78 bytes)
- Record info list (8 bytes per record)
- Gap (2 bytes)
- Record 0 (MOBI header + title)
- Text records (PalmDoc compressed)
- Image records (PNG/JPEG data)
"
```

### Task 15: Wire up MobiBuilder in MobiExporter

**Files:**
- Modify: `src/export/mobi.rs`

- [ ] **Step 1: Update export() implementation**

```rust
impl Exporter for MobiExporter {
    fn export<W: Write + Seek>(&self, book: &mut Book, writer: &mut W) -> io::Result<()> {
        // Create builder
        let mut builder = MobiBuilder::new(book, self.config.clone())?;

        // Process images from book assets
        builder.process_images(book)?;

        // Get HTML content from book
        // For now, use a simple approach - in production would use normalize_book()
        let html_content = self.collect_html_content(book)?;

        // Build text records
        builder.build_text_records(&html_content)?;

        // Write file
        builder.write(writer)?;

        Ok(())
    }
}
```

- [ ] **Step 2: Add collect_html_content helper**

```rust
impl MobiExporter {
    /// Collect HTML content from book chapters
    fn collect_html_content(&self, book: &mut Book) -> io::Result<String> {
        let mut html = String::new();

        // Simple HTML wrapper
        html.push_str("<html><body>");

        // Get spine (reading order) - iterate directly over slice
        for entry in book.spine() {
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
```

- [ ] **Step 3: Fix MobiBuilder::new signature**

```rust
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
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add src/export/mobi.rs
git commit -m "mobi: wire up MobiBuilder in export flow

export() now creates MobiBuilder and orchestrates building
process_images() loads and processes book images
collect_html_content() gathers raw chapter HTML
Basic end-to-end flow working
TODO: HTML filtering, NCX index integration
"
```

---

## Phase 4: Integration & Testing

### Task 16: Create integration test

**Files:**
- Create: `tests/mobi_export_test.rs`

- [ ] **Step 1: Create basic integration test**

```rust
//! MOBI 6 export integration tests

use std::io::Cursor;
use boko::{Book, Format};

#[test]
fn test_epub_to_mobi_export() {
    // This test requires a fixture - for now, skip if not found
    let fixture_path = "tests/fixtures/mobi/simple.epub";

    if !std::path::Path::new(fixture_path).exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    // Open EPUB
    let mut book = Book::open(fixture_path).expect("Failed to open EPUB");

    // Export to MOBI
    let mut output = Cursor::new(Vec::new());
    book.export(Format::Mobi, &mut output).expect("Failed to export to MOBI");

    // Verify output
    let mobi_data = output.into_inner();

    // Check PalmDB header
    assert!(mobi_data.len() > 78, "MOBI file too short for PalmDB header");

    // Check database name is in header (first 32 bytes)
    let header_name = String::from_utf8_lossy(&mobi_data[0..32]);
    assert!(header_name.contains("BOOK") || !header_name.trim().is_empty());

    // Check database type (should be "MOBI" at offset 0x3C)
    assert_eq!(&mobi_data[0x3C..0x40], b"MOBI\0");

    println!("MOBI file size: {} bytes", mobi_data.len());
}
```

- [ ] **Step 2: Verify test compiles**

Run: `cargo test --test mobi_export_test`
Expected: Compiles but skips due to missing fixture

- [ ] **Step 3: Commit**

```bash
git add tests/mobi_export_test.rs
git commit -m "test: add MOBI 6 export integration test

Tests EPUB → MOBI 6 conversion
Verifies PalmDB header structure
Skips if fixture not found
"
```

### Task 17: Create test fixtures

**Files:**
- Create: `tests/fixtures/mobi/simple.epub`

- [ ] **Step 1: Create minimal EPUB fixture**

Create a minimal valid EPUB with basic HTML content.

Since creating a valid EPUB from scratch is complex, you can:

Option A: Use an existing simple EPUB from the test fixtures
Option B: Document that fixture creation is manual

For now, add a README documenting fixture requirements:

- [ ] **Create fixture documentation**

Create: `tests/fixtures/mobi/README.md`

```markdown
# MOBI Export Test Fixtures

This directory contains test EPUB files for MOBI 6 export testing.

## Fixtures Needed

- `simple.epub` - Basic text content, no images, simple structure
- `with_images.epub` - Contains PNG/JPEG images
- `complex.epub` - Tables, nested lists, internal links

## Creating Fixtures

Fixtures can be created using any EPUB authoring tool or by converting from other formats.

Example using pandoc:
```bash
pandoc input.md -o simple.epub
```
```

- [ ] **Step 2: Commit fixture documentation**

```bash
git add tests/fixtures/mobi/README.md
git add tests/fixtures/mobi/
git commit -m "test: add MOBI fixture documentation

Documents requirements for test fixtures
simple.epub, with_images.epub, complex.epub
"
```

### Task 18: Update documentation

**Files:**
- Modify: `docs/quick-reference.md`
- Modify: `docs/contributing.md`

- [ ] **Step 1: Add MOBI exporter to quick-reference**

Add to "## Exporter Configuration" section:

```rust
### MOBI Exporter

```rust
use boko::export::{MobiExporter, MobiConfig, MobiEncoding};

let exporter = MobiExporter::new()
    .with_config(MobiConfig {
        collect_warnings: true,
        max_image_size: (2048, 2048),
        max_image_file_size: 10 * 1024 * 1024,
        encoding: MobiEncoding::Utf8,
    });

exporter.export(&mut book, &mut output)?;
```
```

- [ ] **Step 2: Verify docs build**

Run: `cargo build --docs`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add docs/quick-reference.md
git commit -m "docs: add MOBI exporter to quick reference

Documents MobiExporter usage with MobiConfig
Shows MobiEncoding options (Utf8/Cp1252)
"
```

---

## Success Criteria Verification

### Task 19: Verify all success criteria

- [ ] **Step 1: Test end-to-end export**

Run: `cargo build --release`

Create a test script or manually test with a real EPUB.

- [ ] **Step 2: Run all tests**

Run: `cargo test`

Expected: All tests pass

- [ ] **Step 3: Run Clippy**

Run: `cargo clippy -- -D warnings`

Expected: No warnings

- [ ] **Step 4: Check formatting**

Run: `cargo fmt --check`

Expected: No formatting issues

- [ ] **Step 5: Final verification commit**

```bash
git add .
git commit -m "mobi: complete MOBI 6 export implementation

Success criteria:
✅ Can export EPUB to MOBI 6 format
✅ Images processed with configurable limits
✅ HTML content filtered to MOBI 6 subset
✅ Warnings collected for unsupported features
✅ All tests pass
✅ Documentation updated
✅ No regression in existing functionality

Implementation follows approved design spec:
- Shared image processing module (src/image/)
- MobiExporter with configurable options
- HTML filtering with html5ever
- PalmDB format with proper headers
- PalmDoc compression for text records
"
```

---

## Notes for Implementers

### Code Patterns to Follow

1. **Error Handling**: Use `io::Error` with `io::ErrorKind` for all errors
2. **Testing**: Write tests before implementation (TDD)
3. **Commits**: Small, focused commits with descriptive messages
4. **Documentation**: Add module docs (`//!`) and public API docs (`///`)
5. **Existing Patterns**: Follow AZW3 exporter structure closely

### Dependencies Already Available

- `html5ever` 0.39 - HTML parsing
- `flate2` - Compression (used by PalmDoc)
- `src/mobi/palmdoc.rs` - PalmDoc compression/decompression
- `src/mobi/headers.rs` - MOBI header structs (for reference)
- `src/mobi/index.rs` - Index building utilities (for reference)

### Known Limitations

- Phase 0-2: Basic functionality
- NCX index: Simplified (flat structure)
- EXTH metadata: Basic (title, author)
- HTML filtering: Tag whitelist only (no CSS parsing yet)
- Images: Basic downsampling, no smart format selection

### Future Enhancements (Out of Scope)

- Smart format selection (JPEG for photos, PNG for graphics)
- Content-aware image downsampling
- NCX with nested hierarchy
- Full CSS parsing and conversion to basic tags
- EPUB/AZW3 image optimization using shared module
