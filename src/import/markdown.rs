//! Markdown format importer.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::import::SpineEntry;
use crate::model::{GlobalNodeId, Metadata, TocEntry};

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
            config: MarkdownConfig::default(),
        }
    }
}
