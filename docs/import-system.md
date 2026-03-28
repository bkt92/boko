# Import System

The import system is responsible for reading ebook files and providing access to their content. It uses a polymorphic `Importer` trait to support multiple formats.

## Importer Trait

The `Importer` trait defines the interface for format-specific readers:

```rust
pub trait Importer: Send + Sync {
    // Lifecycle
    fn open(path: &Path) -> io::Result<Self> where Self: Sized;

    // Metadata
    fn metadata(&self) -> &Metadata;
    fn toc(&self) -> &[TocEntry];
    fn landmarks(&self) -> &[Landmark];
    fn spine(&self) -> &[SpineEntry];

    // Track 1: Raw access (high-fidelity conversion)
    fn source_id(&self, id: ChapterId) -> Option<&str>;
    fn load_raw(&mut self, id: ChapterId) -> io::Result<Vec<u8>>;

    // Track 2: Normalized access (IR with styles)
    fn load_chapter(&mut self, id: ChapterId) -> io::Result<Chapter>;

    // Assets
    fn list_assets(&self) -> &[PathBuf];
    fn load_asset(&mut self, path: &Path) -> io::Result<Vec<u8>>;

    // Fonts
    fn font_faces(&mut self) -> Vec<FontFace>;

    // Link resolution
    fn index_anchors(&mut self, chapters: &[(ChapterId, Arc<Chapter>)]);
    fn resolve_toc(&mut self);
    fn resolve_href(&self, from_chapter: ChapterId, href: &str) -> Option<AnchorTarget>;
    fn toc_mut(&mut self) -> &mut [TocEntry];
}
```

## Two-Track Design

### Track 1: Raw Access

Returns original content bytes for high-fidelity conversion:

```rust
let raw_bytes = book.load_raw(chapter_id)?;

// Use case: EPUB → EPUB conversion preserves original HTML
```

**Benefits**:
- Preserves original encoding and formatting
- Enables lossless same-format conversion
- Fast (no parsing required)

### Track 2: Normalized Access

Returns parsed IR with computed styles:

```rust
let chapter = book.load_chapter(chapter_id)?;

// Use case: EPUB → KFX conversion with style application
```

**Benefits**:
- Format-agnostic representation
- Styles computed and applied
- Enables cross-format conversion

## Chapter Loading

### Default Implementation

The default `load_chapter()` implementation:

1. Loads raw HTML via `load_raw()`
2. Detects encoding from XML declaration
3. Extracts linked and inline stylesheets
4. Loads and parses linked CSS
5. Compiles HTML + CSS to IR
6. Resolves relative paths in semantic attributes

```rust
fn load_chapter(&mut self, id: ChapterId) -> io::Result<Chapter> {
    // 1. Load raw HTML
    let html_bytes = self.load_raw(id)?;

    // 2. Detect encoding
    let hint_encoding = crate::util::extract_xml_encoding(&html_bytes);
    let html_str = crate::util::decode_text(&html_bytes, hint_encoding);

    // 3. Extract stylesheets
    let (linked, inline) = extract_stylesheets(&html_str);

    // 4. Build stylesheets list
    let mut stylesheets = Vec::new();
    for href in linked {
        let css_path = resolve_relative_path(self.source_id(id)?, &href);
        if let Some(sheet) = self.load_stylesheet(&css_path) {
            stylesheets.push((sheet, Origin::Author));
        }
    }
    for css in inline {
        stylesheets.push((Stylesheet::parse(&css), Origin::Author));
    }

    // 5. Compile to IR
    let mut chapter = compile_html_bytes(&html_bytes, &stylesheets);

    // 6. Resolve semantic paths
    if let Some(base_path) = self.source_id(id) {
        resolve_semantic_paths(&mut chapter, base_path);
    }

    Ok(chapter)
}
```

### Caching

Chapters are cached to avoid re-parsing:

```rust
pub fn load_chapter_cached(&mut self, id: ChapterId) -> io::Result<Arc<Chapter>> {
    // Fast path: check cache
    {
        let cache = self.ir_cache.read()?;
        if let Some(chapter) = cache.get(&id) {
            return Ok(Arc::clone(chapter));
        }
    }

    // Slow path: load and cache
    let chapter = self.backend.load_chapter(id)?;
    let chapter_arc = Arc::new(chapter);

    {
        let mut cache = self.ir_cache.write()?;
        cache.insert(id, Arc::clone(&chapter_arc));
    }

    Ok(chapter_arc)
}
```

## Asset Loading

### Listing Assets

```rust
let assets = book.list_assets();
for asset in assets {
    println!("Asset: {}", asset.display());
}
```

### Loading Assets

```rust
let image_data = book.load_asset(Path::new("OEBPS/images/cover.png"))?;
```

### Font Face Extraction

```rust
let font_faces = book.font_faces();
for font_face in font_faces {
    println!("Font family: {}", font_face.font_family);
    println!("Font src: {}", font_face.src);
}
```

This parses all CSS files and extracts `@font-face` rules with resolved paths.

## Link Resolution

### Phase 1: Index Anchors

Build a map of `path#fragment` → `GlobalNodeId`:

```rust
fn index_anchors(&mut self, chapters: &[(ChapterId, Arc<Chapter>)]) {
    // Build anchor map for format-specific resolution
    // Default: no-op (path-based resolution works for EPUB)
}
```

### Phase 2: Resolve TOC

Fix up TOC entries with fragment identifiers:

```rust
fn resolve_toc(&mut self) {
    // Fix up TOC hrefs (needed for AZW3/MOBI)
    // Default: no-op (EPUB/KFX have correct hrefs)
}
```

### Phase 3: Resolve Hrefs

Resolve individual hrefs to targets:

```rust
fn resolve_href(&self, from_chapter: ChapterId, href: &str) -> Option<AnchorTarget> {
    // External URLs
    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(AnchorTarget::External(href.to_string()));
    }

    // Fragment-only link (#anchor)
    if let Some(fragment) = href.strip_prefix('#') {
        let key = format!("{}#{}", from_path, fragment);
        if let Some(target) = anchor_map.get(&key) {
            return Some(AnchorTarget::Internal(*target));
        }
    }

    // Path with fragment (path.html#anchor)
    // Path without fragment (path.html)

    None  // Not resolved
}
```

### Path-Based Resolution

Helper for EPUB-style paths:

```rust
pub fn resolve_path_based_href(
    from_path: &str,
    href: &str,
    chapter_for_path: impl Fn(&str) -> Option<ChapterId>,
    anchor: impl Fn(&str) -> Option<GlobalNodeId>,
) -> Option<AnchorTarget> {
    // Fragment-only: #anchor
    // Path + fragment: path.html#anchor
    // Path only: path.html

    // Returns External, Internal, or Chapter target
}
```

## Format-Specific Importers

### EPUB Importer

**Location**: `src/import/epub.rs`

**Features**:
- Parses `container.xml` to find root file
- Parses `content.opf` for metadata, manifest, spine, and TOC
- Parses `toc.ncx` for hierarchical TOC
- Loads chapters and assets from ZIP archive

**Key Implementation Details**:
- Uses `zip` crate for archive access
- Supports EPUB 2 and 3
- Handles extended metadata (contributors, collections)

### KFX Importer

**Location**: `src/import/kfx.rs`

**Features**:
- Parses KFX ZIP container
- Reads Ion binary documents
- Extracts content, metadata, and styles
- Builds IR from KFX tokens

**Key Implementation Details**:
- Uses custom Ion parser (`kfx/ion.rs`)
- Handles KFX-specific features (storyline, fragments)
- Supports latest KFX format versions

### AZW3 Importer

**Location**: `src/import/azw3.rs`

**Features**:
- Parses AZW3/KF8 format (MOBI with EPUB content)
- Extracts HTML content from resource records
- Parses MOBI headers and indexes
- Supports both compressed and uncompressed records

**Key Implementation Details**:
- Reuses MOBI parser infrastructure
- Extracts EPUB-like content from MOBI container
- Handles resource index structures

### MOBI Importer

**Location**: `src/import/mobi.rs`

**Features**:
- Parses legacy MOBI format
- Handles HUFF/CDIC compression
- Extracts text and images from PalmDB records
- Builds TOC from NCX index

**Key Implementation Details**:
- Uses `ByteSource` for efficient seeking
- Handles variable-length integer encoding
- Supports MOBI 6-7 and KF8 formats

## Path Resolution

### Relative Path Resolution

Resolves `../styles/main.css` relative to `OEBPS/text/ch01.xhtml`:

```rust
fn resolve_relative_path(base: &str, relative: &str) -> PathBuf {
    // Handle absolute paths: /styles/main.css
    // Handle URLs: https://example.com/style.css
    // Handle fragments: #anchor

    // Normalize: ../ and ./
    // Return: OEBPS/styles/main.css
}
```

### Semantic Path Resolution

Canonicalizes paths in semantic attributes:

```rust
fn resolve_semantic_paths(chapter: &mut Chapter, base_path: &str) {
    chapter.semantics.resolve_paths(|path| {
        // Skip external URLs and data URIs
        if path.contains("://") || path.starts_with("data:") {
            return path.to_string();
        }

        // Resolve relative path to absolute archive path
        let resolved = resolve_relative_path(base_path, path);

        // Normalize to forward slashes
        resolved.to_string_lossy().replace('\\', "/")
    });
}
```

## ByteSource Trait

Enables efficient random access for large files:

```rust
pub trait ByteSource: Send + Sync {
    /// Read bytes at offset
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;

    /// Get total length
    fn len(&self) -> u64;

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
```

**Implementations**:
- `FileSource`: Memory-mapped file access
- `MemorySource`: In-memory bytes (for stdin)
- `PdbSource`: PalmDB record-based access

## Importer Best Practices

1. **Use raw track for same-format conversion**: Preserves original content
2. **Use normalized track for cross-format conversion**: Applies styles and transforms
3. **Cache parsed chapters**: Avoid re-parsing during export
4. **Resolve paths early**: Canonicalize paths during import
5. **Index anchors before export**: Enables link resolution
6. **Handle encoding detection**: Support UTF-8, Windows-1252, etc.

## Adding a New Importer

1. Create `src/import/your_format.rs`
2. Implement `Importer` trait
3. Add to `src/import/mod.rs`
4. Wire up in `Book::open_format()`

```rust
// src/import/mod.rs
pub use your_format::YourFormatImporter;

// src/model/book.rs
impl Book {
    pub fn open_format(path: &Path, format: Format) -> io::Result<Self> {
        let backend: Box<dyn Importer> = match format {
            Format::YourFormat => Box::new(YourFormatImporter::open(path)?),
            // ...
        };
        // ...
    }
}
```
