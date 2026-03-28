//! Markdown format importer.

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::import::{ChapterId, Importer, SpineEntry};
use crate::model::{
    AnchorTarget, GlobalNodeId, Landmark, Metadata, TocEntry,
};

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
}
