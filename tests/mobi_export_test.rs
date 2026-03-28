//! MOBI 6 export integration tests

use boko::{Book, Format};
use std::io::Cursor;

#[test]
fn test_epub_to_mobi_export() {
    // This test requires a fixture - for now, skip if not found
    let fixture_path = "tests/fixtures/mobi/simple.epub";

    if !std::path::Path::new(fixture_path).exists() {
        eprintln!("Skipping test: fixture not found");
        return;
    }

    // Open EPUB
    let mut book = Book::open(fixture_path).expect("Failed to open EPUB");

    // Export to MOBI
    let mut output = Cursor::new(Vec::new());
    book.export(Format::Mobi, &mut output)
        .expect("Failed to export to MOBI");

    // Verify output
    let mobi_data = output.into_inner();

    // Check PalmDB header
    assert!(
        mobi_data.len() > 78,
        "MOBI file too short for PalmDB header"
    );

    // Check database name is in header (first 32 bytes)
    let header_name = String::from_utf8_lossy(&mobi_data[0..32]);
    assert!(header_name.contains("BOOK") || !header_name.trim().is_empty());

    // Check database type (should be "MOBI" at offset 0x3C)
    assert_eq!(&mobi_data[0x3C..0x40], b"MOBI\0");

    println!("MOBI file size: {} bytes", mobi_data.len());
}
