# Markdown Import Design

**Date**: 2026-03-28
**Status**: Draft
**Author**: Claude (with user input)

## Overview

Add Markdown (`.md`) file import support to Boko, enabling conversion of Markdown documents to ebook formats (EPUB, KFX, AZW3, MOBI). The implementation follows Boko's two-track Importer trait pattern, converting Markdown elements into the Intermediate Representation (IR) used across all format conversions.

## Requirements

### Functional Requirements

1. **Import single Markdown files** - `Book::open("document.md")` loads and parses MD
2. **Split content by headings** - `#` level-1 headings create chapter breaks
3. **Extract embedded assets** - Images and media added to asset list for export
4. **Preserve code blocks** - Including Mermaid diagrams (kept as source, not rendered)
5. **Multi-dialect support** - CommonMark + GitHub Flavored Markdown + extensions
6. **Optional front matter** - Parse YAML metadata if present, with fallback inference

### Non-Functional Requirements

- Follow existing Importer trait pattern (two-track: raw + IR)
- Maintain consistency with other importers (EPUB, KFX, AZW3, MOBI)
- Support both single-file and split-by-heading workflows
- Graceful degradation for malformed MD (best-effort parsing)

## Architecture

### Data Flow

```
.md file → pulldown-cmark parser → Event stream → IR Node tree → Chapter(s)
                ↓
         Asset extraction (images, media)
                ↓
         Metadata extraction (front matter, headings)
                ↓
         TOC generation (heading hierarchy)
```

### Component Structure

```
src/import/
└── markdown.rs          # MarkdownImporter implementation
    ├── MarkdownImporter    # Main struct implementing Importer
    ├── MarkdownConfig      # Configuration options
    ├── Parser              # pulldown-cmark event handling
    ├── AssetExtractor      # Image/media collection
    ├── FrontMatter         # YAML parsing (optional)
    └── ToBuilder          # IR tree construction
```

### Key Structures

```rust
pub struct MarkdownImporter {
    path: PathBuf,
    content: String,           // Original MD content
    metadata: Metadata,
    toc: Vec<TocEntry>,
    spine: Vec<SpineEntry>,
    assets: HashMap<String, Vec<u8>>,
    chapter_offsets: Vec<ChapterRange>,  // Byte ranges for chapters
}

pub struct MarkdownConfig {
    pub split_on_h1: bool,           // Default: true
    pub extract_front_matter: bool,  // Default: true
    pub enable_gfm: bool,            // Default: true
    pub asset_base_dir: Option<PathBuf>,
}

struct ChapterRange {
    start: usize,
    end: usize,
    heading_level: u8,
}
```

## Importer Trait Implementation

### open(path: &Path)

1. Read MD file contents
2. Parse front matter (if present at top of file)
3. Run initial pass to identify:
   - All `#` headings (for chapter splitting)
   - Image/media references (for asset list)
   - Heading hierarchy (for TOC)
4. Store chapter byte ranges for efficient `load_raw()`
5. Build `Metadata` from front matter or inference
6. Return `MarkdownImporter` instance

### load_raw(id: ChapterId)

- Return original Markdown bytes for the specified chapter
- Uses pre-computed `chapter_offsets` to slice content
- Enables high-fidelity access if needed

### load_chapter(id: ChapterId)

- Parse the chapter's Markdown section using pulldown-cmark
- Convert event stream to IR node tree
- Map MD elements to `Role` variants (see mapping table below)
- Return fully constructed `Chapter` with IR

### metadata(), toc(), spine()

- Return references to pre-computed data structures
- No re-parsing required

### list_assets(), load_asset(name)

- `list_assets()`: Return keys from `assets` HashMap
- `load_asset()`: Return asset bytes or error if missing

## IR Element Mapping

| Markdown Element | IR Role | Attributes/Notes |
|-----------------|---------|------------------|
| `# Heading` | `Role::Heading` | `level: 1-6` |
| `paragraph` | `Role::Paragraph` | children contain inline elements |
| `*emphasis*` | `Role::Emphasis` | italic style |
| `**strong**` | `Role::Strong` | bold style |
| `` `code` `` | `Role::Code` | inline code |
| ` ```lang ` | `Role::CodeBlock` | `language: "lang"` in attributes |
| `[text](url)` | `Role::Link` | `href: "url"` in attributes |
| `![alt](url)` | `Role::Image` | `src: "url"` in attributes |
| `- item` | `Role::ListItem` | nested in `Role::List` |
| `| table |` | `Role::Table` | rows/cells as children |
| `---` (hr) | `Role::ThematicBreak` | scene break |
| `mermaid` fence | `Role::CodeBlock` | `language: "mermaid"` |
| HTML block | `Role::RawHtml` | passed through |

## Chapter Splitting Logic

**Strategy**: Split on level-1 headings (`#`)

1. First pass: Scan document for all `#` headings
2. Record byte offsets and heading levels
3. Create `ChapterRange` for each section:
   - From `#` heading to next `#` heading (or EOF)
   - Content before first `#` becomes prologue chapter (optional)
4. Generate `SpineEntry` for each chapter in order

**Example**:
```markdown
# Chapter One
Content...

## Section 1.1
More content...

# Chapter Two
Content...
```

Results in:
- `ChapterId(0)`: "# Chapter One" (including ## subsection)
- `ChapterId(1)`: "# Chapter Two"

## Asset Management

### Image Extraction

During parsing, for each `![alt](path)`:
1. Extract `path` from image event
2. Resolve relative to MD file directory (or `asset_base_dir` config)
3. Read file contents into `assets` HashMap
4. Create `Role::Image` node with `src` attribute

### Media Handling

For HTML-embedded media or future MD extensions:
- `<video src="...">` → `Role::Media` node
- `<audio src="...">` → `Role::Media` node
- Asset loaded and added to asset list

### Asset Loading Errors

- Missing files: Log warning, create placeholder node
- Invalid paths: Return `io::Error` from `load_asset()`
- Exporters decide how to handle broken references

## Metadata Extraction

### Front Matter (Optional)

**Format**: YAML block between `---` delimiters at file start

```yaml
---
title: My Book
author: Jane Doe
description: A great book
date: 2026-03-28
---
```

**Parsing**:
1. Check if file starts with `---`
2. Extract content between `---` delimiters
3. Parse with `serde_yaml` (or basic key-value parser)
4. Map to `Metadata` fields:
   - `title` → `metadata.title`
   - `author` → `metadata.contributors`
   - `description` → `metadata.description`
   - `date` → `metadata.date`

**Fallback**: If front matter missing or parsing fails, infer from document:
- First `#` heading → title
- File name → title (if no heading)
- Empty/defaults for other fields

### TOC Generation

Extract all headings (`#` through `######`) into `TocEntry` tree:
- Level determines nesting depth
- `#` headings are top-level TOC entries
- `##`, `###` etc. are nested children

## Configuration

### MarkdownConfig Options

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `split_on_h1` | `bool` | `true` | Enable chapter splitting on `#` |
| `extract_front_matter` | `bool` | `true` | Parse YAML front matter |
| `enable_gfm` | `bool` | `true` | Enable GFM extensions (tables, etc.) |
| `asset_base_dir` | `Option<PathBuf>` | `None` | Base directory for resolving asset paths |

### Usage Example

```rust
use boko::{Book, Format};
use boko::import::MarkdownConfig;

let config = MarkdownConfig {
    split_on_h1: true,
    extract_front_matter: true,
    enable_gfm: true,
    asset_base_dir: Some(PathBuf::from("./assets")),
};

let mut book = Book::open_with_config("book.md", config)?;
book.export(Format::Epub, &mut File::create("book.epub")?)?;
```

## Dependencies

### Required

- `pulldown-cmark` (^0.12) - Event-based Markdown parser with GFM support
- `serde` (already in project) - Metadata serialization
- `serde_yaml` (^0.9) - Front matter parsing (optional feature)

### Optional (Future)

- `syntect` - Syntax highlighting for code blocks
- Extended parsers for math (LaTeX), footnotes, definition lists

## Error Handling

| Error Type | Handling |
|------------|----------|
| Invalid MD syntax | Log warning, continue parsing (best-effort) |
| Missing asset file | Log warning, create placeholder node |
| Malformed front matter | Fall back to inferred metadata |
| Invalid UTF-8 | Return `io::Error` from `open()` |
| Broken image links | Return error from `load_asset()`, exporter decides |

## Testing Strategy

### Unit Tests

- Individual MD element → IR node mapping
- Front matter parsing
- Chapter boundary detection
- Asset path resolution

### Integration Tests

- Full MD file → IR → export to EPUB/KFX/AZW3
- Verify output files are valid
- Check images are embedded correctly
- Confirm TOC structure

### Fixture Files

Create test fixtures in `tests/fixtures/markdown/`:
- `simple.md` - Basic paragraphs and headings
- `with_images.md` - Document with embedded images
- `with_front_matter.md` - YAML front matter
- `gfm_tables.md` - GitHub Flavored Markdown tables
- `mermaid.md` - Mermaid diagram code blocks
- `no_headings.md` - Edge case: no `#` headings

### Edge Cases

- Empty files
- Files without `#` headings (single chapter mode)
- Malformed YAML
- Broken image references
- Very large files (performance test)

## Implementation Phases

### Phase 1: Basic CommonMark (MVP)

- Importer trait scaffolding
- Parse headings, paragraphs, basic formatting
- Split by `#` headings
- Extract images
- Simple metadata inference (first heading as title)

### Phase 2: GFM + Front Matter

- Tables, strikethrough, task lists
- YAML front matter parsing
- Full metadata extraction
- TOC generation from heading hierarchy

### Phase 3: Extended Features

- Multi-dialect configuration
- Better asset error handling
- Extended MD syntax support (footnotes, math)
- Performance optimizations for large files

## Success Criteria

- ✅ Can import single `.md` files via `Book::open()`
- ✅ Content splits into chapters based on `#` headings
- ✅ Images extracted and embedded in exported ebooks
- ✅ Code blocks (including Mermaid) preserved in IR
- ✅ Front matter metadata extracted when present
- ✅ Can export to EPUB, KFX, AZW3, MOBI formats
- ✅ All tests pass
- ✅ No clippy warnings
- ✅ Code formatted

## Future Considerations

- **Multi-file projects**: Support importing directories of MD files with a manifest
- **Syntax highlighting**: Pre-format code blocks with syntect for HTML-based formats
- **Custom IR nodes**: Special handling for diagrams (convert Mermaid to images?)
- **MD output**: Export IR back to Markdown (round-trip conversion)
- **Performance**: Streaming parser for very large files
