# Intermediate Representation (IR)

The Intermediate Representation (IR) is the core data model that enables conversion between different ebook formats. It captures content, structure, styles, and semantics in a format-agnostic way.

## Overview

The IR consists of three main components:

1. **Nodes**: Tree structure with semantic roles (paragraph, heading, image, etc.)
2. **Styles**: Computed CSS properties for each node
3. **Semantics**: Attributes (href, src, alt, id) for links and media

## Chapter Structure

```rust
pub struct Chapter {
    /// Tree of nodes (parent-child relationships)
    pub nodes: Arena<Node>,

    /// Text content (indexed by TextId)
    pub text: TextPool,

    /// Computed styles (indexed by StyleId)
    pub styles: StylePool,

    /// Semantic attributes (href, src, alt, id)
    pub semantics: SemanticMap,
}
```

## Node Types

Each node has a `Role` that defines its semantic meaning:

### Block-Level Roles

| Role | Description |
|------|-------------|
| `Root` | Document root |
| `Paragraph` | Paragraph of text |
| `Heading(level)` | Heading (H1-H6) |
| `BlockQuote` | Block quotation |
| `CodeBlock` | Preformatted code |
| `OrderedList` | Ordered list (`<ol>`) |
| `UnorderedList` | Unordered list (`<ul>`) |
| `ListItem` | List item (`<li>`) |
| `Table` | Table |
| `TableHead` | Table head (`<thead>`) |
| `TableBody` | Table body (`<tbody>`) |
| `TableRow` | Table row (`<tr>`) |
| `TableCell` | Table cell (`<td>`) |
| `DefinitionList` | Definition list (`<dl>`) |
| `DefinitionTerm` | Definition term (`<dt>`) |
| `DefinitionDescription` | Definition description (`<dd>`) |
| `Sidebar` | Sidebar/aside content |
| `Footnote` | Footnote or endnote |
| `Figure` | Figure with caption |
| `Caption` | Caption for figure/table |

### Inline Roles

| Role | Description |
|------|-------------|
| `Text` | Text content |
| `Inline` | Generic inline container |
| `Link` | Hyperlink (`<a>`) |
| `Image` | Image (`<img>`) |

### Special Roles

| Role | Description |
|------|-------------|
| `Break` | Line break (`<br>`) |
| `Rule` | Horizontal rule (`<hr>`) |

## Node Structure

```rust
pub struct Node {
    /// Unique identifier within the chapter
    pub id: NodeId,

    /// Semantic role (paragraph, heading, etc.)
    pub role: Role,

    /// Computed style reference
    pub style: StyleId,

    /// Parent node (None for root)
    pub parent: Option<NodeId>,

    /// First child (for tree traversal)
    pub first_child: Option<NodeId>,

    /// Next sibling (for tree traversal)
    pub next_sibling: Option<NodeId>,

    /// Text content (for Text role only)
    pub text: TextRange,
}
```

### Tree Traversal

The IR uses an arena-based tree structure for efficient traversal:

```rust
// Depth-first iteration
for node_id in chapter.iter_dfs() {
    let node = chapter.node(node_id)?;
    // Process node...
}

// Get children
for child_id in chapter.children(parent_id) {
    let child = chapter.node(child_id)?;
    // Process child...
}

// Get parent
if let Some(parent_id) = chapter.parent(child_id) {
    let parent = chapter.node(parent_id)?;
    // Process parent...
}
```

## Style System

Each node has a `StyleId` that indexes into the `StylePool`:

```rust
pub struct ComputedStyle {
    // Font properties
    pub font_family: String,
    pub font_size: Length,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_variant: FontVariant,

    // Color
    pub color: Color,
    pub background_color: Color,

    // Layout
    pub display: Display,
    pub text_align: TextAlign,
    pub text_indent: Length,
    pub line_height: Option<Length>,

    // Spacing
    pub margin_top: Length,
    pub margin_bottom: Length,
    pub margin_left: Length,
    pub margin_right: Length,

    pub padding_top: Length,
    pub padding_bottom: Length,
    pub padding_left: Length,
    pub padding_right: Length,

    // Borders
    pub border_top_width: Length,
    pub border_top_color: Color,
    pub border_top_style: BorderStyle,
    // ... (other borders)

    // Text
    pub text_decoration: Vec<TextDecorationValue>,
    pub text_transform: TextTransform,
    pub white_space: WhiteSpace,

    // Lists
    pub list_style_type: ListStyleType,
    pub list_style_position: ListStylePosition,

    // Tables
    pub border_collapse: BorderCollapse,

    // Other
    pub page_break_before: BreakValue,
    pub page_break_after: BreakValue,
    pub float: Float,
    pub clear: Clear,
    pub overflow_wrap: OverflowWrap,
    pub hyphens: Hyphens,
    pub vertical_align: VerticalAlign,
    pub visibility: Visibility,
}
```

### Style Inheritance

Styles are computed using CSS cascade rules:
1. User-agent stylesheet (default styles)
2. Author stylesheets (linked and inline CSS)
3. Inheritance from parent nodes
4. Specificity-based resolution

## Semantic Attributes

The `SemanticMap` stores attributes separately from nodes:

```rust
pub struct SemanticMap {
    /// Hyperlink targets (href attribute)
    href: HashMap<NodeId, String>,

    /// Image sources (src attribute)
    src: HashMap<NodeId, String>,

    /// Alternative text (alt attribute)
    alt: HashMap<NodeId, String>,

    /// Element IDs (id attribute)
    id: HashMap<NodeId, String>,
}
```

### Accessing Semantics

```rust
// Get href for a link node
if let Some(href) = chapter.semantics.href(node_id) {
    println!("Link target: {}", href);
}

// Get src for an image node
if let Some(src) = chapter.semantics.src(node_id) {
    println!("Image source: {}", src);
}

// Get alt text for accessibility
if let Some(alt) = chapter.semantics.alt(node_id) {
    println!("Alt text: {}", alt);
}

// Get element ID for anchor resolution
if let Some(id) = chapter.semantics.id(node_id) {
    println!("Element ID: {}", id);
}
```

## Text Content

Text is stored in a separate `TextPool` to enable deduplication:

```rust
pub struct TextPool {
    /// Text content indexed by TextId
    content: Vec<String>,
}

pub struct TextRange {
    /// Index into TextPool
    pub id: TextId,
    /// Byte range within the text string
    pub start: usize,
    pub end: usize,
}
```

### Accessing Text

```rust
// Get text content for a Text node
let node = chapter.node(node_id)?;
if node.role == Role::Text {
    let text = chapter.text(node.text);
    println!("Text: {}", text);
}
```

## Link Resolution

Links are resolved to `AnchorTarget` enum values:

```rust
pub enum AnchorTarget {
    /// External URL (http://, https://, mailto:, tel:)
    External(String),

    /// Internal link to a specific node
    Internal(GlobalNodeId),

    /// Link to a chapter (no fragment)
    Chapter(ChapterId),
}
```

### Global Node IDs

`GlobalNodeId` uniquely identifies a node across the entire book:

```rust
pub struct GlobalNodeId {
    pub chapter: ChapterId,
    pub node: NodeId,
}
```

## Metadata

Book metadata is stored in the `Metadata` struct:

```rust
pub struct Metadata {
    // Dublin Core
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
    pub identifier: String,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub subjects: Vec<String>,
    pub date: Option<String>,
    pub rights: Option<String>,
    pub cover_image: Option<String>,

    // EPUB3 Extensions
    pub modified_date: Option<String>,  // dcterms:modified
    pub contributors: Vec<Contributor>,  // dc:contributor
    pub title_sort: Option<String>,  // file-as for title
    pub author_sort: Option<String>,  // file-as for author
    pub collection: Option<CollectionInfo>,  // belongs-to-collection
}

pub struct Contributor {
    pub name: String,
    pub file_as: Option<String>,  // Sort key
    pub role: Option<String>,  // MARC relator (trl, edt, ill, etc.)
}

pub struct CollectionInfo {
    pub name: String,
    pub collection_type: Option<String>,  // "series" or "set"
    pub position: Option<f64>,  // group-position (1, 2, 3.5, etc.)
}
```

## Table of Contents

The TOC is a hierarchical structure:

```rust
pub struct TocEntry {
    pub title: String,
    pub href: String,
    pub children: Vec<TocEntry>,
    pub play_order: Option<usize>,  // For sorting
    pub target: Option<AnchorTarget>,  // Resolved after link resolution
}
```

## Landmarks

Landmarks identify structural navigation points:

```rust
pub struct Landmark {
    pub landmark_type: LandmarkType,
    pub href: String,
    pub label: String,
}

pub enum LandmarkType {
    Cover,
    TitlePage,
    Toc,
    StartReading,
    BodyMatter,
    FrontMatter,
    BackMatter,
    Acknowledgements,
    Bibliography,
    Glossary,
    Index,
    Preface,
    Endnotes,
    Loi,  // List of illustrations
    Lot,  // List of tables
}
```

## IR Operations

### Compilation

HTML is compiled to IR via `compile_html()`:

```rust
use boko::dom::{compile_html, Stylesheet, Origin};

let html = r#"<p class="intro">Hello, <em>world</em>!</p>"#;
let css = ".intro { color: blue; }";

let stylesheet = Stylesheet::parse(css);
let chapter = compile_html(html, &[(stylesheet, Origin::Author)]);
```

### Traversal

```rust
// Depth-first iteration
for node_id in chapter.iter_dfs() {
    let node = chapter.node(node_id)?;
    // Process node...
}

// Get children of a specific node
for child_id in chapter.children(parent_id) {
    let child = chapter.node(child_id)?;
    // Process child...
}
```

### Style Access

```rust
let node = chapter.node(node_id)?;
let style = chapter.styles.get(node.style)?;

if style.font_weight == FontWeight::BOLD {
    // Node is bold...
}
```

### Serialization

The IR can be serialized to JSON for debugging:

```bash
boko dump book.epub --json
```

## IR Design Rationale

1. **Arena-based tree**: Efficient memory layout and cache locality
2. **Separate text pool**: Deduplicates text content
3. **Style deduplication**: Similar nodes share styles via StyleId
4. **Semantic attributes**: Separated from structure for flexible export
5. **Format-agnostic**: No format-specific information in core IR

## IR Invariants

1. The tree is always connected (except for the root node)
2. `first_child` and `next_sibling` form a consistent tree structure
3. All `StyleId` values exist in the `StylePool`
4. All `TextId` values exist in the `TextPool`
5. Parent-child relationships are bidirectional
