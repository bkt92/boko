//! Markdown format importer.

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::import::{ChapterId, Importer, SpineEntry};
use crate::io::ByteSource;
use crate::model::{
    AnchorTarget, Chapter, GlobalNodeId, Landmark, Metadata, Node, NodeId, Role, TextRange, TocEntry,
};
use crate::style::{ComputedStyle, FontStyle, FontWeight};

/// Markdown format importer.
pub struct MarkdownImporter {
    path: PathBuf,
    content: String,
    metadata: Metadata,
    toc: Vec<TocEntry>,
    spine: Vec<SpineEntry>,
    #[allow(dead_code)]
    assets: HashMap<String, Vec<u8>>,
    chapter_ranges: Vec<ChapterRange>,
    #[allow(dead_code)]
    anchor_map: HashMap<String, GlobalNodeId>,
    #[allow(dead_code)]
    asset_paths: Vec<PathBuf>,
    #[allow(dead_code)]
    config: MarkdownConfig,
}

/// Configuration for Markdown import.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MarkdownConfig {
    #[allow(dead_code)]
    pub split_on_h1: bool,
    #[allow(dead_code)]
    pub extract_front_matter: bool,
    #[allow(dead_code)]
    pub enable_gfm: bool,
    #[allow(dead_code)]
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

/// Builder for constructing IR from Markdown events.
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
            config: MarkdownConfig::default(),
        }
    }

    /// Create a MarkdownImporter from in-memory bytes.
    pub fn from_source(source: Arc<dyn ByteSource>) -> io::Result<Self> {
        let len = source.len() as usize;
        let data = source.read_at(0, len)?;
        let content = String::from_utf8(data)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 in Markdown"))?;

        let mut importer = Self::new(PathBuf::from("memory.md"), content);
        importer.scan_headings();
        importer.build_metadata();

        Ok(importer)
    }

    fn scan_headings(&mut self) {
        use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};

        let parser = Parser::new(&self.content);
        let mut chapter_index = 0;

        // If no headings, create single chapter
        let mut found_heading = false;

        for (event, range) in parser.into_offset_iter() {
            if let Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) = event {
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
        use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};

        let parser = Parser::new(&self.content);
        let mut in_heading = false;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) => {
                    in_heading = true;
                }
                Event::Text(text) if in_heading => {
                    return Some(text.to_string());
                }
                _ => {}
            }
        }
        None
    }

    fn handle_start_tag(&self, builder: &mut IrBuilder, tag: pulldown_cmark::Tag) {
        use pulldown_cmark::Tag;

        match tag {
            Tag::Paragraph => {
                builder.push_node(Role::Paragraph);
            }
            Tag::Heading { level, .. } => {
                builder.push_node(Role::Heading(level as u8));
            }
            Tag::BlockQuote(_) => {
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

    fn handle_end_tag(&self, builder: &mut IrBuilder, tag: pulldown_cmark::TagEnd) {
        use pulldown_cmark::TagEnd;

        match tag {
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::Table
            | TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::TableCell => {
                builder.pop_node();
            }
            TagEnd::Emphasis | TagEnd::Strong => {
                // Already handled in start_tag
            }
            TagEnd::Link | TagEnd::Image => {
                // Handled directly in event loop
                builder.pop_node();
            }
            _ => {}
        }
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

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
        Ok(self.content.as_bytes()[range.start..range.end].to_vec())
    }

    fn list_assets(&self) -> &[PathBuf] {
        &self.asset_paths
    }

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

    fn font_faces(&mut self) -> Vec<crate::model::FontFace> {
        Vec::new()
    }

    fn index_anchors(&mut self, _chapters: &[(ChapterId, std::sync::Arc<crate::model::Chapter>)]) {
        // TODO: Build anchor map
    }

    fn resolve_href(&self, _from_chapter: ChapterId, href: &str) -> Option<AnchorTarget> {
        let href = href.trim();
        if href.starts_with("http://") || href.starts_with("https://") {
            return Some(AnchorTarget::External(href.to_string()));
        }
        None
    }

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
                Event::Start(tag) => {
                    // Handle Link and Image directly to access &mut self for asset tracking
                    match &tag {
                        pulldown_cmark::Tag::Link { dest_url, .. } => {
                            let url = dest_url.to_string();
                            let node_id = builder.push_node(Role::Link);
                            builder.chapter.semantics.set_href(node_id, &url);
                        }
                        pulldown_cmark::Tag::Image { dest_url, .. } => {
                            let url = dest_url.to_string();
                            let node_id = builder.push_node(Role::Image);
                            builder.chapter.semantics.set_src(node_id, &url);

                            // Track asset if not external
                            if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("data:") {
                                let base_dir = self.path.parent().unwrap_or(Path::new("."));
                                let full_path = base_dir.join(&url);
                                if !self.asset_paths.contains(&full_path) {
                                    self.asset_paths.push(full_path.clone());

                                    // Try to load asset
                                    if let Ok(bytes) = std::fs::read(&full_path) {
                                        self.assets.insert(url, bytes);
                                    }
                                }
                            }
                        }
                        _ => {
                            // Handle other tags through handle_start_tag
                            self.handle_start_tag(&mut builder, tag);
                        }
                    }
                }
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
}
