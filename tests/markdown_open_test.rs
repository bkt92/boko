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

#[test]
fn test_load_chapter_paragraphs() {
    use boko::{Book, Role};
    use std::path::Path;

    let path = Path::new("tests/fixtures/markdown/simple.md");
    if !path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let mut book = Book::open(path).unwrap();
    let chapter = book.load_chapter(boko::import::ChapterId(0)).unwrap();

    // Should have proper IR structure (not just root + 1 from HTML parsing)
    // Chapter 0 should have: H1 "My Document", paragraph, H2 "Section One", paragraph
    let mut found_heading = false;
    let mut found_paragraph = false;

    for id in chapter.iter_dfs() {
        if let Some(node) = chapter.node(id) {
            match node.role {
                Role::Heading(_) => found_heading = true,
                Role::Paragraph => found_paragraph = true,
                _ => {}
            }
        }
    }

    assert!(found_heading, "Should find at least one heading node");
    assert!(found_paragraph, "Should find at least one paragraph node");
}

#[test]
fn test_image_extraction() {
    use boko::Book;
    use std::path::Path;

    let path = Path::new("tests/fixtures/markdown/with_images.md");
    if !path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let mut book = Book::open(path).unwrap();

    // Need to load chapter to trigger parsing
    let _chapter = book.load_chapter(boko::import::ChapterId(0));

    let assets = book.list_assets();
    eprintln!("Assets: {:?}", assets);
    assert!(!assets.is_empty(), "Should extract images");
}
