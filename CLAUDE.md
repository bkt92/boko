# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Boko is a high-performance Rust ebook conversion library supporting EPUB, KFX (Kindle Format 10), AZW3, and MOBI formats. It uses an Intermediate Representation (IR) to enable accurate conversion between formats while preserving CSS, metadata, and document structure.

**Rust Edition**: This project uses Rust 2024 edition, which requires a recent Rust toolchain.

## Common Development Commands

```bash
# Build the project
cargo build --verbose

# Run tests
cargo test --verbose

# Run a specific test
cargo test -- test_name

# Run Clippy linting
cargo clippy -- -D warnings

# Check formatting
cargo fmt -- --check

# Format code
cargo fmt

# Build documentation
cargo doc --no-deps

# Run benchmarks
cargo bench --bench conversion
```

## High-Level Architecture

### Core Data Flow

```
Input Format → Importer → Raw Bytes / IR → Exporter → Output Format
```

The codebase follows a two-track architecture:

1. **Import Module** (`src/import/`): Format-specific readers implement the `Importer` trait
   - `load_raw()`: Returns original content bytes for high-fidelity conversion
   - `load_chapter()`: Returns normalized IR for rendering/analysis
   - Each format (EPUB, KFX, AZW3, MOBI) has its own importer

2. **Export Module** (`src/export/`): Format-specific writers implement the `Exporter` trait
   - Uses a builder pattern with config structs (e.g., `EpubConfig`, `KfxConfig`)
   - `export()` writes to any `Write + Seek` destination

3. **Model Module** (`src/model/`): Core IR data structures
   - `Book`: Runtime handle wrapping the active importer
   - `Chapter`: IR tree with nodes, styles, and semantic attributes
   - `Node`: Individual tree nodes with `Role` (Paragraph, Heading, Image, etc.)
   - `Metadata`: Extended EPUB3 metadata support (contributors, collections, etc.)

### DOM Compilation Pipeline (`src/dom/`)

The `compile_html()` function transforms HTML + CSS into normalized IR:

1. Parse HTML using `html5ever` into `ArenaDom`
2. Extract and parse stylesheets (linked + inline)
3. Apply CSS cascade using `cssparser` and `selectors` crates
4. Transform DOM tree to IR nodes with computed styles
5. Run optimization passes (merge adjacent text nodes, prune empty containers)

### Style System (`src/style/`)

- CSS parsing via `cssparser` crate
- Property types: `Color`, `Length`, `Display`, `FontWeight`, etc.
- `ComputedStyle`: Computed style for each node
- `StylePool`: Deduplicated style storage
- `compute_styles()`: CSS cascade implementation

### KFX Format (`src/kfx/`)

KFX is Amazon's binary format using Ion (Amazon's JSON-like binary format):

- `ion.rs`: Ion binary format parser/writer
- `container.rs`: KFX ZIP container structure
- `schema.rs`: Bidirectional KFX ↔ IR mapping rules
- `tokens.rs`: Token stream abstraction for import/export
- `context.rs`: Export context with central state management
- `style_registry.rs`: Style deduplication for KFX output

### Transform System (`transforms/`)

Standalone transforms for ebook processing (used by external tools):

- `HtmlTransform`: DOM-based operations (CSS flattening, margin removal, scene break detection)
- `AssetTransform`: Streaming operations (image conversion, font embedding)

## Key Design Patterns

### Importer Trait Two-Track Design

Importers provide both raw access (for accurate conversion) and normalized access (for analysis):

```rust
pub trait Importer {
    // Raw access - returns original content
    fn load_raw(&mut self, id: ChapterId) -> io::Result<Vec<u8>>;

    // Normalized access - returns IR with styles applied
    fn load_chapter(&mut self, id: ChapterId) -> io::Result<Chapter>;
}
```

### Lazy Loading with ByteSource

The `ByteSource` trait enables efficient random access into large files without loading everything into memory. Used by MOBI and other formats.

### Link Resolution

Internal links are resolved through a two-phase process:
1. `index_anchors()`: Builds a map of path#fragment → GlobalNodeId
2. `resolve_href()`: Resolves hrefs to `AnchorTarget` (Internal, External, or Chapter)

### Path Resolution

Helper functions in `import/mod.rs` and `dom/mod.rs` handle relative path resolution for archive contents (e.g., `../styles/main.css` relative to `OEBPS/text/ch01.xhtml`).

## Format Support Notes

- **MOBI**: Read-only legacy format. The MOBI parser handles the PalmDOC database format, HUFF/CDIC compression, and index structures.
- **KFX**: Full read/write support with metadata fidelity. The KFX format is complex due to its use of Ion binary serialization and separate content/metadata containers.
- **EPUB**: Full EPUB2/EPUB3 support with extended metadata (contributors, collections, refinements).
- **WASM**: The library can compile to WebAssembly via the `wasm` feature flag.

## Testing

- Unit tests use standard `#[test]` attributes
- Property-based tests use `proptest` crate (see `import/mod.rs` for examples)
- Benchmarks use `criterion` in `benches/conversion.rs`
- Test fixtures are typically minimal HTML/CSS snippets

## ByteSource and I/O

The `io` module provides the `ByteSource` trait for efficient random access. This is critical for MOBI format which stores content in a compressed Palm database format where seeking is expensive.
