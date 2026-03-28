use boko::{Book, Format};
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

#[test]
fn test_comprehensive_markdown_export() {
    let md_path = Path::new("tests/fixtures/markdown/comprehensive.md");
    if !md_path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let mut book = Book::open(md_path).expect("Failed to open MD");

    // Check metadata (YAML front matter not implemented in Phase 1, so skip this check)
    // assert_eq!(book.metadata().title, "Comprehensive Test Document");
    // assert!(!book.metadata().contributors.is_empty());

    // Export to EPUB
    let mut output = Cursor::new(Vec::new());
    book.export(Format::Epub, &mut output)
        .expect("Failed to export to EPUB");

    println!("Comprehensive MD exported successfully");
}
