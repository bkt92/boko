# Boko Architecture

## Overview

Boko is a high-performance ebook conversion library written in Rust. It converts between EPUB, KFX (Kindle Format 10), AZW3, and MOBI formats using an Intermediate Representation (IR) to preserve content, styles, and structure.

## Design Philosophy

1. **Format-Agnostic Core**: The IR (`model` module) is format-agnostic, enabling conversion between any supported formats.
2. **Two-Track Import**: Importers provide both raw bytes (for accurate conversion) and normalized IR (for rendering).
3. **Lazy Loading**: Large files are processed incrementally using the `ByteSource` trait.
4. **CSS Preservation**: Full CSS parsing and cascade ensures styles are preserved during conversion.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Book (Runtime Handle)                    │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Importer (format-specific backend)             │ │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐          │ │
│  │  │   EPUB     │  │    KFX     │  │   AZW3     │  MOBI    │ │
│  │  └────────────┘  └────────────┘  └────────────┘          │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    Intermediate Representation              │ │
│  │  • Nodes (tree structure with semantic roles)              │ │
│  │  • Styles (computed CSS properties per node)               │ │
│  │  • Semantics (href, src, alt, id attributes)               │ │
│  │  • Metadata (title, authors, contributors, etc.)           │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   Exporter (format-specific)                │ │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐          │ │
│  │  │   EPUB     │  │    KFX     │  │   AZW3     │  Markdown│ │
│  │  └────────────┘  └────────────┘  └────────────┘          │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Module Organization

### Core Modules

| Module | Purpose |
|--------|---------|
| `model` | Core IR data structures (`Book`, `Chapter`, `Node`, `Metadata`) |
| `import` | Format-specific importers (`Importer` trait) |
| `export` | Format-specific exporters (`Exporter` trait) |
| `dom` | HTML → IR compilation pipeline |
| `style` | CSS parsing, cascade, and property types |
| `io` | `ByteSource` trait for lazy loading |

### Format Modules

| Module | Format | Read | Write | Notes |
|--------|--------|------|-------|-------|
| `epub` | EPUB 2/3 | ✓ | ✓ | Full metadata support |
| `kfx` | Kindle Format 10 | ✓ | ✓ | Ion binary format |
| `mobi` | MOBI/AZW3 | ✓ | ✓ | PalmDOC database format |

### Utility Modules

**`image`** - Shared image processing
- `convert.rs`: Image format conversion, resizing, optimization
- Used by all exporters (EPUB, AZW3, MOBI 6)
- Configurable quality, dimensions, file size limits
- Supports PNG, JPEG, GIF, SVG, WebP formats

### Transform System

The `transforms/` directory provides standalone transforms used by external tools (like Calibre):

- `HtmlTransform`: DOM-based operations
- `AssetTransform`: Streaming operations

## Data Flow

### Import Flow

```
Input File → Importer::open() → Parse Structure
                                ↓
                    Metadata, TOC, Spine, Landmarks
                                ↓
                    load_raw() → Raw Bytes (high-fidelity)
                                ↓
                    load_chapter() → HTML + CSS
                                ↓
                    compile_html() → IR with computed styles
```

### Export Flow

```
Book → Exporter::export()
        ↓
    For each chapter:
        • If raw available: use raw bytes (EPUB → EPUB)
        • Otherwise: use IR (KFX → EPUB)
        ↓
    Write output format
```

### Link Resolution Flow

```
1. Load all chapters (cached via load_chapter_cached())
2. Build anchor map: path#fragment → GlobalNodeId
3. Resolve TOC hrefs to targets
4. Provide forward/reverse mappings for navigation
```

## Key Design Patterns

### 1. Two-Track Import

Importers implement both tracks:

```rust
pub trait Importer {
    // Track 1: Raw access for high-fidelity conversion
    fn load_raw(&mut self, id: ChapterId) -> io::Result<Vec<u8>>;

    // Track 2: Normalized IR for rendering
    fn load_chapter(&mut self, id: ChapterId) -> io::Result<Chapter>;
}
```

**Rationale**:
- Raw track preserves original content during same-format conversion
- Normalized track enables cross-format conversion with style application

### 2. Builder Pattern for Exporters

```rust
let exporter = EpubExporter::new()
    .with_version(EpubVersion::Epub3)
    .with_compression(Compression::Deflate);

exporter.export(&mut book, &mut output)?;
```

**Rationale**:
- Configuration is held separately from export logic
- Easy to add format-specific options

### 3. Arc<RwLock<Cache>> for Chapter Caching

```rust
pub struct Book {
    backend: Box<dyn Importer>,
    ir_cache: Arc<RwLock<HashMap<ChapterId, Arc<Chapter>>>>,
}
```

**Rationale**:
- `Arc` enables cheap cloning and thread-safe sharing
- `RwLock` allows concurrent reads during export
- Cache avoids re-parsing chapters during normalized export

### 4. Semantic Attribute Map

```rust
pub struct SemanticMap {
    href: HashMap<NodeId, String>,
    src: HashMap<NodeId, String>,
    alt: HashMap<NodeId, String>,
    id: HashMap<NodeId, String>,
}
```

**Rationale**:
- Keeps semantic data separate from node structure
- Enables efficient lookups during export
- Supports format-specific attribute requirements

## Format-Specific Notes

### KFX (Kindle Format 10)

KFX is Amazon's binary format using Ion (Amazon's JSON-like binary format):

- **Container**: ZIP with `.kfx` extension
- **Content**: Binary Ion documents
- **Metadata**: Separate metadata container
- **Styles**: Custom style registry with deduplication

Key modules:
- `kfx/ion.rs`: Ion parser/writer
- `kfx/schema.rs`: Bidirectional KFX ↔ IR mapping
- `kfx/context.rs`: Export context with state management
- `kfx/style_registry.rs`: Style deduplication

### MOBI/AZW3

MOBI is a legacy PalmDOC database format:

- **Container**: PDB database with multiple records
- **Compression**: HUFF/CDIC compression for text
- **Index**: Multi-level index structure
- **Images**: Embedded as separate records

Key modules:
- `mobi/parser.rs`: PDB header and record parsing
- `mobi/huffcdic.rs`: HUFF/CDIC decompression
- `mobi/index.rs`: Index traversal
- `mobi/transform.rs`: MOBI → IR transformation

### EPUB

EPUB is a ZIP-based HTML format:

- **Container**: ZIP with `.epub` extension
- **Structure**: `container.xml`, `content.opf`, `toc.ncx`
- **Content**: XHTML chapters with CSS
- **Metadata**: Dublin Core + EPUB3 extensions

Key modules:
- `epub/parser.rs`: OPF and NCX parsing
- `epub/mod.rs`: EPUB importer implementation

## Performance Considerations

1. **Lazy Loading**: `ByteSource` trait enables seeking without loading entire files
2. **Caching**: IR chapters are cached to avoid re-parsing
3. **Zero-Copy**: Use of `Arc` and `bytes::Bytes` where possible
4. **SIMD**: `memchr` and `bstr` for fast byte operations

## Testing Strategy

- **Unit tests**: Format-specific parsers and transformers
- **Property tests**: `proptest` for path resolution and encoding
- **Benchmarks**: `criterion` for performance regression testing
- **Integration tests**: Round-trip conversion tests

## Future Extensions

To add a new format:

1. Implement `Importer` trait in `import/your_format.rs`
2. Implement `Exporter` trait in `export/your_format.rs`
3. Add `Format` enum variant in `model/book.rs`
4. Wire up in `Book::open_format()` and `Book::export()`

No changes to core IR or model modules are required.
