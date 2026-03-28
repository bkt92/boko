# Export System

The export system writes ebooks in various formats. It uses a polymorphic `Exporter` trait with a builder pattern for configuration.

## Exporter Trait

```rust
pub trait Exporter {
    /// Export the book to the provided writer
    fn export<W: Write + Seek>(&self, book: &mut Book, writer: &mut W) -> io::Result<()>;
}
```

## Builder Pattern

Exporters use a builder pattern for configuration:

```rust
// Create exporter with defaults
let exporter = EpubExporter::new();

// Create exporter with configuration
let exporter = EpubExporter::new()
    .with_version(EpubVersion::Epub3)
    .with_compression(Compression::Deflate);

// Export
exporter.export(&mut book, &mut output)?;
```

## Export Flow

```
Book → Exporter::export()
        ↓
    Normalize content (if needed)
        ↓
    For each chapter:
        • Check if raw available
        • Use raw if same-format and compatible
        • Otherwise use IR (synthesize HTML)
        ↓
    Generate output format
        ↓
    Write to output
```

## Normalized vs Raw Export

### Raw Export

Uses original content bytes when possible:

```rust
// EPUB → EPUB: use raw HTML
if book.format() == Format::Epub && output_format == Format::Epub {
    let raw = book.load_raw(chapter_id)?;
    // Write raw bytes directly
}
```

**Benefits**:
- Preserves original formatting
- Faster (no re-serialization)
- Lossless for same-format conversion

### Normalized Export

Uses IR to generate output:

```rust
// KFX → EPUB: use IR
let chapter = book.load_chapter_cached(chapter_id)?;
let html = synthesize_html(&chapter);
// Write generated HTML
```

**Benefits**:
- Enables cross-format conversion
- Applies consistent formatting
- Supports format-specific features

## Format-Specific Exporters

### EPUB Exporter

**Location**: `src/export/epub.rs`

**Configuration**:
```rust
pub struct EpubConfig {
    pub version: EpubVersion,
    pub compression: Compression,
}

pub enum EpubVersion {
    Epub2,
    Epub3,
}

pub enum Compression {
    None,
    Deflate,
    Store,
}
```

**Export Process**:
1. Create ZIP archive structure
2. Write `mimetype` (uncompressed)
3. Write `container.xml`
4. Write `content.opf` with metadata
5. Write `toc.ncx` with navigation
6. For each chapter:
   - Use raw if available (EPUB → EPUB)
   - Otherwise synthesize HTML from IR
7. Copy assets (images, fonts, CSS)
8. Write ZIP central directory

**Key Implementation Details**:
- Uses `zip` crate with `flate2` for compression
- Generates proper EPUB structure
- Supports both EPUB 2 and 3
- Preserves extended metadata

### KFX Exporter

**Location**: `src/export/kfx.rs`

**Configuration**:
```rust
pub struct KfxConfig {
    pub encoding: String,  // "utf-8"
}
```

**Export Process**:
1. Normalize all chapters to IR
2. Extract style registry with deduplication
3. Generate KFX tokens from IR
4. Write Ion binary documents
5. Create KFX container structure
6. Write metadata fragment
7. Write content fragments
8. Write auxiliary data (landmarks, cover)

**Key Implementation Details**:
- Uses `kfx/context.rs` for state management
- Uses `kfx/style_registry.rs` for style deduplication
- Uses `kfx/schema.rs` for IR → KFX mapping
- Uses `kfx/serialization.rs` for binary format
- Supports latest KFX format

**Style Deduplication**:

```rust
// Build style registry
let mut registry = StyleRegistry::new();

for chapter in chapters {
    for node in chapter.nodes {
        let style = chapter.styles.get(node.style)?;
        registry.register(style)?;
    }
}

// Assign style IDs
let style_id = registry.get_id(style)?;
```

### AZW3 Exporter

**Location**: `src/export/azw3.rs`

**Configuration**:
```rust
pub struct Azw3Config {
    pub compression: Compression,
}
```

**Export Process**:
1. Create MOBI skeleton structure
2. Write MOBI headers
3. For each chapter:
   - Use raw if available (AZW3 → AZW3)
   - Otherwise transform IR to MOBI format
4. Write resource records (images, fonts)
5. Build indexes (NCX, INDX)
6. Write FDST (flow data) for pagination
7. Write PalmDB header and records

**Key Implementation Details**:
- Uses `mobi/skeleton.rs` for structure
- Uses `mobi/writer_transform.rs` for IR → MOBI
- Supports both compressed and uncompressed output
- Generates proper index structures

### Markdown Exporter

**Location**: `src/export/text.rs`

**Configuration**:
```rust
pub struct MarkdownConfig {
    pub title: bool,
    pub toc: bool,
    pub code_block_style: CodeBlockStyle,
}

pub enum CodeBlockStyle {
    Fenced,  // ```language
    Indented,  // 4-space indent
}
```

**Export Process**:
1. Write title (if enabled)
2. Write TOC (if enabled)
3. For each chapter:
   - Traverse IR nodes
   - Convert to Markdown syntax
   - Handle lists, code blocks, tables
4. Write to output

**Key Implementation Details**:
- Uses `markdown/` module for rendering
- Preserves headings hierarchy
- Handles inline formatting (bold, italic, links)
- Supports code blocks with language hints

## HTML Synthesis

When exporting from IR (normalized export), HTML is synthesized:

```rust
pub fn synthesize_html(chapter: &Chapter) -> String {
    let mut buf = String::new();
    synthesize_html_with_class_list(chapter, &mut buf);
    buf
}
```

### Synthesis Process

1. Start with `<!DOCTYPE html>` and `<html>` root
2. For each node in DFS order:
   - Get element tag name from role
   - Generate class list from style
   - Write semantic attributes (href, src, alt, id)
   - Write inline styles (if needed)
   - Recursively process children
3. Close all tags

### Class List Generation

```rust
fn generate_class_list(style: &ComputedStyle) -> Vec<String> {
    // Generate CSS class names based on style properties
    // e.g., ["bold", "italic", "text-align-center"]
}
```

### Style Generation

```rust
pub fn generate_css(chapter: &Chapter) -> String {
    // Generate CSS rules for all unique styles
    // Maps class names to CSS properties
}
```

## Metadata Export

### EPUB Metadata

```rust
// Dublin Core
write_dc_element("dc:title", &metadata.title)?;
write_dc_element("dc:creator", &metadata.authors.join(", "))?;
write_dc_element("dc:language", &metadata.language)?;
write_dc_element("dc:identifier", &metadata.identifier)?;

// EPUB3 Extensions
write_meta_element("dcterms:modified", &metadata.modified_date)?;

// Contributors
for contributor in &metadata.contributors {
    write_dc_element_with_role(
        "dc:contributor",
        &contributor.name,
        &contributor.role,
    )?;
}

// Collection
if let Some(ref collection) = metadata.collection {
    write_meta_element(
        "belongs-to-collection",
        &collection.name,
        &[("collection-type", &collection.collection_type)],
    )?;
}
```

### KFX Metadata

```rust
// Write metadata fragment
let metadata_fragment = IonWriter::new();
metadata_fragment.write_string("title", &metadata.title)?;
metadata_fragment.write_list("authors", &metadata.authors)?;
metadata_fragment.write_string("language", &metadata.language)?;
// ...
```

## Asset Export

### Images

```rust
for asset_path in book.list_assets() {
    if is_image(asset_path) {
        let data = book.load_asset(asset_path)?;
        // Write to output archive
    }
}
```

### Fonts

```rust
let font_faces = book.font_faces();
for font_face in font_faces {
    let font_data = book.load_asset(Path::new(&font_face.src))?;
    // Write font file
    // Create @font-face rule
}
```

### CSS

```rust
// For EPUB export: include original CSS
// For KFX export: include in style registry
// For Markdown export: skip
```

## Link Resolution

During export, links are resolved to proper targets:

```rust
if let Some(href) = chapter.semantics.href(node_id) {
    match book.resolve_href(chapter_id, href)? {
        AnchorTarget::External(url) => {
            // Write external URL
            write_attr("href", &url)?;
        }
        AnchorTarget::Internal(target) => {
            // Write internal link: path#fragment
            let href = format!("{}#{}",
                book.source_id(target.chapter)?,
                get_fragment_id(target.node)?
            );
            write_attr("href", &href)?;
        }
        AnchorTarget::Chapter(target_chapter) => {
            // Write chapter link: path
            let href = book.source_id(target_chapter)?;
            write_attr("href", href)?;
        }
    }
}
```

## Table of Contents Export

### EPUB NCX

```rust
fn write_ncx(toc: &[TocEntry], writer: &mut XmlWriter) {
    writeln!(writer, r#"<?xml version="1.0"?>"#)?;
    writeln!(writer, r#"<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/">"#)?;

    write_nav_map(toc, writer)?;

    writeln!(writer, "</ncx>")?;
}

fn write_nav_map(entries: &[TocEntry], writer: &mut XmlWriter) {
    for (idx, entry) in entries.iter().enumerate() {
        writeln!(writer, r#"<navPoint id="nav{}" playOrder="{}">"#, idx, idx)?;
        writeln!(writer, r#"<navLabel><text>{}</text></navLabel>"#, escape_xml(&entry.title))?;
        writeln!(writer, r#"<content src="{}"/>"#, escape_xml(&entry.href))?;
        write_nav_map(&entry.children, writer)?;
        writeln!(writer, "</navPoint>")?;
    }
}
```

### KFX Landmarks

```rust
fn write_landmarks(landmarks: &[Landmark], writer: &mut IonWriter) {
    for landmark in landmarks {
        writer.write_struct("landmark")?;
        writer.write_string("type", format!("{:?}", landmark.landmark_type))?;
        writer.write_string("href", &landmark.href)?;
        writer.write_string("label", &landmark.label)?;
        writer.end_struct()?;
    }
}
```

## Performance Considerations

1. **Use raw export when possible**: Faster and lossless
2. **Cache IR chapters**: Avoid re-parsing during export
3. **Batch style operations**: Deduplicate styles before export
4. **Reuse buffers**: Pre-allocate buffers for HTML synthesis
5. **Compress efficiently**: Use appropriate compression levels

## Adding a New Exporter

1. Create `src/export/your_format.rs`
2. Implement `Exporter` trait
3. Create config struct (if needed)
4. Add to `src/export/mod.rs`
5. Wire up in `Book::export()`

```rust
// src/export/mod.rs
pub use your_format::{YourFormatConfig, YourFormatExporter};

// src/model/book.rs
impl Book {
    pub fn export<W>(&mut self, format: Format, writer: &mut W) -> io::Result<()>
    where
        W: Write + Seek,
    {
        match format {
            Format::YourFormat => YourFormatExporter::new().export(self, writer),
            // ...
        }
    }
}
```

## Export Best Practices

1. **Check for raw content first**: Use `book.load_raw()` for same-format export
2. **Use cached IR**: Call `book.resolve_links()` before export for caching
3. **Handle encoding properly**: Support UTF-8 with BOM if needed
4. **Validate output**: Ensure generated files are valid
5. **Preserve metadata**: Map IR metadata to format-specific fields
6. **Generate proper structure**: Follow format specifications (ZIP, XML, etc.)
