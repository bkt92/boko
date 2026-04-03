//! Export module for writing ebooks to various formats.
//!
//! Provides the `Exporter` trait and format-specific implementations.
//!
//! # Architecture
//!
//! The `Exporter` trait uses a builder pattern:
//! - `new()` creates an exporter with default configuration
//! - `with_config()` allows customization
//! - `export()` writes to any `Write + Seek` destination
//!
//! # Example
//!
//! ```no_run
//! use boko::{Book, Format};
//! use boko::export::{EpubExporter, Exporter};
//! use std::fs::File;
//!
//! let mut book = Book::open("input.azw3")?;
//! let mut file = File::create("output.epub")?;
//!
//! // Using the exporter directly
//! EpubExporter::new().export(&mut book, &mut file)?;
//!
//! // Or using the Book convenience method
//! // book.export(Format::Epub, &mut file)?;
//! # Ok::<(), std::io::Error>(())
//! ```

use std::io::{self, Seek, Write};
use std::path::Path;

use crate::model::Book;

mod azw3;
mod css_gen;
mod epub;
mod html_synth;
mod kfx;
mod mobi;
mod normalize;
mod text;

pub use azw3::{Azw3Config, Azw3Exporter};
pub use css_gen::{CssArtifact, generate_css, generate_css_all};
pub use epub::{EpubConfig, EpubExporter};
pub use html_synth::{
    SynthesisResult, escape_xml, escape_xml_into, synthesize_html, synthesize_html_with_class_list,
    synthesize_xhtml_document, synthesize_xhtml_document_with_class_list,
};
pub use kfx::{KfxConfig, KfxExporter};
pub use mobi::{MobiConfig, MobiEncoding, MobiExporter};
pub use normalize::{ChapterContent, GlobalStylePool, NormalizedContent, normalize_book};
pub use text::{MarkdownConfig, MarkdownExporter};

/// Trait for exporting books to specific formats.
///
/// Exporters use a builder pattern where configuration is held in the struct,
/// and the `export` method writes to any `Write + Seek` destination.
pub trait Exporter {
    /// Export the book to the provided writer.
    ///
    /// The writer can be:
    /// - `std::fs::File` for disk output
    /// - `Vec<u8>` for in-memory output
    /// - `std::io::Cursor<Vec<u8>>` for seekable in-memory output
    /// - Any other type implementing `Write + Seek`
    fn export<W: Write + Seek>(&self, book: &mut Book, writer: &mut W) -> io::Result<()>;
}

/// Resolve the cover image path from metadata against the available asset paths.
///
/// Uses progressively broader matching:
/// 1. Direct match after normalizing separators and case
/// 2. Suffix match (one path is a parent of the other)
/// 3. Filename-only match as last resort
///
/// Returns the index into `asset_paths` that matches the cover, or `None`.
pub fn resolve_cover_asset(cover_image: Option<&str>, asset_paths: &[impl AsRef<Path>]) -> Option<usize> {
    let cover = cover_image?;
    let cover_norm = cover.replace('\\', "/").to_lowercase();
    let cover_filename = Path::new(cover)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if cover_norm.is_empty() {
        return None;
    }

    // Pass 1: Direct or suffix match
    for (i, asset) in asset_paths.iter().enumerate() {
        let asset_str = asset.as_ref().to_string_lossy();
        let asset_norm = asset_str.replace('\\', "/").to_lowercase();

        if asset_norm == cover_norm {
            return Some(i);
        }
        if asset_norm.ends_with(&format!("/{cover_norm}"))
            || cover_norm.ends_with(&format!("/{asset_norm}"))
        {
            return Some(i);
        }
    }

    // Pass 2: Filename-only match (handles cross-format path differences)
    if !cover_filename.is_empty() {
        for (i, asset) in asset_paths.iter().enumerate() {
            let asset_filename = asset
                .as_ref()
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if asset_filename == cover_filename {
                return Some(i);
            }
        }
    }

    None
}
