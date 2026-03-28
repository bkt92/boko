# Markdown Import

Boko supports importing Markdown (`.md`) files for conversion to ebook formats.

## Features

- **Chapter splitting**: Level-1 headings (`#`) create chapter breaks
- **Inline formatting**: Emphasis, strong, code via CSS styles
- **Images**: Embedded in exported ebooks
- **Links**: Internal and external links preserved
- **Code blocks**: Including Mermaid diagrams (preserved as source)
- **TOC generation**: Automatic from heading hierarchy

## Usage

```rust
use boko::Book;

// Open markdown file
let mut book = Book::open("document.md")?;

// Export to EPUB
book.export(boko::Format::Epub, &mut File::create("output.epub")?)?;
```

## Limitations

- Single-file import only (multi-file projects planned for Phase 3)
- Front matter parsing (YAML) planned for Phase 2
- GFM extensions (tables, strikethrough) planned for Phase 2
