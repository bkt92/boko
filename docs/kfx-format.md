# KFX Format Documentation

KFX (Kindle Format 10) is Amazon's latest ebook format, successor to AZW3/KF8. It uses a binary format based on Amazon's Ion data format.

## Overview

KFX files are ZIP archives containing:
- Binary Ion documents for content and metadata
- Auxiliary data for navigation and resources
- Separate containers for metadata and content

## File Structure

```
book.kfx (ZIP archive)
├── metadata.kfx (Ion binary)
├── content/
│   ├── fragment_1.kfx
│   ├── fragment_2.kfx
│   └── ...
├── resources/
│   ├── images/
│   │   ├── cover.jpg
│   │   └── ...
│   └── fonts/
│       └── ...
└── auxiliary/
    └── landmarks.kfx
```

## Ion Format

Ion is Amazon's JSON-like binary data format:

**Ion Types**:
- Null
- Bool
- Integer (variable-length)
- Float (32/64-bit)
- Decimal (high-precision)
- Timestamp
- String (UTF-8)
- Symbol (interned strings)
- Blob (binary data)
- Clob (UTF-8 text)
- List (ordered sequence)
- Sexp (symbolic expression)
- Struct (key-value pairs)

**Example Ion Value**:

```ion
{
    name: "Chapter 1",
    type: "body",
    content: [
        {type: "paragraph", style: 1, text: "Hello, world!"},
        {type: "image", resource: "images/cover.jpg"}
    ],
    metadata: {
        title: "My Book",
        author: "Jane Doe"
    }
}
```

## KFX Schema

### Content Fragment

```rust
pub struct ContentFragment {
    pub fragment_type: String,  // "body", "front_matter", etc.
    pub content: Vec<Token>,
    pub resources: Vec<Resource>,
}

pub enum Token {
    Text { text: String, style: StyleId },
    Paragraph { children: Vec<Token> },
    Heading { level: u8, children: Vec<Token> },
    Image { resource: String, alt: Option<String> },
    Link { href: String, children: Vec<Token> },
    // ... more token types
}
```

### Metadata Fragment

```rust
pub struct MetadataFragment {
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub identifiers: Vec<Identifier>,
    pub contributors: Vec<Contributor>,
    pub collection: Option<Collection>,
    // ... more metadata fields
}
```

### Style Registry

```rust
pub struct StyleRegistry {
    pub styles: Vec<ComputedStyle>,
    pub mapping: HashMap<StyleKey, StyleId>,
}

pub struct StyleKey {
    // Hash of style properties for deduplication
}
```

## KFX Modules

### `kfx/ion.rs`

Ion binary format parser and writer.

**Key Types**:

```rust
pub enum IonValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Decimal(String),
    Timestamp(String),
    String(String),
    Symbol(Symbol),
    Blob(Vec<u8>),
    List(Vec<IonValue>),
    Struct(Vec<(Symbol, IonValue)>),
}

pub struct IonReader {
    // Reads Ion binary format
}

pub struct IonWriter {
    // Writes Ion binary format
}
```

**Usage**:

```rust
// Read Ion value
let mut reader = IonReader::new(&data)?;
let value = reader.read_value()?;

// Write Ion value
let mut writer = IonWriter::new();
writer.write_struct("book")?;
writer.write_string("title", "My Book")?;
writer.end_struct()?;
let data = writer.finish()?;
```

### `kfx/symbols.rs`

KFX symbol table and enum.

```rust
pub enum KfxSymbol {
    // Content types
    Body,
    FrontMatter,
    BackMatter,

    // Token types
    Paragraph,
    Heading,
    Image,
    Link,

    // Style properties
    FontFamily,
    FontSize,
    Color,
    // ... more symbols
}
```

### `kfx/schema.rs`

Bidirectional KFX ↔ IR mapping rules.

```rust
pub fn kfx_to_ir(tokens: Vec<Token>) -> Chapter {
    // Convert KFX tokens to IR nodes
}

pub fn ir_to_kfx(chapter: &Chapter) -> Vec<Token> {
    // Convert IR nodes to KFX tokens
}
```

### `kfx/context.rs`

Export context for central state management.

```rust
pub struct ExportContext {
    pub style_registry: StyleRegistry,
    pub resource_map: HashMap<String, ResourceId>,
    pub anchor_map: HashMap<String, GlobalNodeId>,
    pub next_style_id: StyleId,
    pub next_resource_id: ResourceId,
}
```

**Usage**:

```rust
let mut ctx = ExportContext::new();

// Register style
let style_id = ctx.register_style(&style)?;

// Register resource
let resource_id = ctx.register_resource("images/cover.jpg")?;

// Get style ID
let style_id = ctx.get_style_id(&style)?;
```

### `kfx/style_registry.rs`

Style deduplication and ID assignment.

```rust
pub struct StyleRegistry {
    styles: Vec<ComputedStyle>,
    mapping: HashMap<StyleKey, StyleId>,
}

impl StyleRegistry {
    pub fn register(&mut self, style: &ComputedStyle) -> io::Result<StyleId> {
        // Check if style already registered
        let key = StyleKey::from(style);
        if let Some(id) = self.mapping.get(&key) {
            return Ok(*id);
        }

        // Register new style
        let id = StyleId(self.styles.len());
        self.styles.push(style.clone());
        self.mapping.insert(key, id);
        Ok(id)
    }

    pub fn get_id(&self, style: &ComputedStyle) -> Option<StyleId> {
        self.mapping.get(&StyleKey::from(style)).copied()
    }

    pub fn get_style(&self, id: StyleId) -> Option<&ComputedStyle> {
        self.styles.get(id.0)
    }
}
```

### `kfx/tokens.rs`

Token stream for import/export.

```rust
pub enum Token {
    StartContainer { role: Role },
    EndContainer,
    Text { text: String, style: StyleId },
    Image { resource: ResourceId, alt: Option<String> },
    Link { href: String },
    Style { style: ComputedStyle },
    Resource { id: ResourceId, data: Vec<u8>, media_type: String },
}
```

### `kfx/storyline.rs`

Storyline tokenization and IR building.

```rust
pub fn tokenize_storyline(ion: &IonValue) -> Vec<Token> {
    // Convert Ion document to token stream
}

pub fn build_ir(tokens: Vec<Token>) -> Chapter {
    // Build IR from token stream
}
```

### `kfx/transforms.rs`

Attribute value transformers for bidirectional conversion.

```rust
pub fn transform_color(value: &IonValue) -> Option<Color> {
    // Convert Ion color to IR Color
}

pub fn transform_length(value: &IonValue) -> Option<Length> {
    // Convert Ion length to IR Length
}

pub fn transform_font_style(value: &IonValue) -> Option<FontStyle> {
    // Convert Ion font style to IR FontStyle
}
```

### `kfx/container.rs`

KFX container format parsing (pure functions).

```rust
pub fn parse_container(data: &[u8]) -> io::Result<KfxContainer> {
    // Parse KFX ZIP container
}

pub struct KfxContainer {
    pub metadata: MetadataFragment,
    pub content: Vec<ContentFragment>,
    pub resources: Vec<Resource>,
    pub auxiliary: AuxiliaryData,
}
```

### `kfx/metadata.rs`

Metadata schema for book metadata mapping.

```rust
pub fn read_metadata(ion: &IonValue) -> Metadata {
    // Read metadata from Ion struct
}

pub fn write_metadata(metadata: &Metadata) -> IonValue {
    // Write metadata to Ion struct
}
```

### `kfx/fragment.rs`

KFX fragment representation.

```rust
pub struct Fragment {
    pub id: FragmentId,
    pub fragment_type: FragmentType,
    pub content: Vec<IonValue>,
}

pub enum FragmentType {
    Body,
    FrontMatter,
    BackMatter,
    Cover,
    Metadata,
}
```

### `kfx/serialization.rs`

Binary container format serialization.

```rust
pub fn serialize_container(container: &KfxContainer) -> io::Result<Vec<u8>> {
    // Serialize KFX container to bytes
}

pub fn deserialize_container(data: &[u8]) -> io::Result<KfxContainer> {
    // Deserialize bytes to KFX container
}
```

### `kfx/cover.rs`

Cover section detection and generation.

```rust
pub fn extract_cover(metadata: &Metadata) -> Option<CoverImage> {
    // Extract cover from metadata
}

pub struct CoverImage {
    pub resource: String,
    pub media_type: String,
}
```

### `kfx/auxiliary.rs`

Auxiliary data generation for navigation targets.

```rust
pub fn generate_auxiliary(landmarks: &[Landmark]) -> AuxiliaryData {
    // Generate auxiliary data from landmarks
}

pub struct AuxiliaryData {
    pub landmarks: Vec<Landmark>,
    pub cover_target: Option<String>,
    pub start_target: Option<String>,
}
```

## KFX Import Process

```
1. Parse KFX ZIP container
   ↓
2. Read metadata fragment
   ↓
3. Read content fragments
   ↓
4. Tokenize Ion documents
   ↓
5. Build IR from tokens
   ↓
6. Extract resources (images, fonts)
   ↓
7. Parse auxiliary data (landmarks)
```

## KFX Export Process

```
1. Normalize all chapters to IR
   ↓
2. Build style registry (deduplicate styles)
   ↓
3. Register resources (images, fonts)
   ↓
4. Index anchors for link resolution
   ↓
5. Convert IR to KFX tokens
   ↓
6. Serialize tokens to Ion documents
   ↓
7. Create content fragments
   ↓
8. Generate metadata fragment
   ↓
9. Generate auxiliary data
   ↓
10. Write KFX ZIP container
```

## Style Deduplication

KFX uses style deduplication to reduce file size:

```rust
let mut registry = StyleRegistry::new();

// Register styles from all chapters
for chapter in chapters {
    for node in chapter.nodes {
        let style = chapter.styles.get(node.style)?;
        registry.register(style)?;
    }
}

// Export style registry
let styles = registry.export_styles();
```

## Resource Management

Resources are referenced by ID in KFX:

```rust
pub struct ResourceId(pub u32);

pub struct Resource {
    pub id: ResourceId,
    pub path: String,
    pub data: Vec<u8>,
    pub media_type: String,
}
```

## Link Resolution

KFX uses entity-based linking:

```rust
pub struct Entity {
    pub id: String,
    pub entity_type: EntityType,
    pub target: Option<String>,
}

pub enum EntityType {
    Chapter,
    Anchor,
    Resource,
}
```

## KFX vs AZW3

| Feature | KFX | AZW3 |
|---------|-----|------|
| Format | Ion binary | MOBI database |
| Styles | Style registry | Inline CSS |
| Metadata | Separate fragment | EXTH header |
| Layout | Advanced typography | Basic formatting |
| Compression | Deflate | PalmDoc compression |

## KFX Limitations

1. **Proprietary format**: Only works on Kindle devices
2. **Limited tooling**: Few editors/viewers support KFX
3. **Complex format**: Ion binary format is complex to parse
4. **Version changes**: Format evolves with Kindle updates

## Debugging KFX

### Dump KFX Structure

```bash
# Dump KFX as JSON
kfx-dump book.kfx

# Dump with Ion pretty-printing
kfx-dump --pretty book.kfx
```

### Inspect Ion Values

```rust
let ion = parse_ion(&data)?;
println!("{:#?}", ion);
```

### Validate Style Registry

```rust
let registry = build_style_registry(&chapters)?;
for (id, style) in registry.styles.iter().enumerate() {
    println!("Style {}: {}", id, style.to_css_string());
}
```

## Further Reading

- [Amazon Ion Specification](https://amzn.github.io/ion-docs/)
- [Kindle File Format Documentation](https://kdp.amazon.com/en_US/help/topic/G200646480)
- `src/kfx/ion.rs` - Ion parser implementation
- `src/kfx/schema.rs` - KFX ↔ IR mapping
