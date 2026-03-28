# Style System

The style system handles CSS parsing, cascade, and computed styles for nodes in the IR. It uses `cssparser` and `selectors` crates for standards-compliant CSS processing.

## Overview

```
CSS Source → Stylesheet → Cascade → ComputedStyle → StylePool
```

## Stylesheet Parsing

```rust
use boko::dom::Stylesheet;

let css = r#"
    p { color: blue; }
    .highlight { font-weight: bold; }
    "#;

let stylesheet = Stylesheet::parse(css);
```

### CSS Rules

```rust
pub struct CssRule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
    pub specificity: Specificity,
}

pub struct Declaration {
    pub property: String,
    pub value: String,
    pub important: bool,
}
```

### Selector Types

- Type selectors: `p`, `div`, `h1`
- Class selectors: `.class`, `p.class`
- ID selectors: `#id`
- Attribute selectors: `[href]`, `[type="text"]`
- Pseudo-classes: `:hover`, `:first-child`
- Combinators: ` `, `>`, `+`, `~`

### Specificity

```rust
pub struct Specificity {
    pub id: u32,        // ID selectors
    pub class: u32,     // Class, attribute, pseudo-class
    pub element: u32,   // Element, pseudo-element
}
```

Specificity is calculated as: `(id, class, element)`

Example:
- `p`: `(0, 0, 1)`
- `.class`: `(0, 1, 0)`
- `#id`: `(1, 0, 0)`
- `p.class`: `(0, 1, 1)`
- `p#id.class`: `(1, 1, 1)`

## CSS Cascade

The cascade applies styles in order:

1. User-agent stylesheet (defaults)
2. User stylesheet (not implemented)
3. Author stylesheets (linked + inline)
4. `!important` rules override normal rules

```rust
pub fn compute_styles(
    dom: &ArenaDom,
    stylesheets: &[(Stylesheet, Origin)],
) -> HashMap<ArenaNodeId, ComputedStyle> {
    let mut styles = HashMap::new();

    // Apply UA styles
    let ua = user_agent_stylesheet();
    apply_styles(&mut styles, dom, &[(ua, Origin::UserAgent)]);

    // Apply author styles
    apply_styles(&mut styles, dom, stylesheets);

    // Inherit from parents
    inherit_styles(&mut styles, dom);

    styles
}
```

### Origin

```rust
pub enum Origin {
    UserAgent,  // Browser defaults
    User,       // User preferences
    Author,     // Document styles
}
```

## Computed Style

```rust
pub struct ComputedStyle {
    // Font
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
    pub border_bottom_width: Length,
    pub border_bottom_color: Color,
    pub border_bottom_style: BorderStyle,
    pub border_left_width: Length,
    pub border_left_color: Color,
    pub border_left_style: BorderStyle,
    pub border_right_width: Length,
    pub border_right_color: Color,
    pub border_right_style: BorderStyle,

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

## Property Types

### Length

```rust
pub enum Length {
    Px(f32),       // Pixels
    Em(f32),       // EM (relative to font size)
    Rem(f32),      // REM (relative to root font size)
    Percent(f32),  // Percentage
    Auto,          // Auto value
}
```

### Color

```rust
pub enum Color {
    Named(String),      // "red", "blue", etc.
    Rgb(u8, u8, u8),    // rgb(255, 0, 0)
    Rgba(u8, u8, u8, f32),  // rgba(255, 0, 0, 0.5)
    Hsl(u8, u8, u8),    // hsl(0, 100%, 50%)
    Hsla(u8, u8, u8, f32),  // hsla(0, 100%, 50%, 0.5)
    Hex(String),        // #ff0000, #f00
    CurrentColor,       // currentcolor keyword
    Transparent,        // transparent keyword
}
```

### Font Weight

```rust
pub enum FontWeight {
    Normal,
    Bold,
    Bolder,
    Lighter,
    Value(u16),  // 100-900
}
```

### Font Style

```rust
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}
```

### Display

```rust
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    None,
    ListItem,
    Table,
    TableRow,
    TableCell,
    // ... more values
}
```

### Text Align

```rust
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justify,
}
```

### Text Decoration

```rust
pub struct TextDecorationValue {
    pub line: TextDecorationLine,
    pub style: TextDecorationStyle,
    pub color: Option<Color>,
}

pub enum TextDecorationLine {
    None,
    Underline,
    Overline,
    LineThrough,
    Blink,
}

pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}
```

### List Style Type

```rust
pub enum ListStyleType {
    None,
    Disc,
    Circle,
    Square,
    Decimal,
    LowerRoman,
    UpperRoman,
    LowerAlpha,
    UpperAlpha,
    // ... more values
}
```

## Style Pool

Styles are deduplicated in a `StylePool`:

```rust
pub struct StylePool {
    styles: Vec<ComputedStyle>,
}

impl StylePool {
    pub fn insert(&mut self, style: ComputedStyle) -> StyleId {
        // Check if style already exists
        for (id, existing) in self.styles.iter().enumerate() {
            if styles_equal(existing, &style) {
                return StyleId(id);
            }
        }

        // Insert new style
        let id = StyleId(self.styles.len());
        self.styles.push(style);
        id
    }

    pub fn get(&self, id: StyleId) -> Option<&ComputedStyle> {
        self.styles.get(id.0)
    }
}
```

## Style ID

```rust
pub struct StyleId(pub u32);

impl Default for StyleId {
    fn default() -> Self {
        StyleId(0)  // Style ID 0 is the default style
    }
}
```

## ToCss Trait

Convert properties back to CSS:

```rust
pub trait ToCss {
    fn to_css(&self, buf: &mut String);
    fn to_css_string(&self) -> String;
}
```

Example:

```rust
let style = ComputedStyle { /* ... */ };
let css = style.to_css_string();
// "color: rgb(0, 0, 255); font-weight: bold;"
```

## User-Agent Stylesheet

Default styles for HTML elements:

```rust
pub fn user_agent_stylesheet() -> Stylesheet {
    let css = r#"
        html, body {
            display: block;
            font-family: serif;
            color: black;
            background-color: white;
            margin: 0;
            padding: 0;
        }

        h1, h2, h3, h4, h5, h6 {
            display: block;
            font-weight: bold;
            margin-top: 1em;
            margin-bottom: 0.5em;
        }

        h1 { font-size: 2em; }
        h2 { font-size: 1.5em; }
        h3 { font-size: 1.17em; }
        h4 { font-size: 1em; }
        h5 { font-size: 0.83em; }
        h6 { font-size: 0.67em; }

        p {
            display: block;
            margin-top: 1em;
            margin-bottom: 1em;
        }

        a {
            color: blue;
            text-decoration: underline;
        }

        img {
            display: inline;
        }

        ul, ol {
            display: block;
            margin-top: 1em;
            margin-bottom: 1em;
            padding-left: 2em;
        }

        li {
            display: list-item;
        }

        table {
            display: table;
            border-collapse: separate;
            border-spacing: 2px;
        }

        td, th {
            display: table-cell;
            padding: 1px;
        }
    "#;

    Stylesheet::parse(css)
}
```

## Inheritance

Some properties inherit from parent nodes:

```rust
fn inherit_styles(styles: &mut HashMap<ArenaNodeId, ComputedStyle>, dom: &ArenaDom) {
    for node_id in dom.iter() {
        if let Some(parent_id) = dom.parent(node_id) {
            if let Some(parent_style) = styles.get(&parent_id) {
                let node_style = styles.entry(node_id).or_default();

                // Inherit inherited properties
                node_style.font_family = parent_style.font_family.clone();
                node_style.font_size = parent_style.font_size;
                node_style.color = parent_style.color;
                node_style.line_height = parent_style.line_height;
                node_style.text_align = parent_style.text_align;
                // ... more inherited properties
            }
        }
    }
}
```

### Inherited Properties

- `font-family`
- `font-size`
- `font-style`
- `font-variant`
- `font-weight`
- `line-height`
- `color`
- `text-align`
- `text-indent`
- `text-transform`
- `white-space`
- `visibility`
- `list-style-type`
- `list-style-position`

### Non-Inherited Properties

- `display`
- `margin-*`
- `padding-*`
- `border-*`
- `background-color`
- `width`
- `height`
- `float`
- `clear`
- `position`

## Style Modules

### `style/parse/stylesheet.rs`

Stylesheet parsing:

```rust
pub fn parse_stylesheet(css: &str) -> Stylesheet {
    // Parse CSS into rules
    // Calculate specificity
    // Return Stylesheet
}
```

### `style/parse/properties.rs`

Property value parsing:

```rust
pub fn parse_color(value: &str) -> Option<Color> {
    // Parse color value
}

pub fn parse_length(value: &str) -> Option<Length> {
    // Parse length value
}

pub fn parse_font_weight(value: &str) -> Option<FontWeight> {
    // Parse font weight
}
```

### `style/cascade.rs`

CSS cascade implementation:

```rust
pub fn compute_styles(
    dom: &ArenaDom,
    stylesheets: &[(Stylesheet, Origin)],
) -> HashMap<NodeId, ComputedStyle> {
    // Apply stylesheets in order
    // Calculate specificity
    // Resolve !important
    // Return computed styles
}
```

### `style/style_pool.rs`

Style deduplication:

```rust
pub struct StylePool {
    styles: Vec<ComputedStyle>,
    index: HashMap<StyleKey, StyleId>,
}
```

### `style/to_css.rs`

CSS serialization:

```rust
pub fn style_to_css(style: &ComputedStyle) -> String {
    // Convert style to CSS string
}
```

## Style System Usage

### Parse CSS

```rust
let css = r#"
    p {
        color: blue;
        font-weight: bold;
    }
"#;

let stylesheet = Stylesheet::parse(css);
```

### Apply Styles to DOM

```rust
let stylesheets = &[(stylesheet, Origin::Author)];
let styles = compute_styles(&dom, stylesheets);
```

### Get Computed Style

```rust
let style = styles.get(&node_id).unwrap();
println!("Color: {:?}", style.color);
```

### Serialize to CSS

```rust
let css = style.to_css_string();
println!("CSS: {}", css);
```

## Style System Best Practices

1. **Use specificity correctly**: More specific selectors override less specific ones
2. **Minimize !important**: Use specificity instead
3. **Group common styles**: Use class selectors for shared styles
4. **Leverage inheritance**: Don't repeat inherited properties
5. **Use shorthand properties**: `margin: 1em 2em;` instead of four properties
6. **Validate input**: Reject invalid CSS during parsing
7. **Cache computed styles**: Avoid re-computing for the same node

## Debugging Styles

### Inspect Computed Style

```rust
let style = chapter.styles.get(node_id)?;
println!("Style: {}", style.to_css_string());
```

### List All Styles

```bash
# Dump style pool
boko dump book.epub --styles-only
```

### Trace Style Application

```rust
// Enable debug logging
env_logger::init();

let styles = compute_styles(&dom, &stylesheets);
// Logs will show which rules match which elements
```

## Further Reading

- [CSS Selectors Level 3](https://www.w3.org/TR/selectors-3/)
- [CSS Cascading and Inheritance Level 3](https://www.w3.org/TR/css-cascade-3/)
- `src/style/parse/stylesheet.rs` - Stylesheet parsing
- `src/style/cascade.rs` - CSS cascade implementation
