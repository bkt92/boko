# Boko Documentation

Welcome to the Boko ebook conversion library documentation.

## About Boko

Boko is a high-performance Rust library for converting between ebook formats (EPUB, KFX, AZW3, MOBI). It uses an Intermediate Representation (IR) to preserve content, styles, and structure during conversion.

**Key Features**:
- Multi-format support (EPUB 2/3, KFX, AZW3, MOBI)
- CSS preservation with full cascade
- Extended EPUB3 metadata support
- Lazy loading for efficient memory usage
- WebAssembly support via `wasm` feature

## Quick Start

### Installation

```bash
cargo install boko
```

### Library Usage

```rust
use boko::Book;

// Open a book
let mut book = Book::open("input.epub")?;

// Read metadata
println!("Title: {}", book.metadata().title);

// Convert to another format
let mut output = std::fs::File::create("output.kfx")?;
book.export(boko::Format::Kfx, &mut output)?;
```

### CLI Usage

```bash
# Show book info
boko info book.epub

# Convert formats
boko convert book.epub book.kfx

# Dump IR for debugging
boko dump book.epub
```

## Documentation

### Core Concepts

| Document | Description |
|----------|-------------|
| [Architecture](architecture.md) | High-level architecture and design patterns |
| [Intermediate Representation](intermediate-representation.md) | IR data structures and node types |
| [Import System](import-system.md) | Reading ebooks from various formats |
| [Export System](export-system.md) | Writing ebooks to various formats |
| [Style System](style-system.md) | CSS parsing, cascade, and computed styles |

### Format Documentation

| Document | Description |
|----------|-------------|
| [KFX Format](kfx-format.md) | Kindle Format 10 (Ion binary format) |
| [MOBI/AZW3 Format](mobi-format.md) | Legacy Kindle formats |

### Additional Resources

| Resource | Description |
|----------|-------------|
| [README](../README.md) | Project overview and basic usage |
| [CLAUDE.md](../CLAUDE.md) | Developer guide for working with the codebase |

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Benchmarking

```bash
cargo bench --bench conversion
```

### Linting

```bash
# Check with Clippy
cargo clippy -- -D warnings

# Check formatting
cargo fmt -- --check

# Format code
cargo fmt
```

## Project Structure

```
boko/
├── src/
│   ├── model/          # Core IR data structures
│   ├── import/         # Format-specific importers
│   ├── export/         # Format-specific exporters
│   ├── dom/            # HTML → IR compilation
│   ├── style/          # CSS parsing and cascade
│   ├── io/             # ByteSource trait for lazy loading
│   ├── epub/           # EPUB format support
│   ├── kfx/            # KFX format support
│   ├── mobi/           # MOBI/AZW3 format support
│   └── main.rs         # CLI entry point
├── transforms/         # Standalone transforms (external tools)
├── benches/            # Benchmarks
├── examples/           # Example code
└── docs/               # This documentation
```

## Module Overview

### Core Modules

**`model`** - Format-agnostic data structures
- `Book`: Runtime handle for ebooks
- `Chapter`: IR tree with nodes and styles
- `Node`: Individual tree nodes with semantic roles
- `Metadata`: Extended EPUB3 metadata
- `TocEntry`: Hierarchical table of contents

**`import`** - Format-specific importers
- `Importer` trait for reading ebooks
- Two-track design (raw + normalized)
- EPUB, KFX, AZW3, MOBI support

**`export`** - Format-specific exporters
- `Exporter` trait for writing ebooks
- Builder pattern for configuration
- EPUB, KFX, AZW3, Markdown support

**`dom`** - HTML → IR compilation
- `compile_html()`: Parse HTML to IR
- `html5ever` integration
- CSS cascade and style application
- Tree optimization passes

**`style`** - CSS system
- CSS parsing via `cssparser`
- Property types (Color, Length, etc.)
- Cascade implementation
- `ComputedStyle` and `StylePool`

**`io`** - I/O abstraction
- `ByteSource` trait for lazy loading
- `FileSource`: Memory-mapped files
- `MemorySource`: In-memory bytes

### Format Modules

**`epub`** - EPUB format
- ZIP archive parsing
- OPF and NCX parsing
- Extended metadata support

**`kfx`** - Kindle Format 10
- Ion binary format parser
- Style deduplication
- Token-based transformation

**`mobi`** - MOBI/AZW3 formats
- PalmDB parsing
- HUFF/CDIC decompression
- Index traversal

## Key Concepts

### Intermediate Representation (IR)

The IR is a format-agnostic tree structure that captures:
- Semantic structure (headings, paragraphs, lists, etc.)
- Computed CSS styles
- Semantic attributes (href, src, alt, id)
- Text content

### Two-Track Import

Importers provide two tracks:
1. **Raw**: Original content bytes (fast, lossless)
2. **Normalized**: Parsed IR with styles (enables conversion)

### Style Deduplication

Styles are deduplicated in a `StylePool` to reduce memory:
- Similar nodes share the same `StyleId`
- Critical for KFX export
- Reduces IR size significantly

### Link Resolution

Links are resolved in phases:
1. Index anchors: `path#fragment` → `GlobalNodeId`
2. Resolve TOC hrefs to targets
3. Provide forward/reverse mappings

### Lazy Loading

The `ByteSource` trait enables efficient seeking:
- Don't load entire file into memory
- Random access via `read_at()`
- Critical for MOBI format

## Contributing

We welcome contributions! Please:

1. Check existing issues for what to work on
2. Fork the repository and create a branch
3. Add tests for new features
4. Ensure all tests pass: `cargo test`
5. Run Clippy: `cargo clippy -- -D warnings`
6. Format code: `cargo fmt`
7. Submit a pull request

### Adding Format Support

To add a new format:

1. Implement `Importer` trait in `src/import/your_format.rs`
2. Implement `Exporter` trait in `src/export/your_format.rs`
3. Add `Format` enum variant in `src/model/book.rs`
4. Wire up in `Book::open_format()` and `Book::export()`
5. Add tests and documentation

No changes to core IR are required!

## License

MIT License - see LICENSE file for details.

## Resources

- [crates.io](https://crates.io/crates/boko)
- [docs.rs](https://docs.rs/boko)
- [GitHub](https://github.com/zacharydenton/boko)
- [Issues](https://github.com/zacharydenton/boko/issues)

## Changelog

See [GitHub Releases](https://github.com/zacharydenton/boko/releases) for version history.
