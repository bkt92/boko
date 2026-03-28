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
    ├── IrBuilder           # IR tree construction from MD events
    └── LinkResolver        # MD-specific link resolution
```

### Key Structures

```rust
pub struct MarkdownImporter {
    path: PathBuf,
    content: String,                    // Original MD content
    metadata: Metadata,
    toc: Vec<TocEntry>,
    spine: Vec<SpineEntry>,
    assets: HashMap<String, Vec<u8>>,
    chapter_offsets: Vec<ChapterRange>, // Byte ranges for chapters
    anchor_map: HashMap<String, GlobalNodeId>, // For link resolution
    config: MarkdownConfig,
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
    virtual_path: String,  // e.g., "chapter-1.md"
}
```

## Importer Trait Implementation

### open(path: &Path)

1. Read MD file contents as UTF-8 string
2. Detect and extract front matter (if file starts with `---`)
3. Parse YAML metadata if present
4. Run initial scan pass:
   - Identify all `#` headings (for chapter splitting)
   - Collect image/media references (for asset list)
   - Build heading hierarchy (for TOC)
5. Store chapter byte ranges in `chapter_offsets`
6. Build `Metadata` from front matter or inference
7. Generate virtual source paths for each chapter (e.g., "chapter-1.md", "chapter-2.md")
8. Return `MarkdownImporter` instance

### load_raw(id: ChapterId)

- Return original Markdown bytes for the specified chapter
- Uses pre-computed `chapter_offsets[id.0 as usize]` to slice content
- Enables high-fidelity access and debugging

### source_id(id: ChapterId)

- Returns the virtual source path for the chapter
- Example: `Some("chapter-1.md")`, `Some("chapter-2.md")`
- These are NOT real files, but virtual paths used for:
  - Asset path resolution in `resolve_semantic_paths()`
  - Link resolution in `resolve_href()`
- All chapters share the same parent directory (the MD file's directory)

### load_chapter(id: ChapterId) - Override Required

**CRITICAL**: Must override the default implementation because:
- Default expects HTML and calls `compile_html_bytes()`
- Markdown produces event streams, not HTML
- Must manually construct `Chapter` struct

**Implementation steps**:
1. Get chapter's MD content via `chapter_offsets`
2. Parse with `pulldown-cmark` to get event iterator
3. Walk events, building IR tree (see IrBuilder section below)
4. Construct `Chapter` manually:
   - `nodes: Vec<Node>` - IR node tree
   - `text: String` - global text buffer
   - `styles: StylePool` - deduplicated styles
   - `semantics: SemanticMap` - href/src/alt attributes
5. Return completed `Chapter`

### metadata(), toc(), spine()

- Return references to pre-computed data structures
- No re-parsing required

### landmarks()

- Return empty slice `&[]`
- Markdown doesn't have EPUB-style landmarks (cover, start-of-book, etc.)

### toc_mut()

- Return mutable reference to `toc` field
- Used by `resolve_toc()` for fixing fragment identifiers

### font_faces()

- Return empty `Vec<FontFace>`
- Markdown has no embedded CSS with @font-face rules

### list_assets(), load_asset(path)

- `list_assets()`: Return keys from `assets` HashMap as `&[PathBuf]`
- `load_asset()`: Return asset bytes or `io::Error` if missing
- Assets are loaded during initial scan in `open()`

### index_anchors(chapters)

**Purpose**: Build anchor map for internal link resolution

**Implementation**:
1. Iterate through all loaded chapters
2. For each chapter, scan for heading nodes
3. Generate anchor IDs from heading text (slugify: lowercase, hyphens for spaces)
4. Map `virtual_path#anchor` → `GlobalNodeId`
5. Store in `anchor_map` for use in `resolve_href()`

**Example**:
```rust
// Chapter with virtual path "chapter-1.md"
// Contains: "# My Heading"
// Generates: "chapter-1.md#my-heading" → GlobalNodeId(chapter_id, node_id)
```

### resolve_toc()

- Walk through TOC entries
- Use `anchor_map` to resolve fragment identifiers
- Update `TocEntry.content_id` with resolved `GlobalNodeId`

### resolve_href(from_chapter, href)

**Purpose**: Resolve Markdown links to IR targets

**Link types in Markdown**:
1. `#anchor` - Fragment-only, same chapter
2. `path/to/file.md#anchor` - Cross-file reference (error for single-file MD)
3. `https://example.com` - External URL
4. `mailto:user@example.com` - External URL

**Implementation**:
```rust
fn resolve_href(&self, from_chapter: ChapterId, href: &str) -> Option<AnchorTarget> {
    let href = href.trim();

    // External URLs
    if href.starts_with("http://") || href.starts_with("https://")
        || href.starts_with("mailto:") || href.starts_with("tel:") {
        return Some(AnchorTarget::External(href.to_string()));
    }

    // Fragment-only link (#anchor) - resolve within same chapter
    if let Some(fragment) = href.strip_prefix('#') {
        let from_path = self.source_id(from_chapter)?;
        let key = format!("{}#{}", from_path, fragment);
        if let Some(target) = self.anchor_map.get(&key) {
            return Some(AnchorTarget::Internal(*target));
        }
        return None; // Broken link
    }

    // .md file reference (not supported in single-file mode)
    if href.contains(".md") {
        // Could return None or log warning
        // Future: Support multi-file MD projects
        return None;
    }

    None
}
```

## IR Element Mapping

### Block-Level Elements

| Markdown Element | IR Role | Notes |
|-----------------|---------|-------|
| `# Heading` | `Role::Heading(1)` | Level 1-6 stored in parameter |
| `paragraph` | `Role::Paragraph` | Contains inline children |
| ` ```lang ` | `Role::CodeBlock` | Language in `SemanticMap.language` |
| `> quote` | `Role::BlockQuote` | Nested blockquotes supported |
| `- list item` | `Role::ListItem` | Parented by `Role::UnorderedList` |
| `1. item` | `Role::ListItem` | Parented by `Role::OrderedList` |
| `| table |` | `Role::Table` | With `TableHead`, `TableRow`, `TableCell` children |
| `---` (hr) | `Role::Rule` | Thematic break |

### Inline Formatting (CRITICAL)

**Key Design Principle**: Boko's IR does **NOT** have separate roles for emphasis, strong, or inline code. These are handled via **CSS styles** in `ComputedStyle`.

| Markdown Element | IR Role | Style Applied |
|-----------------|---------|---------------|
| `*text*` / `_text_` | `Role::Inline` | `font-style: italic` |
| `**text**` / `__text__` | `Role::Inline` | `font-weight: bold` |
| `` `code` `` | `Role::Inline` | `font-family: monospace` |
| `~~text~~` | `Role::Inline` | `text-decoration: line-through` |
| Plain text | `Role::Text` | No special style |
| `[text](url)` | `Role::Link` | href in `SemanticMap.href` |
| `![alt](url)` | `Role::Image` | src/alt in `SemanticMap` |

**How to build styled inline nodes**:
```rust
// Example: *emphasis* text
let node = Node::new(Role::Inline);
node.style = styles.add(ComputedStyle {
    font_style: Some(FontStyle::Italic),
    // ... other properties
});
```

### Special Elements

| Markdown Element | IR Role | Handling |
|-----------------|---------|----------|
| Mermaid code block | `Role::CodeBlock` | `language: "mermaid"` in semantics |
| HTML block | `Role::Inline` | Parse with html5ever or skip |
| `<br>` | `Role::Break` | Line break |
| Footnote reference | `Role::Inline` | With `id` in semantics (future) |

## IR Tree Construction (IrBuilder)

### Algorithm Overview

The `IrBuilder` walks `pulldown-cmark` events and constructs the IR tree:

```rust
struct IrBuilder {
    nodes: Vec<Node>,
    text: String,
    styles: StylePool,
    semantics: SemanticMap,
    stack: Vec<NodeId>,  // Parent stack for nesting
    text_buffer: String, // Accumulates text for current text node
}
```

**Event Processing Loop**:
1. **Start(tag)**: Create new node, push to parent stack
2. **End(tag)**: Pop parent, flush text buffer to Text node
3. **Text(text)**: Append to `text_buffer`
4. **Code(code)**: Create Text node + Inline node with monospace style
5. **Html(html)**: Skip or parse as inline HTML
6. **InlineHtml(html)**: Skip or parse as inline
7. **SoftBreak/HardBreak**: Create `Role::Break` node
8. **Rule**: Create `Role::Rule` node

**Text Buffer Management**:
- Accumulate text events in `text_buffer`
- When encountering a non-text event, flush buffer as `Role::Text` node
- Add text to global `text` String, track byte range
- Create `Node::text(TextRange { start, len })`

**Style Application**:
- For emphasis/strong/code: Create `Role::Inline` node
- Add appropriate `ComputedStyle` to `StylePool`
- Set `node.style` to style ID
- Inline node contains Text children

### Example: Building `*emphasis*`

```rust
// Event sequence: Start(Emphasis), Text("hello"), End(Emphasis)

// Start(Emphasis)
let emphasis_id = NodeId(nodes.len());
nodes.push(Node::new(Role::Inline));
nodes[emphasis_id].style = styles.add(italic_style);
stack.push(emphasis_id);

// Text("hello")
text_buffer.push_str("hello");

// End(Emphasis) - flush text
let text_start = text.len();
text.push_str(&text_buffer);
let text_len = text_buffer.len();
text_buffer.clear();

let text_id = NodeId(nodes.len());
nodes.push(Node::text(TextRange::new(text_start, text_len)));
nodes[emphasis_id].first_child = Some(text_id);
nodes[text_id].parent = Some(emphasis_id);
stack.pop();
```

## Chapter Splitting Logic

**Strategy**: Split on level-1 headings (`#`)

### Algorithm

1. Scan document for all heading occurrences
2. Record byte offsets and heading levels
3. Create `ChapterRange` for each section:
   - From `#` heading to next `#` heading (or EOF)
4. Assign virtual paths: `"chapter-1.md"`, `"chapter-2.md"`, etc.

### Edge Cases

| Scenario | Behavior |
|----------|----------|
| No `#` headings | Single chapter with entire document, virtual path `"document.md"` |
| Content before first `#` | Prepend to first chapter as prologue (no heading) |
| `#` in code block | Don't split (context-aware: track code block state) |
| Empty sections between `#` | Create chapter with minimal content (just heading) |
| H1 → H6 → H1 | All create chapter breaks (only `#` matters for splitting) |
| Deeply nested headings | `##`, `###` etc. become sections within chapter |

### Example

```markdown
# Chapter One
Content...

## Section 1.1
More content...

# Chapter Two
Content...
```

Results in:
- `ChapterId(0)`: `"chapter-1.md"` - "# Chapter One" + ## subsection
- `ChapterId(1)`: `"chapter-2.md"` - "# Chapter Two"

## Asset Management

### Image Extraction Algorithm

```
For each ![alt](path) event:
1. Extract path from image event
2. Resolve path:
   a. If starts with "http://" or "https://" → External URL, skip
   b. If starts with "data:" → Inline data URI, skip
   c. If starts with "/" → Absolute from asset_base_dir or MD file dir
   d. Otherwise → Relative to MD file directory (or asset_base_dir if set)
3. Normalize path:
   - Convert backslashes to forward slashes
   - Resolve "." and ".." segments
   - Make absolute
4. Load file contents
5. Store in assets HashMap
6. Create Role::Image node with src/alt in SemanticMap
```

### Path Resolution Examples

| MD File | Image Path | Resolved To |
|---------|------------|-------------|
| `/docs/book.md` | `images/pic.png` | `/docs/images/pic.png` |
| `/docs/book.md` | `../assets/pic.png` | `/assets/pic.png` |
| `/docs/book.md` | `/static/pic.png` | `/static/pic.png` (if exists) |
| `/docs/book.md` | `https://example.com/pic.png` | External (skip) |

### Asset Loading Errors

| Error Type | Handling |
|------------|----------|
| File not found | Log warning, create placeholder Image node with broken src |
| Invalid UTF-8 path | Return `io::Error` from `load_asset()` |
| Permission denied | Return `io::Error` from `load_asset()` |

### Media Handling

For HTML-embedded media in Markdown:
- `<video src="...">` → Create `Role::Inline` with `src` in semantics
- `<audio src="...">` → Create `Role::Inline` with `src` in semantics
- Load and store in assets HashMap
- Exporters decide how to render (may not support all media types)

## Metadata Extraction

### Front Matter (Optional)

**Format**: YAML block between `---` delimiters at file start

```yaml
---
title: My Book
author: Jane Doe
description: A great book
date: 2026-03-28
language: en
publisher: My Publisher
rights: © 2026
identifier: ISBN-978-0-1234567-8-9
---
```

### Complete Field Mapping

| YAML Field | Metadata Field | Notes |
|------------|----------------|-------|
| `title` | `title` | Required |
| `author` | `contributors[0]` | `role: ContributorRole::Author` |
| `description` | `description` | Optional |
| `date` | `date` | ISO 8601 format |
| `language` | `language` | BCP 47 language tag |
| `publisher` | `publisher` | Optional |
| `rights` / `license` | `rights` | Copyright/license info |
| `identifier` / `isbn` | `identifier` | Unique identifier |
| `subject` / `tags` | `collections` | Tag-based collections |

**Parsing Steps**:
1. Check if file starts with `---`
2. Extract content between `---` delimiters
3. Parse with `serde_yaml` (or basic key-value parser)
4. Map fields to `Metadata` struct
5. Handle multiple authors: `authors: [Jane, John]`

### Fallback Inference

If front matter missing or parsing fails:
1. First `#` heading → `metadata.title`
2. File name (without extension) → `metadata.title` (if no heading)
3. Empty `String` for other fields
4. Default `language: "en"` if not specified

### TOC Generation

Extract all headings (`#` through `######`) into `TocEntry` tree:
- Level determines nesting depth
- `#` headings are top-level TOC entries
- `##`, `###` etc. are nested children
- Each TOC entry gets `content_id` resolved later in `resolve_toc()`

## Configuration

### MarkdownConfig Options

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `split_on_h1` | `bool` | `true` | Enable chapter splitting on `#` |
| `extract_front_matter` | `bool` | `true` | Parse YAML front matter |
| `enable_gfm` | `bool` | `true` | Enable GFM extensions |
| `asset_base_dir` | `Option<PathBuf>` | `None` | Base for resolving asset paths |

### API Design

**Option 1**: Extend `Book::open()` with config detection
```rust
// Uses default config
let book = Book::open("book.md")?;

// Uses custom config via new method
let book = Book::open_markdown("book.md", MarkdownConfig { ... })?;
```

**Option 2**: Add to `Format` enum
```rust
let format = Format::Markdown { config: Some(markdown_config) };
let book = Book::open_format("book.md", format)?;
```

**Recommendation**: Start with Option 1 (simpler), extend to Option 2 in Phase 3.

## Format Enum Integration

To enable Markdown import, update `src/model/book.rs`:

### 1. Update `Format::can_import()`

```rust
impl Format {
    pub fn can_import(&self) -> bool {
        matches!(
            self,
            Format::Epub | Format::Kfx | Format::Azw3 | Format::Mobi | Format::Markdown
        )
    }
}
```

### 2. Remove Error from `Book::open_format()`

Remove lines that prevent Markdown import:
```rust
Format::Markdown => {
    return Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Markdown import is not yet implemented",
    ));
}
```

### 3. Add Importer Match Case

```rust
let importer: Box<dyn Importer> = match format {
    // ... existing cases ...
    Format::Markdown => Box::new(MarkdownImporter::open(path)?),
    _ => {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cannot detect format automatically",
        ));
    }
};
```

## Dependencies

### Required

- `pulldown-cmark` (^0.12) - Event-based Markdown parser with GFM support
- `serde` (already in project) - Metadata serialization
- `serde_yaml` (^0.9) - Front matter parsing (optional feature "yaml-frontmatter")

### Optional (Phase 3)

- `syntect` - Syntax highlighting for code blocks
- Extended parsers for math (LaTeX), footnotes, definition lists

**Note**: Verify `pulldown-cmark 0.12` compatibility with Rust 2024 edition.

## Error Handling

| Error Type | Handling |
|------------|----------|
| Invalid MD syntax | Log warning, continue parsing (best-effort) |
| Missing asset file | Log warning, create placeholder node |
| Malformed front matter | Fall back to inferred metadata |
| Invalid UTF-8 | Return `io::Error` from `open()` |
| Broken image links | Return error from `load_asset()`, exporter decides |
| Circular link references | Detect and return `None` from `resolve_href()` |

## Testing Strategy

### Unit Tests

- Individual MD element → IR node mapping
- Front matter parsing (valid and invalid YAML)
- Chapter boundary detection (including edge cases)
- Asset path resolution (relative, absolute, URLs)
- Style application for inline formatting
- Link resolution (fragments, external, broken)

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn test_heading_splitting_preserves_structure(md in any_markdown_document()) {
        // Verify chapter boundaries don't split code blocks
        // Verify heading levels are preserved
        // Verify content order is maintained
    }

    #[test]
    fn test_inline_formatting_styles(text in any_inline_text()) {
        // Verify *emphasis* creates italic style
        // Verify **strong** creates bold style
        // Verify nested formatting works
    }
}
```

### Integration Tests

- Full MD file → IR → export to EPUB/KFX/AZW3
- Verify output files are valid (EPUB check, KFX structure)
- Check images are embedded correctly
- Confirm TOC structure matches heading hierarchy
- Test link resolution works within exported ebook

### Fixture Files

Create test fixtures in `tests/fixtures/markdown/`:
- `simple.md` - Basic paragraphs and headings
- `with_images.md` - Document with embedded images
- `with_front_matter.md` - YAML front matter (valid and invalid)
- `gfm_tables.md` - GitHub Flavored Markdown tables
- `mermaid.md` - Mermaid diagram code blocks
- `no_headings.md` - Edge case: no `#` headings
- `nested_formatting.md` - Complex inline formatting
- `links.md` - Various link types (fragments, external, broken)
- `large_file.md` - Performance testing (100KB+)

### Edge Cases

- Empty files
- Files without `#` headings
- `#` headings in code blocks (shouldn't split)
- Malformed YAML (incomplete, invalid syntax)
- Broken image references (missing files, invalid paths)
- Circular link references
- Very deeply nested heading hierarchies
- Mixed line endings (CRLF vs LF)
- Unicode and emoji in headings/text

## Implementation Phases

### Phase 1: Basic CommonMark (MVP)

**Goal**: Import simple MD documents and export to EPUB

- [ ] Importer trait scaffolding (all required methods)
- [ ] Parse headings, paragraphs, basic inline formatting
- [ ] Implement IrBuilder for IR tree construction
- [ ] Split by `#` headings with virtual paths
- [ ] Extract and resolve image assets
- [ ] Simple metadata inference (first heading as title)
- [ ] Basic TOC generation
- [ ] Override `load_chapter()` to build IR manually
- [ ] Implement `index_anchors()` and `resolve_href()`
- [ ] Update `Format` enum to support Markdown
- [ ] Unit tests for core functionality
- [ ] Integration test: MD → EPUB

**Acceptance**: Can import `simple.md` and export to valid EPUB with chapters and images.

### Phase 2: GFM + Front Matter

**Goal**: Full-featured MD import with metadata

- [ ] Enable GFM extensions (tables, strikethrough, task lists)
- [ ] YAML front matter parsing with `serde_yaml`
- [ ] Complete metadata field mapping
- [ ] TOC generation with full heading hierarchy
- [ ] Resolve TOC fragments in `resolve_toc()`
- [ ] Asset path resolution with `asset_base_dir` config
- [ ] Handle edge cases (no headings, malformed YAML)
- [ ] Extended test fixtures
- [ ] Property-based tests for parsing correctness

**Acceptance**: Can import complex MD with front matter, tables, and nested headings.

### Phase 3: Extended Features

**Goal**: Production-ready MD import with optimizations

- [ ] Multi-dialect configuration (disable GFM, custom extensions)
- [ ] Better asset error handling and recovery
- [ ] Extended MD syntax support (footnotes, definition lists, math)
- [ ] Performance optimizations for large files
- [ ] Streaming parser option for memory efficiency
- [ ] Configuration API (`Book::open_markdown_with_config()`)
- [ ] Comprehensive error messages
- [ ] Benchmark suite (parse time, memory usage)

**Acceptance**: Can import 100KB+ MD files in < 100ms with minimal memory overhead.

## Success Criteria

- ✅ Can import single `.md` files via `Book::open()`
- ✅ Content splits into chapters based on `#` headings
- ✅ Images extracted and embedded in exported ebooks
- ✅ Code blocks (including Mermaid) preserved in IR
- ✅ Front matter metadata extracted when present
- ✅ Can export to EPUB, KFX, AZW3, MOBI formats
- ✅ Inline formatting (emphasis, strong, code) uses CSS styles correctly
- ✅ Internal links resolve correctly within exported ebook
- ✅ All tests pass (unit, integration, property-based)
- ✅ No clippy warnings
- ✅ Code formatted
- ✅ Performance: < 100ms for 100KB file

## Future Considerations

- **Multi-file projects**: Support importing directories of MD files with a manifest file
- **Syntax highlighting**: Pre-format code blocks with syntect for HTML-based formats
- **Custom IR nodes**: Special handling for diagrams (convert Mermaid to images during export)
- **MD output**: Export IR back to Markdown (round-trip conversion)
- **Streaming parser**: For very large files (> 1MB), parse incrementally
- **Wiki-style links**: Support `[[Page Name]]` syntax for internal linking
- **Pandoc compatibility**: Support Pandoc-style metadata blocks and extensions
