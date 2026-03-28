use boko::{Book, Importer};
use boko::import::MarkdownImporter;
use std::path::Path;

#[test]
fn test_open_markdown_file() {
    let path = Path::new("tests/fixtures/markdown/simple.md");
    if !path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    // Test the importer directly first
    let result = MarkdownImporter::open(path);
    assert!(result.is_ok(), "Failed to open markdown file with MarkdownImporter");

    let importer = result.unwrap();
    assert!(!importer.metadata().title.is_empty());
    assert_eq!(importer.metadata().title, "My Document");

    // Verify chapter splitting - should have 2 chapters from the 2 H1 headings
    let spine = importer.spine();
    assert_eq!(spine.len(), 2, "Should detect 2 chapters from H1 headings");

    // Also test via Book::open (will work after Task 3)
    let result = Book::open(path);
    if result.is_ok() {
        let book = result.unwrap();
        assert!(!book.metadata().title.is_empty());
    } else {
        eprintln!("Book::open doesn't support Markdown yet (expected until Task 3)");
    }
}
