# Markdown Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Markdown file import support to Boko, enabling conversion of MD documents to ebook formats (EPUB, KFX, AZW3, MOBI)

**Architecture:** Implement `Importer` trait for Markdown using `pulldown-cmark` parser. Parse MD events into Boko's IR (nodes, styles, semantics), split by `#` headings, extract images, and resolve links.

**Tech Stack:** Rust 2024, pulldown-cmark 0.12, serde_yaml 0.9 (optional), existing Boko IR/Style system

---

## Phase 1: Basic CommonMark (MVP)

**Acceptance:** Can import `simple.md` and export to valid EPUB with chapters and images.

### Task 1: Add Dependencies and Module Structure

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/import/mod.rs`
- Create: `src/import/markdown.rs`

- [ ] **Step 1: Add pulldown-cmark dependency to Cargo.toml**

```toml
# In [dependencies] section
pulldown-cmark = "0.12"
```

Run: `cargo check`
Expected: No errors, dependency resolved

- [ ] **Step 2: Add optional serde_yaml dependency**

```toml
# In [dependencies] section
serde_yaml = { version = "0.9", optional = true }
```

Run: `cargo check`
Expected: No errors

- [ ] **Step 3: Add markdown module to src/import/mod.rs**

```rust
// Add after the other module declarations (mod epub;, mod kfx;, etc.)
mod markdown;

// Add after the other pub use statements (pub use epub::EpubImporter;, etc.)
pub use markdown::MarkdownImporter;
```

Run: `cargo check`
Expected: Error "cannot find module `markdown`" (expected, we create it next)

- [ ] **Step 4: Create src/import/markdown.rs with basic structure**

```rust
//! Markdown format importer.

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::import::{ChapterId, Importer, SpineEntry};
use crate::model::{
    AnchorTarget, Chapter, GlobalNodeId, Landmark, Metadata, TocEntry,
};
use crate::style::{ComputedStyle, FontStyle, FontWeight, Length};

/// Markdown format importer.
pub struct MarkdownImporter {
    path: PathBuf,
    content: String,
    metadata: Metadata,
    toc: Vec<TocEntry>,
    spine: Vec<SpineEntry>,
    assets: HashMap<String, Vec<u8>>,
    chapter_ranges: Vec<ChapterRange>,
    anchor_map: HashMap<String, GlobalNodeId>,
    asset_paths: Vec<PathBuf>,
    heading_nodes: Vec<(usize, String)>,
    config: MarkdownConfig,
}

/// Configuration for Markdown import.
#[derive(Debug, Clone)]
pub struct MarkdownConfig {
    pub split_on_h1: bool,
    pub extract_front_matter: bool,
    pub enable_gfm: bool,
    pub asset_base_dir: Option<PathBuf>,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            split_on_h1: true,
            extract_front_matter: true,
            enable_gfm: true,
            asset_base_dir: None,
        }
    }
}

struct ChapterRange {
    start: usize,
    end: usize,
    virtual_path: String,
}

impl MarkdownImporter {
    fn new(path: PathBuf, content: String) -> Self {
        Self {
            path,
            content,
            metadata: Metadata::default(),
            toc: Vec::new(),
            spine: Vec::new(),
            assets: HashMap::new(),
            chapter_ranges: Vec::new(),
            anchor_map: HashMap::new(),
            asset_paths: Vec::new(),
            heading_nodes: Vec::new(),
            config: MarkdownConfig::default(),
        }
    }
}
```

Run: `cargo check`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/import/mod.rs src/import/markdown.rs
git commit -m "feat(markdown): add module structure and dependencies"
```

---

### Task 2: Implement Importer::open() - File Loading and Scanning

**Files:**
- Modify: `src/import/markdown.rs:40-80`

- [ ] **Step 1: Write failing test for open()**

Create `tests/markdown_open_test.rs`:

```rust
use boko::Book;
use std::path::Path;

#[test]
fn test_open_markdown_file() {
    let path = Path::new("tests/fixtures/markdown/simple.md");
    if !path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let result = Book::open(path);
    assert!(result.is_ok(), "Failed to open markdown file");

    let book = result.unwrap();
    assert!(!book.metadata().title.is_empty());
}
```

Run: `cargo test test_open_markdown_file`
Expected: FAIL - "format not supported" or similar

- [ ] **Step 2: Implement Importer::open() basic skeleton**

Add to `src/import/markdown.rs` after `impl MarkdownImporter`:

```rust
impl Importer for MarkdownImporter {
    fn open(path: &Path) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Read file
        let mut file = std::fs::File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        // Create importer
        let mut importer = Self::new(path.to_path_buf(), content);

        // Scan for headings (simplified - full implementation in later tasks)
        importer.scan_headings();

        // Build metadata from first heading
        importer.build_metadata();

        Ok(importer)
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn toc(&self) -> &[TocEntry] {
        &self.toc
    }

    fn toc_mut(&mut self) -> &mut [TocEntry] {
        &mut self.toc
    }

    fn landmarks(&self) -> &[Landmark] {
        &[]
    }

    fn spine(&self) -> &[SpineEntry] {
        &self.spine
    }

    fn source_id(&self, id: ChapterId) -> Option<&str> {
        self.chapter_ranges
            .get(id.0 as usize)
            .map(|r| r.virtual_path.as_str())
    }

    fn load_raw(&mut self, id: ChapterId) -> io::Result<Vec<u8>> {
        let range = self.chapter_ranges.get(id.0 as usize).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Chapter {} not found", id.0),
            )
        })?;
        Ok(self.content[range.start..range.end].as_bytes().to_vec())
    }

    fn list_assets(&self) -> &[PathBuf] {
        // TODO: Return asset paths
        &[]
    }

    fn load_asset(&mut self, _path: &Path) -> io::Result<Vec<u8>> {
        // TODO: Load assets
        Err(io::Error::new(io::ErrorKind::NotFound, "Asset loading not implemented"))
    }

    fn font_faces(&mut self) -> Vec<crate::model::FontFace> {
        Vec::new()
    }

    fn index_anchors(&mut self, _chapters: &[(ChapterId, Arc<Chapter>)]) {
        // TODO: Build anchor map
    }

    fn resolve_href(&self, _from_chapter: ChapterId, href: &str) -> Option<AnchorTarget> {
        let href = href.trim();
        if href.starts_with("http://") || href.starts_with("https://") {
            return Some(AnchorTarget::External(href.to_string()));
        }
        None
    }
}
```

- [ ] **Step 3: Implement scan_headings() helper**

Add to `impl MarkdownImporter`:

```rust
impl MarkdownImporter {
    fn scan_headings(&mut self) {
        use pulldown_cmark::Parser;

        let parser = Parser::new(&self.content);
        let mut chapter_start = 0;
        let mut chapter_index = 0;

        // If no headings, create single chapter
        let mut found_heading = false;

        for (event, range) in parser.into_offset_iter() {
            use pulldown_cmark::Event::{Start, End};
            use pulldown_cmark::Tag::Heading;

            match event {
                Start(Heading { level: 1, .. }) => {
                    found_heading = true;
                    // Save previous chapter range
                    if chapter_index > 0 {
                        let prev_end = range.start;
                        self.chapter_ranges.last_mut().unwrap().end = prev_end;
                    }

                    // Start new chapter
                    self.chapter_ranges.push(ChapterRange {
                        start: range.start,
                        end: self.content.len(),
                        virtual_path: format!("chapter-{}.md", chapter_index + 1),
                    });

                    // Create spine entry
                    self.spine.push(SpineEntry {
                        id: ChapterId(chapter_index),
                        size_estimate: 0,
                    });

                    chapter_index += 1;
                }
                _ => {}
            }
        }

        // If no headings found, create single chapter with entire document
        if !found_heading {
            self.chapter_ranges.push(ChapterRange {
                start: 0,
                end: self.content.len(),
                virtual_path: format!("{}.md", file_stem(&self.path)),
            });
            self.spine.push(SpineEntry {
                id: ChapterId(0),
                size_estimate: self.content.len(),
            });
        }
    }

    fn build_metadata(&mut self) {
        // Extract title from first heading or filename
        self.metadata.title = self
            .extract_first_heading()
            .unwrap_or_else(|| file_stem(&self.path));

        // Set default language
        if self.metadata.language.is_empty() {
            self.metadata.language = "en".to_string();
        }
    }

    fn extract_first_heading(&self) -> Option<String> {
        use pulldown_cmark::Event::{Start, Text};
        use pulldown_cmark::Tag::Heading;

        let parser = Parser::new(&self.content);
        let mut in_heading = false;

        for event in parser {
            match event {
                Start(Heading { level: 1, .. }) => {
                    in_heading = true;
                }
                Text(text) if in_heading => {
                    return Some(text.to_string());
                }
                _ => {}
            }
        }
        None
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_open_markdown_file`
Expected: PASS

- [ ] **Step 5: Create test fixture**

Create `tests/fixtures/markdown/simple.md`:

```markdown
# My Document

This is a simple paragraph.

## Section One

Content for section one.

# Chapter Two

More content here.
```

- [ ] **Step 6: Run test again**

Run: `cargo test test_open_markdown_file`
Expected: PASS with 2 chapters detected

- [ ] **Step 7: Commit**

```bash
git add src/import/markdown.rs tests/markdown_open_test.rs tests/fixtures/markdown/simple.md
git commit -m "feat(markdown): implement open() with chapter scanning"
```

---

### Task 3: Update Format Enum to Support Markdown

**Files:**
- Modify: `src/model/book.rs`
- Modify: `src/format.rs` (if exists)

- [ ] **Step 1: Find Format enum and can_import() method**

Run: `grep -n "pub enum Format" src/*.rs src/**/*.rs`
Expected: Found in src/model/book.rs or src/format.rs

- [ ] **Step 2: Update Format::can_import() to include Markdown**

Find the `can_import()` method and add `Format::Markdown`:

```rust
pub fn can_import(&self) -> bool {
    matches!(
        self,
        Format::Epub | Format::Kfx | Format::Azw3 | Format::Mobi | Format::Markdown
    )
}
```

- [ ] **Step 3: Remove Markdown import blocking code**

Find and remove this code block in `Book::open_format()`:

```rust
Format::Markdown => {
    return Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Markdown import is not yet implemented",
    ));
}
```

- [ ] **Step 4: Add MarkdownImporter to match statement**

In `Book::open_format()`, add:

```rust
let importer: Box<dyn Importer> = match format {
    Format::Epub => Box::new(EpubImporter::open(path)?),
    Format::Kfx => Box::new(KfxImporter::open(path)?),
    Format::Azw3 => Box::new(Azw3Importer::open(path)?),
    Format::Mobi => Box::new(MobiImporter::open(path)?),
    Format::Markdown => Box::new(MarkdownImporter::open(path)?),
    _ => {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cannot detect format automatically",
        ));
    }
};
```

Also add import at top: `use crate::import::MarkdownImporter;`

- [ ] **Step 5: Run test to verify Format integration**

Run: `cargo test test_open_markdown_file`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/model/book.rs
git commit -m "feat(markdown): enable Markdown in Format enum"
```

---

### Task 4: Implement IR Builder - Block Elements

**Files:**
- Modify: `src/import/markdown.rs`

- [ ] **Step 1: Write failing test for load_chapter()**

Add to `tests/markdown_open_test.rs`:

```rust
#[test]
fn test_load_chapter_paragraphs() {
    use boko::Book;
    use std::path::Path;

    let path = Path::new("tests/fixtures/markdown/simple.md");
    if !path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let mut book = Book::open(path).unwrap();
    let chapter = book.load_chapter(boko::import::ChapterId(0)).unwrap();

    // Should have nodes
    assert!(chapter.node_count() > 1); // At least root + content
}
```

Run: `cargo test test_load_chapter_paragraphs`
Expected: FAIL - default implementation expects HTML

- [ ] **Step 2: Implement IrBuilder struct and basic event loop**

Add to `src/import/markdown.rs`:

```rust
use crate::model::{Chapter, Node, NodeId, Role, TextRange};
use crate::model::semantic::SemanticMap;
use crate::style::StylePool;

struct IrBuilder {
    chapter: Chapter,
    parent_stack: Vec<NodeId>,
    text_buffer: String,
}

impl IrBuilder {
    fn new() -> Self {
        let mut chapter = Chapter::new();
        let root = chapter.root();
        Self {
            chapter,
            parent_stack: vec![root],
            text_buffer: String::new(),
        }
    }

    fn flush_text(&mut self) {
        if self.text_buffer.is_empty() {
            return;
        }

        let text_range = self.chapter.append_text(&self.text_buffer);
        let text_id = self.chapter.alloc_node(Node::text(text_range));

        if let Some(&parent) = self.parent_stack.last() {
            self.chapter.append_child(parent, text_id);
        }

        self.text_buffer.clear();
    }

    fn push_node(&mut self, role: Role) -> NodeId {
        self.flush_text();
        let node_id = self.chapter.alloc_node(Node::new(role));
        if let Some(&parent) = self.parent_stack.last() {
            self.chapter.append_child(parent, node_id);
        }
        self.parent_stack.push(node_id);
        node_id
    }

    fn pop_node(&mut self) {
        self.flush_text();
        if self.parent_stack.len() > 1 {
            self.parent_stack.pop();
        }
    }

    fn build(self) -> Chapter {
        self.chapter
    }
}
```

- [ ] **Step 3: Override load_chapter() with IrBuilder**

Replace the default `load_chapter()` in `Importer` impl with:

```rust
fn load_chapter(&mut self, id: ChapterId) -> io::Result<Chapter> {
    use pulldown_cmark::{Event, Tag};

    // Get chapter content
    let range = self.chapter_ranges.get(id.0 as usize).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Chapter {} not found", id.0),
        )
    })?;

    let content = &self.content[range.start..range.end];
    let parser = pulldown_cmark::Parser::new(content);

    let mut builder = IrBuilder::new();

    for event in parser {
        match event {
            Event::Start(tag) => self.handle_start_tag(&mut builder, tag),
            Event::End(tag) => self.handle_end_tag(&mut builder, tag),
            Event::Text(text) => {
                builder.text_buffer.push_str(&text);
            }
            Event::Code(code) => {
                // Inline code: create Inline node with monospace style
                let style = ComputedStyle {
                    font_family: Some("monospace".to_string()),
                    ..Default::default()
                };
                let style_id = builder.chapter.styles.intern(style);
                let inline_id = builder.push_node(Role::Inline);
                builder.chapter.node_mut(inline_id).unwrap().style = style_id;
                builder.text_buffer.push_str(&code);
                builder.flush_text(); // Flush to create Text node as child
                builder.pop_node();
            }
            Event::Rule => {
                builder.push_node(Role::Rule);
                builder.pop_node();
            }
            Event::SoftBreak | Event::HardBreak => {
                builder.push_node(Role::Break);
                builder.pop_node();
            }
            _ => {} // Ignore other events for now
        }
    }

    Ok(builder.build())
}
```

- [ ] **Step 4: Implement tag handlers**

Add to `impl MarkdownImporter`:

```rust
impl MarkdownImporter {
    fn handle_start_tag(&self, builder: &mut IrBuilder, tag: pulldown_cmark::Tag) {
        use pulldown_cmark::Tag;

        match tag {
            Tag::Paragraph => {
                builder.push_node(Role::Paragraph);
            }
            Tag::Heading { level, .. } => {
                builder.push_node(Role::Heading(level as u8));
            }
            Tag::BlockQuote => {
                builder.push_node(Role::BlockQuote);
            }
            Tag::CodeBlock(..) => {
                builder.push_node(Role::CodeBlock);
            }
            Tag::List(..) => {
                // Default to unordered list
                builder.push_node(Role::UnorderedList);
            }
            Tag::Item => {
                builder.push_node(Role::ListItem);
            }
            Tag::Table(..) => {
                builder.push_node(Role::Table);
            }
            Tag::TableHead => {
                builder.push_node(Role::TableHead);
            }
            Tag::TableRow => {
                builder.push_node(Role::TableRow);
            }
            Tag::TableCell => {
                builder.push_node(Role::TableCell);
            }
            Tag::Emphasis => {
                // Create Inline node with italic style
                let style = ComputedStyle {
                    font_style: FontStyle::Italic,
                    ..Default::default()
                };
                let style_id = builder.chapter.styles.intern(style);
                let inline_id = builder.push_node(Role::Inline);
                builder.chapter.node_mut(inline_id).unwrap().style = style_id;
            }
            Tag::Strong => {
                // Create Inline node with bold style
                let style = ComputedStyle {
                    font_weight: FontWeight::BOLD,
                    ..Default::default()
                };
                let style_id = builder.chapter.styles.intern(style);
                let inline_id = builder.push_node(Role::Inline);
                builder.chapter.node_mut(inline_id).unwrap().style = style_id;
            }
            Tag::Link { .. } => {
                builder.push_node(Role::Link);
            }
            Tag::Image { .. } => {
                builder.push_node(Role::Image);
            }
            _ => {} // Ignore other tags
        }
    }

    fn handle_end_tag(&self, builder: &mut IrBuilder, tag: pulldown_cmark::Tag) {
        use pulldown_cmark::Tag;

        match tag {
            Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote
            | Tag::CodeBlock(..)
            | Tag::List(..)
            | Tag::Item
            | Tag::Table(..)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::Link
            | Tag::Image => {
                builder.pop_node();
            }
            Tag::Emphasis | Tag::Strong => {
                // Already handled in start_tag
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test test_load_chapter_paragraphs`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/import/markdown.rs tests/markdown_open_test.rs
git commit -m "feat(markdown): implement IR builder for block elements"
```

---

### Task 5: Handle Links and Images with Semantics

**Files:**
- Modify: `src/import/markdown.rs`

- [ ] **Step 1: Write failing test for image extraction**

Add to `tests/markdown_open_test.rs`:

```rust
#[test]
fn test_image_extraction() {
    use boko::Book;
    use std::path::Path;

    let path = Path::new("tests/fixtures/markdown/with_images.md");
    if !path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let mut book = Book::open(path).unwrap();
    let assets = book.list_assets();
    assert!(!assets.is_empty(), "Should extract images");
}
```

Run: `cargo test test_image_extraction`
Expected: FAIL - asset list empty

- [ ] **Step 2: Track assets during parsing**

Note: `asset_paths` field is already in `MarkdownImporter` struct from Task 1, Step 4.

- [ ] **Step 3: Scan for images during open()**

Add to `impl MarkdownImporter`:

```rust
impl MarkdownImporter {
    fn scan_assets(&mut self) {
        use pulldown_cmark::{Event, Tag};

        let parser = pulldown_cmark::Parser::new(&self.content);
        let base_dir = self.path.parent().unwrap_or(Path::new("."));

        for event in parser {
            if let Event::Start(Tag::Image { .. }) = event {
                // Image tags are followed by link destination
                // For now, we'll extract during load_chapter
            }
        }
    }
}
```

- [ ] **Step 4: Handle Link and Image events directly in event loop**

Modify `load_chapter()` to handle Link and Image Start events:

```rust
Event::Start(Tag::Link { dest_url, .. }) => {
    let url = dest_url.to_string();
    let node_id = builder.push_node(Role::Link);
    builder.chapter.semantics.set_href(node_id, url);
}

Event::Start(Tag::Image { dest_url, .. }) => {
    let url = dest_url.to_string();
    let node_id = builder.push_node(Role::Image);
    builder.chapter.semantics.set_src(node_id, url.clone());

    // Track asset if not external
    if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("data:") {
        let base_dir = self.path.parent().unwrap_or(Path::new("."));
        let full_path = base_dir.join(&url);
        if !self.asset_paths.contains(&full_path) {
            self.asset_paths.push(full_path.clone());

            // Try to load asset
            if let Ok(bytes) = std::fs::read(&full_path) {
                self.assets.insert(url.clone(), bytes);
            }
        }
    }
}
```

Also update `handle_start_tag` to remove Link and Image cases since we handle them directly:

```rust
impl MarkdownImporter {
    fn handle_start_tag(&self, builder: &mut IrBuilder, tag: pulldown_cmark::Tag) {
        use pulldown_cmark::Tag;

        match tag {
            Tag::Paragraph => {
                builder.push_node(Role::Paragraph);
            }
            Tag::Heading { level, .. } => {
                builder.push_node(Role::Heading(level as u8));
            }
            Tag::BlockQuote => {
                builder.push_node(Role::BlockQuote);
            }
            Tag::CodeBlock(..) => {
                builder.push_node(Role::CodeBlock);
            }
            Tag::List(..) => {
                // Default to unordered list
                builder.push_node(Role::UnorderedList);
            }
            Tag::Item => {
                builder.push_node(Role::ListItem);
            }
            Tag::Table(..) => {
                builder.push_node(Role::Table);
            }
            Tag::TableHead => {
                builder.push_node(Role::TableHead);
            }
            Tag::TableRow => {
                builder.push_node(Role::TableRow);
            }
            Tag::TableCell => {
                builder.push_node(Role::TableCell);
            }
            Tag::Emphasis => {
                // Create Inline node with italic style
                let style = ComputedStyle {
                    font_style: FontStyle::Italic,
                    ..Default::default()
                };
                let style_id = builder.chapter.styles.intern(style);
                let inline_id = builder.push_node(Role::Inline);
                builder.chapter.node_mut(inline_id).unwrap().style = style_id;
            }
            Tag::Strong => {
                // Create Inline node with bold style
                let style = ComputedStyle {
                    font_weight: FontWeight::BOLD,
                    ..Default::default()
                };
                let style_id = builder.chapter.styles.intern(style);
                let inline_id = builder.push_node(Role::Inline);
                builder.chapter.node_mut(inline_id).unwrap().style = style_id;
            }
            // Link and Image are handled directly in event loop
            _ => {} // Ignore other tags
        }
    }
}
```

Also update `handle_end_tag` to remove Link and Image from the match:

```rust
fn handle_end_tag(&self, builder: &mut IrBuilder, tag: pulldown_cmark::Tag) {
    use pulldown_cmark::Tag;

    match tag {
        Tag::Paragraph
        | Tag::Heading { .. }
        | Tag::BlockQuote
        | Tag::CodeBlock(..)
        | Tag::List(..)
        | Tag::Item
        | Tag::Table(..)
        | Tag::TableHead
        | Tag::TableRow
        | Tag::TableCell => {
            builder.pop_node();
        }
        Tag::Emphasis | Tag::Strong => {
            // Already handled in start_tag
        }
        _ => {}
    }
}
```

- [ ] **Step 6: Update list_assets() to return asset_paths**

```rust
fn list_assets(&self) -> &[PathBuf] {
    &self.asset_paths
}
```

- [ ] **Step 7: Update load_asset()**

```rust
fn load_asset(&mut self, path: &Path) -> io::Result<Vec<u8>> {
    // Try to get from cache
    let key = path.to_string_lossy().to_string();
    if let Some(bytes) = self.assets.get(&key) {
        return Ok(bytes.clone());
    }

    // Try to load from disk
    std::fs::read(path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to load asset {}: {}", key, e),
        )
    })
}
```

- [ ] **Step 8: Create test fixture with images**

Create `tests/fixtures/markdown/with_images.md`:

```markdown
# Document with Images

Here's an image:

![Test Image](image/test.jpg)

And another reference to the same image.
```

Create a dummy image file `tests/fixtures/markdown/image/test.jpg` (can use the create_jpeg_fixture.rs pattern).

- [ ] **Step 9: Run tests**

Run: `cargo test test_image_extraction`
Expected: PASS

- [ ] **Step 10: Commit**

```bash
git add src/import/markdown.rs tests/markdown_open_test.rs tests/fixtures/markdown/with_images.md
git commit -m "feat(markdown): handle images and links with semantics"
```

---

### Task 6: Implement TOC Generation

**Files:**
- Modify: `src/import/markdown.rs`

- [ ] **Step 1: Write failing test for TOC**

Add to `tests/markdown_open_test.rs`:

```rust
#[test]
fn test_toc_generation() {
    use boko::Book;
    use std::path::Path;

    let path = Path::new("tests/fixtures/markdown/simple.md");
    if !path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let book = Book::open(path).unwrap();
    let toc = book.toc();
    assert!(!toc.is_empty(), "Should have TOC entries");
}
```

Run: `cargo test test_toc_generation`
Expected: FAIL - TOC is empty

- [ ] **Step 2: Implement TOC building during scan**

Update `scan_headings()` to build TOC:

```rust
impl MarkdownImporter {
    fn scan_headings(&mut self) {
        use pulldown_cmark::{Event, Tag};
        use pulldown_cmark::Event::Start;

        let parser = pulldown_cmark::Parser::new(&self.content);
        let mut chapter_index = 0;
        let mut toc_stack: Vec<(usize, ChapterId)> = Vec::new(); // (level, entry_index)

        // ... existing chapter splitting logic ...

        // Add TOC building
        let mut last_level = 0;

        for (event, _range) in parser.into_offset_iter() {
            if let Start(Tag::Heading { level, .. }) = event {
                let level = level as usize;

                // Pop stack to appropriate parent level
                while let Some(&(l, _)) = toc_stack.last() {
                    if l < level {
                        break;
                    }
                    toc_stack.pop();
                }

                // Create TOC entry
                let entry = TocEntry {
                    id: self.toc.len(),
                    title: String::new(), // Will fill during IR building
                    children: Vec::new(),
                    content_id: None,
                };

                let entry_index = self.toc.len();

                // Add to parent or make top-level entry
                if let Some((_, parent_index)) = toc_stack.last() {
                    self.toc[*parent_index].children.push(entry_index);
                }
                // Top-level entries are added directly to self.toc

                self.toc.push(entry);
                toc_stack.push((level, entry_index));

                last_level = level;
            }
        }
    }
}
```

- [ ] **Step 3: Store heading node IDs during scanning**

Note: `heading_nodes` field is already in `MarkdownImporter` struct from Task 1, Step 4.

Update `scan_headings()` to store heading titles:

```rust
impl MarkdownImporter {
    fn scan_headings(&mut self) {
        use pulldown_cmark::{Event, Tag};
        use pulldown_cmark::Event::Start;

        let parser = pulldown_cmark::Parser::new(&self.content);
        let mut chapter_index = 0;
        let mut toc_stack: Vec<(usize, ChapterId)> = Vec::new();
        let mut last_level = 0;
        let mut in_heading = false;
        let mut current_heading_text = String::new();

        // ... existing chapter splitting logic ...

        // Add TOC building with text capture
        for (event, _range) in parser.into_offset_iter() {
            match event {
                Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    current_heading_text.clear();
                    let level = level as usize;

                    // Pop stack to appropriate parent level
                    while let Some(&(l, _)) = toc_stack.last() {
                        if l < level {
                            break;
                        }
                        toc_stack.pop();
                    }

                    // Create TOC entry with placeholder title
                    let entry_index = self.toc.len();
                    let entry = TocEntry {
                        id: entry_index,
                        title: String::new(), // Will fill below
                        children: Vec::new(),
                        content_id: None,
                    };

                    // Add to parent or make top-level entry
                    if let Some((_, parent_index)) = toc_stack.last() {
                        self.toc[*parent_index].children.push(entry_index);
                    }

                    self.toc.push(entry);
                    toc_stack.push((level, entry_index));
                    self.heading_nodes.push((entry_index, String::new()));

                    last_level = level;
                }
                Event::Text(text) if in_heading => {
                    current_heading_text.push_str(&text);
                }
                Event::End(Tag::Heading { .. }) => {
                    in_heading = false;
                    // Fill in the title
                    if let Some(entry) = self.heading_nodes.last_mut() {
                        entry.1 = current_heading_text.clone();
                        self.toc[entry.0].title = current_heading_text.clone();
                    }
                    current_heading_text.clear();
                }
                _ => {}
            }
        }
    }
}
```

- [ ] **Step 4: Implement index_anchors() for link resolution**

```rust
fn index_anchors(&mut self, chapters: &[(ChapterId, Arc<Chapter>)]) {
    self.anchor_map.clear();

    for (chapter_id, chapter) in chapters {
        let virtual_path = match self.source_id(*chapter_id) {
            Some(p) => p,
            None => continue,
        };

        // Find all heading nodes
        for node_id in chapter.iter_dfs() {
            if let Some(node) = chapter.node(node_id) {
                if matches!(node.role, Role::Heading(_)) {
                    // Extract text from all Text node descendants
                    let text = chapter
                        .iter_dfs(node_id)
                        .filter_map(|id| chapter.node(id))
                        .filter(|n| matches!(n.role, Role::Text))
                        .filter_map(|n| {
                            let range = n.text;
                            chapter.text.get(range.start as usize..(range.start + range.len) as usize)
                        })
                        .collect::<String>();

                    let slug = slugify(&text);
                    let key = format!("{}#{}", virtual_path, slug);
                    self.anchor_map
                        .insert(key, GlobalNodeId::new(*chapter_id, node_id));
                }
            }
        }
    }
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test test_toc_generation`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/import/markdown.rs tests/markdown_open_test.rs
git commit -m "feat(markdown): implement TOC generation and anchor indexing"
```

---

### Task 7: Integration Test - MD to EPUB Export

**Files:**
- Create: `tests/markdown_epub_export_test.rs`

- [ ] **Step 1: Write integration test**

Create `tests/markdown_epub_export_test.rs`:

```rust
use boko::Book;
use boko::model::book::Format;  // Format is in model::book module
use std::io::Cursor;
use std::path::Path;
use zip::ZipArchive;

#[test]
fn test_markdown_to_epub_export() {
    let md_path = Path::new("tests/fixtures/markdown/simple.md");
    if !md_path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    // Open MD
    let mut book = Book::open(md_path).expect("Failed to open MD");

    // Export to EPUB
    let mut output = Cursor::new(Vec::new());
    book.export(Format::Epub, &mut output)
        .expect("Failed to export to EPUB");

    // Verify output is a valid ZIP (EPUB is a ZIP)
    let epub_bytes = output.into_inner();
    let zip = ZipArchive::new(std::io::Cursor::new(epub_bytes))
        .expect("Output is not a valid ZIP/EPUB");

    // Check for required EPUB files
    assert!(zip.file_names().any(|n| n.contains("mimetype")), "Missing mimetype");
    assert!(zip.file_names().any(|n| n.contains(".opf")), "Missing OPF");
    assert!(zip.file_names().any(|n| n.contains("toc.ncx") || n.contains("nav.xhtml")), "Missing TOC");

    println!("EPUB exported successfully with {} files", zip.len());
}
```

Run: `cargo test test_markdown_to_epub_export`
Expected: PASS

- [ ] **Step 2: Add more comprehensive test fixtures**

Create `tests/fixtures/markdown/comprehensive.md`:

```markdown
---
title: Comprehensive Test Document
author: Test Author
description: A test document with all features
---

# Main Title

This is a paragraph with *italic* and **bold** text.

## Section One

- List item one
- List item two

### Subsection

Code example:

```rust
fn hello() {
    println!("Hello, world!");
}
```

# Chapter Two

[External link](https://example.com)

![Image](image/test.jpg)

> Blockquote here
```

- [ ] **Step 3: Test with comprehensive fixture**

Add test:

```rust
#[test]
fn test_comprehensive_markdown_export() {
    let md_path = Path::new("tests/fixtures/markdown/comprehensive.md");
    if !md_path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let mut book = Book::open(md_path).expect("Failed to open MD");

    // Check metadata
    assert_eq!(book.metadata().title, "Comprehensive Test Document");
    assert!(!book.metadata().contributors.is_empty());

    // Export to EPUB
    let mut output = Cursor::new(Vec::new());
    book.export(Format::Epub, &mut output)
        .expect("Failed to export to EPUB");

    println!("Comprehensive MD exported successfully");
}
```

Run: `cargo test test_comprehensive_markdown_export`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add tests/markdown_epub_export_test.rs tests/fixtures/markdown/comprehensive.md
git commit -m "test(markdown): add integration tests for MD to EPUB export"
```

---

### Task 8: Final Polish - Clippy, Format, Documentation

**Files:**
- Modify: `src/import/markdown.rs`
- Create: `docs/markdown-import.md`

- [ ] **Step 1: Run clippy and fix warnings**

Run: `cargo clippy -- -D warnings`
Expected: No warnings
Fix any issues found

- [ ] **Step 2: Format code**

Run: `cargo fmt`
Expected: All code formatted

- [ ] **Step 3: Add documentation**

Create `docs/markdown-import.md`:

```markdown
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
```

- [ ] **Step 4: Update CLAUDE.md with Markdown import**

Add to `CLAUDE.md` in the relevant sections:

```markdown
## Format Support

- **Markdown**: Full import support with chapter splitting and image extraction
```

- [ ] **Step 5: Run full test suite**

Run: `cargo test --verbose`
Expected: All tests pass (including existing tests)

- [ ] **Step 6: Run benchmarks (if applicable)**

Run: `cargo bench --bench conversion`
Expected: No regressions

- [ ] **Step 7: Final commit**

```bash
git add src/import/markdown.rs docs/markdown-import.md CLAUDE.md
git commit -m "docs(markdown): add documentation and final polish"

# Create tag for Phase 1 completion
git tag -a v0.1.0-markdown-import -m "Phase 1: Basic Markdown import complete"
```

---

## Success Criteria Checklist

After completing all tasks, verify:

- [ ] Can import single `.md` files via `Book::open()`
- [ ] Content splits into chapters based on `#` headings
- [ ] Images extracted and embedded in exported ebooks
- [ ] Code blocks (including Mermaid) preserved in IR
- [ ] Inline formatting (emphasis, strong, code) uses CSS styles correctly
- [ ] Internal links resolve correctly within exported ebook
- [ ] All tests pass (unit, integration)
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] Documentation updated

---

## Next Steps (Phase 2)

After Phase 1 is complete, Phase 2 will add:
1. GFM extensions (tables, strikethrough, task lists)
2. YAML front matter parsing
3. Complete metadata field mapping
4. Enhanced TOC generation with fragment resolution
5. Asset path resolution with `asset_base_dir` config
6. Extended test fixtures
7. Property-based tests

See `docs/superpowers/specs/2026-03-28-markdown-import-design.md` for full details.
