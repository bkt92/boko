//! Markdown Exporter - I/O orchestration for markdown output.
//!
//! This module provides the thin I/O layer for exporting books to Markdown.
//! The actual rendering logic is in [`crate::markdown`].

use std::io::{self, Seek, Write};
use std::path::Path;

use crate::markdown::{build_heading_slugs, render_chapter};
use crate::model::Book;

use super::Exporter;

/// Configuration for Markdown export.
#[derive(Debug, Clone, Default)]
pub struct MarkdownConfig {
    /// Line width for wrapping (0 = no wrapping).
    /// Reserved for future use.
    pub line_width: usize,
}

/// Exporter for Markdown output.
#[derive(Debug, Clone, Default)]
pub struct MarkdownExporter {
    config: MarkdownConfig,
}

impl MarkdownExporter {
    /// Create a new MarkdownExporter with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a MarkdownExporter with the specified configuration.
    pub fn with_config(config: MarkdownConfig) -> Self {
        Self { config }
    }

    /// Export book to a markdown file, extracting images to `<filename>_img/` folder.
    ///
    /// When writing to a file (e.g., `output.md`), this method:
    /// - Writes the markdown content to the file
    /// - Creates an `output_img/` directory alongside it
    /// - Extracts all image assets into that directory
    /// - Rewrites image references in the markdown to point to `output_img/<filename>`
    pub fn export_to_path(&self, book: &mut Book, output_path: &Path) -> io::Result<()> {
        let _ = self.config;

        // 1. Resolve all links
        let resolved = book.resolve_links()?;

        let spine: Vec<_> = book.spine().to_vec();

        // 2. Load all chapters and build heading slugs
        let chapters: Vec<_> = spine
            .iter()
            .map(|e| Ok((e.id, book.load_chapter_cached(e.id)?)))
            .collect::<io::Result<Vec<_>>>()?;

        let heading_slugs = build_heading_slugs(&chapters, &resolved);

        // 3. Collect image assets and build path mapping
        let img_dir = build_img_dir_path(output_path);
        let assets: Vec<_> = book.list_assets().to_vec();
        let image_assets: Vec<_> = assets.iter().filter(|p| is_image_path(p)).collect();

        // Build old_path -> new_filename mapping
        let mut img_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (i, asset_path) in image_assets.iter().enumerate() {
            let path_str = asset_path.to_string_lossy();
            let ext = Path::new(&*path_str)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin");
            let new_name = format!("image_{:04}.{}", i, ext);
            img_map.insert(path_str.to_string(), new_name.clone());

            // Also map by just the filename portion
            if let Some(fname) = asset_path.file_name() {
                let fname_str = fname.to_string_lossy().to_string();
                if fname_str != path_str {
                    img_map.insert(fname_str, new_name);
                }
            }
        }

        // 4. Render chapters, rewriting image paths
        let mut md_content = String::new();
        let mut first = true;
        for (chapter_id, chapter) in &chapters {
            if !first {
                md_content.push('\n');
                md_content.push_str("---\n\n");
            }
            first = false;

            let result = render_chapter(chapter, *chapter_id, &resolved, &heading_slugs);
            md_content.push_str(&result.content);

            if !result.footnotes.is_empty() {
                md_content.push('\n');
                for note in &result.footnotes {
                    md_content.push_str(&format!("[^{}]: {}\n", note.number, note.content));
                }
            }
        }

        // Rewrite image references: ![alt](old_path) -> ![alt](img_dir/filename)
        let img_dir_name = img_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("images");
        for (old_path, new_name) in &img_map {
            // Match both ![alt](old_path) and ![alt](old_path with quotes)
            let old_ref = format!("]({})", old_path);
            let new_ref = format!("]({}/{})", img_dir_name, new_name);
            md_content = md_content.replace(&old_ref, &new_ref);
        }

        // 5. Create image directory and write images
        if !image_assets.is_empty() {
            std::fs::create_dir_all(&img_dir)?;
        }
        for asset_path in &image_assets {
            let path_str = asset_path.to_string_lossy();
            if let Some(new_name) = img_map.get(&*path_str)
                && let Ok(data) = book.load_asset(asset_path)
            {
                let img_path = img_dir.join(new_name);
                std::fs::write(&img_path, &data)?;
            }
        }

        // 6. Write markdown file
        std::fs::write(output_path, &md_content)?;

        Ok(())
    }
}

impl Exporter for MarkdownExporter {
    fn export<W: Write + Seek>(&self, book: &mut Book, writer: &mut W) -> io::Result<()> {
        let _ = self.config;

        // 1. Resolve all links (I/O: loads chapters internally)
        let resolved = book.resolve_links()?;

        let spine: Vec<_> = book.spine().to_vec();

        // 2. Load all chapters and build heading slugs
        let chapters: Vec<_> = spine
            .iter()
            .map(|e| Ok((e.id, book.load_chapter_cached(e.id)?)))
            .collect::<io::Result<Vec<_>>>()?;

        let heading_slugs = build_heading_slugs(&chapters, &resolved);

        // 3. Render each chapter (pure) and write (I/O)
        let mut first = true;
        for (chapter_id, chapter) in &chapters {
            if !first {
                // Chapter separator
                writeln!(writer)?;
                writeln!(writer, "---")?;
                writeln!(writer)?;
            }
            first = false;

            // Pure rendering
            let result = render_chapter(chapter, *chapter_id, &resolved, &heading_slugs);

            // I/O: write content
            write!(writer, "{}", result.content)?;

            // I/O: write footnotes
            if !result.footnotes.is_empty() {
                writeln!(writer)?;
                for note in &result.footnotes {
                    writeln!(writer, "[^{}]: {}", note.number, note.content)?;
                }
            }
        }

        Ok(())
    }
}

/// Build the image directory path from the output file path.
/// e.g., `output.md` -> `output_img/`
fn build_img_dir_path(output_path: &Path) -> std::path::PathBuf {
    let stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let parent = output_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}_img", stem))
}

/// Check if a path looks like an image file.
fn is_image_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "svg" | "webp" | "bmp" | "tiff" | "tif"
    )
}

// Unit tests for rendering are in `markdown/render.rs`.
// Integration tests using real EPUB files are in `tests/`.
