# MOBI 6 Export Design

**Date:** 2025-03-28
**Status:** Draft
**Author:** AI Design

## Overview

Add legacy MOBI 6 export support to the Boko ebook conversion library. MOBI 6 is the legacy Kindle format that predates AZW3/KF8.

**Important Clarification:** MOBI 6 **does support HTML** - it uses a subset of HTML tags, not plain text. This is a key distinction from the initial design.

**Goal:** Enable exporting books to MOBI 6 format by:
- Using simplified HTML (subset of full HTML) that MOBI 6 supports
- Leveraging existing HTML synthesis infrastructure
- Keeping images in supported formats (PNG/JPEG preferred over GIF)
- Converting only when necessary (unsupported formats, oversize images)
- Warning users about features that can't be represented

## Requirements

### Functional Requirements

1. **Format Support:** Export books to MOBI 6 format (`.mobi` extension)
2. **Content Conversion:**
   - Filter HTML to MOBI 6 supported tags (h1-h6, p, br, i, b, u, ul, ol, li, table, img, etc.)
   - Strip or simplify CSS styles
   - Preserve images in PNG/JPEG format (convert unsupported formats like SVG/WebP)
   - Downscale oversized images (> 2048x2048)
   - Remove custom fonts (use device fonts)
   - Preserve basic structure (headings, paragraphs, lists, tables)
3. **Navigation:** Preserve chapter-to-chapter links
4. **Metadata:** Include title, author, and other metadata in MOBI headers
5. **Warnings:** Warn users about unsupported features (CSS, fonts, complex layouts)

### Non-Functional Requirements

1. **Performance:** Handle large books efficiently
2. **Compatibility:** Work with existing `Exporter` trait pattern
3. **Code Quality:** Follow existing codebase patterns
4. **Testing:** Comprehensive unit and integration tests

## Architecture

### Approach

Create a separate `MobiExporter` following the same pattern as `Azw3Exporter` and `EpubExporter`.

```
src/export/mobi.rs (new)
├── MobiExporter       - Main exporter implementing Exporter trait
├── MobiConfig         - Configuration options
└── MobiBuilder        - Internal builder for MOBI file creation

Reuses from existing code:
├── src/export/html_synth.rs  - HTML synthesis (reused with MOBI 6 constraints)
├── src/export/normalize.rs  - Normalization pipeline (reused)
└── src/mobi/:
    ├── palmdoc.rs        - PalmDoc compression
    ├── headers.rs        - MOBI header structures
    └── index.rs          - Index building utilities

New modules:
├── src/mobi/html_filter.rs  - Filter HTML to MOBI 6 supported tags
└── src/mobi/image_convert.rs - Image processing (downscale, format convert)
```

### Key Design Changes from Initial Draft

1. **HTML Support, Not Plain Text:** MOBI 6 supports HTML tags including `<h1>`-`<h6>`, `<p>`, `<br>`, `<i>`, `<b>`, `<u>`, `<img>`, `<table>`, etc. We will use the existing HTML synthesis pipeline with MOBI 6 constraints.

2. **Reuse Existing Infrastructure:** Instead of creating a new text conversion module, we extend the existing `html_synth.rs` and `normalize.rs` to support MOBI 6's HTML subset.

3. **Image Handling:** Keep images in PNG/JPEG format (preferred over GIF). Only convert when:
   - Format is unsupported (SVG, WebP, etc.)
   - Image dimensions are too large (> 2048x2048)
   - File size is too large (> 10MB)

4. **Warning System:** Simplified approach - collect warnings during export and return them, rather than callback pattern.

### Data Flow

```
Book (IR)
    ↓
MobiBuilder::normalize_content()
    ├─ Use existing normalize_book() to get HTML + CSS
    └─ Collect warnings about unsupported features
    ↓
MobiBuilder::filter_html()
    ├─ Filter HTML to MOBI 6 supported tags
    ├─ Remove/transform CSS
    └─ Collect warnings
    ↓
MobiBuilder::process_images()
    ├─ Downscale oversized images
    ├─ Convert unsupported formats
    └─ Collect warnings
    ↓
MobiBuilder::build_text_records()
    ├─ Apply PalmDoc compression
    ├─ Insert image record references
    └─ Split into 4KB records
    ↓
MobiBuilder::build_indexes()
    ├─ NCX index (TOC)
    ├─ INDX records (MOBI 6 specific structure)
    └─ SKEL/FRAG indexes (simplified, optional)
    ↓
MobiBuilder::build_headers()
    ├─ PalmDB header
    ├─ MOBI header (version 6, not 8)
    ├─ EXTH metadata
    └─ Character encoding (CP1252 or UTF-8)
    ↓
MobiBuilder::write()
    └─ Write PDB file with records
```

## MOBI 6 File Format Structure

MOBI 6 files are PalmDB databases with a specific record layout. Understanding this structure is critical for implementation.

### Record Layout

```
Record 0: PalmDB + MOBI Header (merged)
├── PalmDB header (78 bytes)
│   ├── Database name: "BOOKMOBI"
│   ├── File type: "MOBI"
│   ├── Creator: "MOBI"
│   └── Number of records
└── MOBI header (variable, ~232+ bytes)
    ├── Compression type: 2 (PalmDoc)
    ├── MOBI version: 6 (NOT 8!)
    ├── Text record count
    ├── Text record size: 4096
    ├── Encoding: 65001 (UTF-8) or 1252 (CP1252)
    ├── First image index
    └── EXTH header flag

Record 1 to N: Text Records
├── PalmDoc compressed HTML content
├── Split into 4KB (4096 byte) chunks
└── Last record may be smaller

Record N+1 to M: Image Records
├── Raw image data (PNG/JPEG/GIF)
├── One record per image
└── Order matches first_image_index

Record M+1: EXTH Header (if exth_flags set)
├── Author
├── Title
├── Publisher
└── Other metadata

Record M+2+: Index Records (optional but recommended)
├── NCX index: Table of contents
└── INDX records: Navigation structures
```

### Key Header Fields

**MOBI Header (offsets from Record 0 start):**
- `0x00-0x01`: Compression type (2 = PalmDoc)
- `0x08-0x09`: Text record count
- `0x0A-0x0B`: Text record size (4096)
- `0x14-0x17`: MOBI header length
- `0x18-0x1B`: MOBI type (2 = standard book)
- `0x1C-0x1F`: Codepage (65001 = UTF-8, 1252 = CP1252)
- `0x68-0x6B`: MOBI version (6 for MOBI 6, 8 for KF8/AZW3)
- `0x6C-0x6F`: First image index

**EXTH Header (if present):**
- Starts with "EXTH" magic
- Contains header length, record count
- Metadata records: author, title, publisher, etc.

### Image Reference Format

**Critical difference between MOBI 6 and AZW3:**

- **MOBI 6**: Uses `<img recindex="N"/>` format where N is the record number
  ```html
  <img recindex="0"/> refers to first image record
  <img recindex="5"/> refers to sixth image record
  ```

- **AZW3/KF8**: Uses `<img src="kindle:embed:XXXX?mime=image/jpeg"/>` format

**Implementation requirement:**
```rust
// Must convert standard HTML <img src="path/to/image.jpg">
// to MOBI 6 format <img recindex="3"/>
// where 3 is the record number containing that image
```

### Character Encoding

**Default strategy:**
- Use UTF-8 encoding (codepage 65001) for modern devices
- Optional CP1252 (codepage 1252) for legacy devices
- Transcode if needed: UTF-8 → CP1252, replace unsupported chars with '?'

**Implementation:**
```rust
pub enum MobiEncoding {
    Utf8,      // Default - modern Kindles
    Cp1252,    // Legacy - very old devices
}

pub struct MobiConfig {
    pub collect_warnings: bool,
    pub max_image_size: (u32, u32),
    pub max_image_file_size: u64,
    pub encoding: MobiEncoding,  // Add this
}
```

## Components

### 1. MobiExporter

**Location:** `src/export/mobi.rs`

```rust
pub struct MobiExporter {
    config: MobiConfig,
}

pub struct MobiConfig {
    pub collect_warnings: bool,
    pub max_image_size: (u32, u32),      // (width, height) - default (2048, 2048)
    pub max_image_file_size: u64,         // bytes - default 10MB
    pub encoding: MobiEncoding,           // default Utf8
}

pub enum MobiEncoding {
    Utf8,      // Modern Kindles - recommended
    Cp1252,    // Legacy devices - transcoded from UTF-8
}

impl Default for MobiConfig {
    fn default() -> Self {
        Self {
            collect_warnings: true,
            max_image_size: (2048, 2048),
            max_image_file_size: 10 * 1024 * 1024,  // 10MB
            encoding: MobiEncoding::Utf8,
        }
    }
}

impl Exporter for MobiExporter {
    fn export<W: Write + Seek>(&self, book: &mut Book, writer: &mut W) -> io::Result<()>;
}

impl MobiExporter {
    pub fn export_with_warnings<W: Write + Seek>(
        &self,
        book: &mut Book,
        writer: &mut W,
    ) -> io::Result<MobiResult>;
}
```

### 2. MobiBuilder

Internal builder that constructs the MOBI file.

**State:**
- `text_records: Vec<Vec<u8>>` - Compressed text records
- `image_records: Vec<Vec<u8>>` - Images (PNG/JPEG preferred)
- `metadata: Metadata` - Book metadata
- `toc: Vec<TocEntry>` - Table of contents
- `warnings: Vec<String>` - Collected warnings

**Key Methods:**
- `new(book: &mut Book) -> io::Result<Self>` - Collect content
- `build_text_records() -> io::Result<()>` - Compress and chunk text
- `build_indexes() -> io::Result<()>` - Build navigation indexes
- `build_headers() -> io::Result<Vec<u8>>` - Build record 0
- `write<W: Write + Seek>(writer: &mut W) -> io::Result<()>` - Write PDB file

### 3. HTML Filter (MOBI 6 Constraints)

**Location:** `src/mobi/html_filter.rs`

Filters HTML content to only include MOBI 6 supported tags.

**Implementation Approach:**

Use `html5ever` (already a dependency) for DOM-based filtering:

```rust
use html5ever::{parse_document, ParseOpts};
use html5ever::tree_builder::TreeBuilder;
use html5ever::rcdom::{RcDom, Handle};

pub fn filter_html_for_mobi6(html: &str) -> (String, Vec<String>) {
    // 1. Parse HTML with html5ever (robust, handles malformed HTML)
    let dom = parse_document(RcDom::new(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .unwrap();

    // 2. Walk DOM tree recursively
    // 3. For each node:
    //    - If unsupported tag: remove or replace with closest equivalent
    //    - If style attribute: strip or convert to basic formatting
    //    - If class/id: remove (not useful in MOBI 6)
    // 4. Serialize back to HTML

    let filtered = serialize_filtered_dom(dom.document);
    let warnings = collect_warnings();

    (filtered, warnings)
}

fn convert_css_to_basic_tag(css_property: &str, css_value: &str) -> Option<&'static str> {
    match (css_property, css_value) {
        ("font-style", "italic") => Some("i"),
        ("font-weight", "bold") => Some("b"),
        ("text-decoration", "underline") => Some("u"),
        _ => None,
    }
}
```

**Supported Tags:**
- Headings: `<h1>` through `<h6>`
- Paragraphs: `<p>`
- Line breaks: `<br>`
- Basic formatting: `<i>`, `<b>`, `<u>`
- Lists: `<ul>`, `<ol>`, `<li>`
- Tables: `<table>`, `<tr>`, `<td>`, `<th>`
- Images: `<img>` (converted to `recindex` format later)
- Divs/spans (limited support - stripped if no attributes)

**Removed/Transformed:**
- CSS styles → parsed, converted to basic tags if possible, otherwise stripped
- Style attributes → removed (MOBI 6 doesn't support inline styles)
- Class/id attributes → removed
- Fonts → removed, use device fonts
- Colored text → stripped (color info lost)
- Complex layouts → flattened
- Unsupported tags → removed or replaced with simpler alternatives

**API:**
```rust
pub fn filter_html_for_mobi6(html: &str) -> (String, Vec<String>);
// Returns (filtered_html, warnings)

pub fn is_supported_tag(tag: &str) -> bool;
```

### 4. Image Converter

**Location:** `src/mobi/image_convert.rs`

Process images for MOBI 6 compatibility (downscale, format conversion if needed).

**Operations:**
- Check image dimensions
- Downscale if too large (> 2048x2048)
- Convert unsupported formats (SVG → PNG, WebP → JPEG)
- Optimize file size if needed

**Dependencies:**
- `image` crate (feature-gated: `gif`, `jpeg`, `png`)

**API:**
```rust
pub fn process_image(data: &[u8], max_size: (u32, u32)) -> io::Result<(Vec<u8>, Vec<String>)>;
// Returns (processed_image_data, warnings)

pub fn is_supported_format(path: &str) -> bool;
```

## Warning System

### Warning Categories

1. **Content Warnings** (non-fatal):
   - CSS detected - will be stripped or simplified
   - Custom fonts - ignored, using device fonts
   - Unsupported HTML tags - removed or replaced
   - Complex layouts - simplified
   - Deep nesting - flattened
   - Tables - may be simplified if too complex
   - Links - only chapter-level links preserved (anchors lost)

2. **Image Warnings:**
   - Unsupported image formats (SVG, WebP) - converted or skipped
   - Images too large - downsampled
   - Corrupted images - skipped
   - Animated GIFs - first frame only

3. **Structure Warnings:**
   - Missing chapter titles - using generic names
   - TOC entries without valid targets - link may be broken

### Warning Collection (Simplified)

Simplified approach to avoid thread safety issues with callback pattern:

```rust
pub struct MobiConfig {
    pub collect_warnings: bool,
}

pub struct MobiResult {
    pub success: bool,
    pub warnings: Vec<String>,
}

// Alternative: Extended export API
impl MobiExporter {
    pub fn export_with_warnings<W: Write + Seek>(
        &self,
        book: &mut Book,
        writer: &mut W,
    ) -> io::Result<MobiResult>;
}
```

**CLI Usage:**
```rust
let exporter = MobiExporter::new();
let result = exporter.export_with_warnings(&mut book, &mut output)?;

for warning in &result.warnings {
    eprintln!("Warning: {}", warning);
}
```

**Library Usage (simple):**
```rust
let exporter = MobiExporter::new();
exporter.export(&mut book, &mut output)?;  // Warnings go to stderr
```

## Integration Points

### Modify Existing Files

**`src/export/mod.rs`:**
```rust
pub use mobi::{MobiConfig, MobiExporter};
```

**`src/model/book.rs`:**
```rust
pub fn export<W>(&mut self, format: Format, writer: &mut W) -> io::Result<()>
where
    W: Write + Seek,
{
    match format {
        Format::Epub => EpubExporter::new().export(self, writer),
        Format::Azw3 => Azw3Exporter::new().export(self, writer),
        Format::Mobi => MobiExporter::new().export(self, writer), // Enable
        Format::Kfx => KfxExporter::new().export(self, writer),
        Format::Markdown => MarkdownExporter::new().export(self, writer),
    }
}
```

**`src/main.rs`:**
Remove the restriction on MOBI output:
```rust
// Remove this check:
if output_format == Format::Mobi {
    return Err("MOBI output is not supported; use .azw3 instead".to_string());
}
```

## Dependencies

### New Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# Image processing (for format conversion and downsampling)
# Using 0.24 for stability (0.25 is very new)
image = { version = "0.24", default-features = false, features = ["gif", "jpeg", "png", "webp"] }
```

**Feature explanation:**
- `gif`: For decoding existing GIF images in source books
- `jpeg`: For JPEG encoding/decoding (preferred format)
- `png`: For PNG encoding/decoding (preferred format)
- `webp`: For WebP decoding (convert to JPEG/PNG)

Note: We prefer PNG/JPEG for output. GIF feature is only for decoding existing images. WebP images are converted to JPEG/PNG.

### Reused Dependencies

- `flate2` - Already used for PalmDoc compression
- `html5ever` - Already used for HTML parsing in `src/export/html_synth.rs`
- `encoding_rs` - Already used for character encoding conversion
- Existing `mobi/` module utilities (`palmdoc.rs`, `headers.rs`, `index.rs`)

## Testing Strategy

### Unit Tests

**HTML Filtering** (`src/mobi/html_filter.rs` tests):
- Tag filtering (supported tags preserved, unsupported removed)
- CSS stripping and simplification
- Style attribute conversion to basic formatting
- Nested element handling
- Table structure preservation

**Image Processing** (`src/mobi/image_convert.rs` tests):
- PNG/JPEG preservation (no conversion when possible)
- SVG → PNG conversion
- WebP → JPEG conversion
- Large image downsampling (> 2048x2048)
- File size optimization (> 10MB)

**Header/Record Building** (`src/export/mobi.rs` tests):
- MOBI 6 header generation (version 6, not 8)
- PalmDoc compression
- Record size calculations (4KB chunks)
- EXTH metadata
- Image record references

**Parsing Validation** (`tests/` integration):
- Verify generated MOBI 6 files are valid PalmDB format
- Verify MOBI header has correct version (6)
- Verify text records decompress correctly
- Verify index structures are valid

### Integration Tests

**Round-trip tests:**
- EPUB → MOBI 6 → verify readable
- Test with sample books of varying complexity

**Regression tests:**
- Compare output against known-good MOBI 6 files
- Ensure structure compatibility (not byte-for-byte due to timestamps)

**Comparison tests:**
- Export same book to both MOBI 6 and AZW3
- Verify MOBI 6 is subset (simpler formatting, no fonts)
- Verify text content is identical
- Verify images are present (possibly different formats)

**Warning system tests:**
- Verify warnings issued for CSS
- Verify warnings issued for custom fonts
- Verify warnings issued for unsupported HTML tags
- Verify warnings issued for image conversions

### Test Fixtures

Place in `tests/fixtures/mobi/`:
- `simple.epub` - Basic text, no images
- `with_images.epub` - Contains images
- `complex.epub` - Tables, nested lists, links

## Implementation Phases

### Phase 1: Foundation
1. Create `src/export/mobi.rs` with basic structure
2. Implement `MobiExporter` and `MobiConfig`
3. Wire up in `src/export/mod.rs` and `src/model/book.rs`
4. Remove CLI restriction in `src/main.rs`
5. Write basic unit tests for exporter structure

### Phase 2: HTML Filtering
1. Create `src/mobi/html_filter.rs`
2. Implement `filter_html_for_mobi6()` function
3. Define supported tag set (h1-h6, p, br, i, b, u, ul, ol, li, table, tr, td, th, img)
4. Implement CSS stripping/simplification
5. Write unit tests for HTML filtering

### Phase 3: Image Processing
1. Create `src/mobi/image_convert.rs`
2. Implement format detection (PNG/JPEG/SVG/WebP)
3. Implement image downsampling (> 2048x2048)
4. Implement format conversion (SVG → PNG, WebP → JPEG)
5. Add `image` crate dependency (feature-gated: gif, jpeg, png)
6. Write unit tests for image processing

### Phase 4: MOBI 6 File Building
1. Implement `MobiBuilder` with state management
2. Integrate with existing `normalize_book()` from `src/export/normalize.rs`
3. Implement text record building with PalmDoc compression
4. Implement MOBI 6 header generation (version 6, not 8)
5. Implement index building (NCX, INDX records)
6. Implement PDB file writing
7. Write unit tests for each component

### Phase 5: Integration & Testing
1. Add integration tests (EPUB → MOBI 6)
2. Add test fixtures (simple, with images, complex)
3. Add comparison tests (MOBI 6 vs AZW3 output)
4. Test with real books from `tests/fixtures/`
5. Update documentation (quick-reference, contributing)
6. Verify warning collection works correctly

## Open Questions

1. **Image Format for MOBI 6:** Should we use PNG, JPEG, or GIF?
   - **Answer:** Prefer PNG/JPEG (most compatible). Only convert to GIF if device requires it. MOBI 6 supports all three formats.

2. **Image Size Limits:** What's the maximum image size to support?
   - **Answer:** Downscale images > 2048x2048 pixels. Skip/warn if file size > 10MB after downsampling. These limits ensure compatibility with older Kindle devices.

3. **Table Handling:** MOBI 6 supports `<table>` tags - should we preserve them or flatten?
   - **Answer:** Preserve table structure (MOBI 6 supports basic tables). Only warn if tables are deeply nested or complex (merged cells, nested tables).

4. **Link Handling:** How to handle internal links to specific sections?
   - **Answer:** Preserve chapter-to-chapter links. For intra-chapter anchors, convert to chapter-start links and warn about lost precision. MOBI 6 has limited anchor support.

5. **CSS Handling:** How much CSS should we preserve?
   - **Answer:** Strip all CSS (MOBI 6 has minimal CSS support). Convert basic formatting to `<i>`, `<b>`, `<u>` tags. Warn user about loss of styling.

## Success Criteria

1. ✅ Can export EPUB to MOBI 6 format
2. ✅ Images preserved as PNG/JPEG when possible, converted only when necessary
3. ✅ HTML content filtered to MOBI 6 supported tags
4. ✅ Text content preserved with basic formatting (i, b, u tags)
5. ✅ Warnings issued for unsupported features (CSS, fonts, complex layouts)
6. ✅ All tests pass (unit, integration, comparison)
7. ✅ Documentation updated (quick-reference, contributing)
8. ✅ No regression in existing functionality
