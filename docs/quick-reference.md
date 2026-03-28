# Boko Quick Reference

Quick reference for common Boko tasks.

## CLI Commands

### Show Book Info

```bash
# Human-readable
boko info book.epub

# JSON output
boko info --json book.epub
```

### Convert Formats

```bash
# EPUB to KFX
boko convert book.epub book.kfx

# EPUB to AZW3
boko convert book.epub book.azw3

# KFX to EPUB
boko convert book.kfx book.epub

# To Markdown
boko convert book.epub book.md

# From stdin
cat book.epub | boko convert - -f epub -t md
```

### Dump IR

```bash
# Dump all chapters
boko dump book.epub

# Structure only
boko dump -s book.epub

# Specific chapter
boko dump -c 0 book.epub

# Style pool only
boko dump --styles-only book.epub

# JSON output
boko dump --json book.epub

# With expanded styles
boko dump --styles book.epub

# Limit depth
boko dump --depth 2 book.epub
```

### Sections

```bash
# Extract section tree as JSON
boko sections book.epub
```

## Library Usage

### Open Book

```rust
use boko::Book;

// Auto-detect format
let mut book = Book::open("input.epub")?;

// Explicit format
let mut book = Book::open_format("input.azw3", boko::Format::Azw3)?;

// From bytes
let data = std::fs::read("input.epub")?;
let mut book = Book::from_bytes(&data, boko::Format::Epub)?;
```

### Read Metadata

```rust
let meta = book.metadata();

println!("Title: {}", meta.title);
println!("Authors: {}", meta.authors.join(", "));
println!("Language: {}", meta.language);

// Extended metadata
if let Some(ref desc) = meta.description {
    println!("Description: {}", desc);
}

for contrib in &meta.contributors {
    println!("Contributor: {} ({:?})", contrib.name, contrib.role);
}

if let Some(ref coll) = meta.collection {
    println!("Series: {} #{}", coll.name, coll.position.unwrap_or(0.0));
}
```

### Navigate Chapters

```rust
// Get spine (reading order)
let spine: Vec<_> = book.spine().to_vec();

for entry in spine {
    let source = book.source_id(entry.id).unwrap_or("?");
    println!("Chapter {}: {} ({} bytes)", entry.id.0, source, entry.size_estimate);
}
```

### Load Content

```rust
// Raw bytes (fast, lossless)
let raw = book.load_raw(chapter_id)?;

// IR with styles (for analysis/conversion)
let chapter = book.load_chapter(chapter_id)?;

// Cached IR (reuses parsed chapter)
let chapter = book.load_chapter_cached(chapter_id)?;
```

### Traverse IR

```rust
let chapter = book.load_chapter(chapter_id)?;

// Depth-first iteration
for node_id in chapter.iter_dfs() {
    let node = chapter.node(node_id)?;

    match node.role {
        boko::Role::Heading(level) => {
            println!("Heading {}: {}", level, chapter.text(node.text));
        }
        boko::Role::Paragraph => {
            println!("Paragraph");
        }
        _ => {}
    }
}
```

### Access Styles

```rust
let node = chapter.node(node_id)?;
let style = chapter.styles.get(node.style)?;

println!("Color: {:?}", style.color);
println!("Font: {} {:?}", style.font_family, style.font_weight);
println!("CSS: {}", style.to_css_string());
```

### Access Semantics

```rust
// Get link href
if let Some(href) = chapter.semantics.href(node_id) {
    println!("Link: {}", href);
}

// Get image src
if let Some(src) = chapter.semantics.src(node_id) {
    println!("Image: {}", src);
}

// Get alt text
if let Some(alt) = chapter.semantics.alt(node_id) {
    println!("Alt: {}", alt);
}

// Get element ID
if let Some(id) = chapter.semantics.id(node_id) {
    println!("ID: {}", id);
}
```

### Export

```rust
use std::fs::File;

// To file
let mut output = File::create("output.kfx")?;
book.export(boko::Format::Kfx, &mut output)?;

// To memory
let mut buffer = Vec::new();
book.export(boko::Format::Epub, &mut buffer)?;
```

### Link Resolution

```rust
// Resolve all links
let resolved = book.resolve_links()?;

// Check broken links
for (source, href) in resolved.broken_links() {
    eprintln!("Broken link at {:?}: {}", source, href);
}

// Get forward mapping
if let Some(target) = resolved.get_target(source_node_id) {
    match target {
        boko::AnchorTarget::External(url) => println!("External: {}", url),
        boko::AnchorTarget::Internal(node_id) => println!("Internal: {:?}", node_id),
        boko::AnchorTarget::Chapter(chapter_id) => println!("Chapter: {:?}", chapter_id),
    }
}
```

## DOM Compilation

### Compile HTML

```rust
use boko::dom::{compile_html, Stylesheet, Origin};

let html = r#"
    <html>
    <head>
        <style>p { color: blue; }</style>
    </head>
    <body>
        <p>Hello, world!</p>
    </body>
    </html>
"#;

// Parse inline CSS
let inline_css = Stylesheet::parse("p { color: blue; }");

// Compile to IR
let chapter = compile_html(html, &[(inline_css, Origin::Author)]);
```

### Compile from Bytes

```rust
use boko::dom::compile_html_bytes;

let html_bytes = std::fs::read("chapter.xhtml")?;
let chapter = compile_html_bytes(&html_bytes, &[])?;
```

### Extract Stylesheets

```rust
use boko::dom::extract_stylesheets;

let (linked, inline) = extract_stylesheets(html);

// linked: Vec<String> of stylesheet hrefs
// inline: Vec<String> of inline CSS
```

## Style System

### Parse CSS

```rust
use boko::dom::Stylesheet;

let css = r#"
    p {
        color: blue;
        font-weight: bold;
    }
"#;

let stylesheet = Stylesheet::parse(css);
```

### Computed Style

```rust
use boko::style::{ComputedStyle, Length, Color, FontWeight};

let style = ComputedStyle {
    color: Color::Rgb(0, 0, 255),
    font_size: Length::Px(16.0),
    font_weight: FontWeight::Bold,
    // ... other properties
    ..ComputedStyle::default()
};
```

### Serialize CSS

```rust
use boko::style::ToCss;

let css = style.to_css_string();
println!("CSS: {}", css);
```

## Exporter Configuration

### Image Processing (Shared)

```rust
use boko::image::{ImageConfig, ImageFormat, process_image};

// Configure image processing for export
let config = ImageConfig {
    max_dimensions: (2048, 2048),
    max_file_size: 10 * 1024 * 1024,  // 10MB
    output_format: ImageFormat::Auto,
    jpeg_quality: 85,
    png_compression: 6,
};

// Process image data
let (processed, warnings) = process_image(&image_data, &config)?;
```

### EPUB Exporter

```rust
use boko::export::{EpubExporter, EpubConfig, EpubVersion, Compression};

let exporter = EpubExporter::new()
    .with_version(EpubVersion::Epub3)
    .with_compression(Compression::Deflate);

exporter.export(&mut book, &mut output)?;
```

### KFX Exporter

```rust
use boko::export::{KfxExporter, KfxConfig};

let exporter = KfxExporter::new();
exporter.export(&mut book, &mut output)?;
```

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

### Markdown Exporter

```rust
use boko::export::{MarkdownExporter, MarkdownConfig, CodeBlockStyle};

let exporter = MarkdownExporter::new()
    .with_title(true)
    .with_toc(true)
    .with_code_block_style(CodeBlockStyle::Fenced);

exporter.export(&mut book, &mut output)?;
```

## Error Handling

```rust
use std::io;

fn convert_book() -> io::Result<()> {
    let mut book = Book::open("input.epub")?;

    let mut output = File::create("output.kfx")?;
    book.export(boko::Format::Kfx, &mut output)?;

    Ok(())
}

// Handle errors
if let Err(e) = convert_book() {
    eprintln!("Error: {}", e);
    std::process::exit(1);
}
```

## Utility Functions

### Format Detection

```rust
use boko::Format;

let format = Format::from_path("book.epub").unwrap();
match format {
    Format::Epub => println!("EPUB file"),
    Format::Kfx => println!("KFX file"),
    _ => println!("Other format"),
}
```

### Asset Loading

```rust
// List assets
for asset in book.list_assets() {
    println!("Asset: {}", asset.display());
}

// Load asset
let data = book.load_asset(std::path::Path::new("images/cover.jpg"))?;
```

### Font Faces

```rust
let font_faces = book.font_faces();

for font_face in font_faces {
    println!("Font: {} from {}", font_face.font_family, font_face.src);
}
```

## Common Patterns

### Process All Chapters

```rust
let spine: Vec<_> = book.spine().to_vec();

for entry in spine {
    let chapter = book.load_chapter_cached(entry.id)?;

    // Process chapter...
    let node_count = chapter.node_count();
    println!("Chapter {}: {} nodes", entry.id.0, node_count);
}
```

### Extract Text

```rust
let chapter = book.load_chapter(chapter_id)?;

let mut text = String::new();
for node_id in chapter.iter_dfs() {
    let node = chapter.node(node_id)?;
    if node.role == boko::Role::Text && !node.text.is_empty() {
        text.push_str(chapter.text(node.text));
        text.push(' ');
    }
}

println!("Text: {}", text);
```

### Count Elements

```rust
use boko::Role;
use std::collections::HashMap;

let mut counts = HashMap::new();

for node_id in chapter.iter_dfs() {
    let node = chapter.node(node_id)?;
    *counts.entry(node.role).or_insert(0) += 1;
}

for (role, count) in counts {
    println!("{:?}: {}", role, count);
}
```

## Debugging

### Dump Chapter

```rust
let chapter = book.load_chapter(chapter_id)?;

// Serialize to JSON
let json = serde_json::to_string_pretty(&chapter)?;
println!("{}", json);
```

### Inspect Style

```rust
let style = chapter.styles.get(style_id)?;
println!("Style: {}", style.to_css_string());
```

### Trace Compilation

```rust
// Enable logging
env_logger::init();

let chapter = compile_html(html, &stylesheets);
// Logs will show compilation steps
```

## Performance Tips

1. **Use cached IR**: `load_chapter_cached()` for repeated access
2. **Pre-resolve links**: `resolve_links()` before export
3. **Use raw export**: For same-format conversion
4. **Clear cache**: `clear_cache()` when done with IR
5. **Reuse buffers**: Pre-allocate for large operations

## See Also

- [Architecture](architecture.md) - High-level architecture
- [Intermediate Representation](intermediate-representation.md) - IR details
- [Import System](import-system.md) - Reading formats
- [Export System](export-system.md) - Writing formats
- [Style System](style-system.md) - CSS processing
