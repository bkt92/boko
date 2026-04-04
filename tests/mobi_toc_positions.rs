use boko::{Book, Format};
use std::io::Cursor;

#[test]
fn mobi_export_produces_valid_toc() {
    let mut book = Book::open("tests/fixtures/test_book.epub").unwrap();
    let toc_before = book.toc().to_vec();
    assert!(
        !toc_before.is_empty(),
        "Source EPUB should have TOC entries"
    );

    let mobi_path = "/tmp/test_mobi_toc_positions.mobi";
    {
        let mut cursor = Cursor::new(Vec::new());
        book.export(Format::Mobi, &mut cursor).unwrap();
        std::fs::write(mobi_path, cursor.into_inner()).unwrap();
    }

    let mobi = Book::open(mobi_path).unwrap();
    let toc_after = mobi.toc().to_vec();

    println!("Original TOC: {} entries", toc_before.len());
    println!("MOBI TOC: {} entries", toc_after.len());

    // MOBI reader should extract at least the top-level TOC entry
    assert!(
        !toc_after.is_empty(),
        "MOBI should have at least some TOC entries"
    );

    // Verify TOC entries have distinct, non-empty hrefs
    let mut seen_hrefs = std::collections::HashSet::new();
    for (i, entry) in toc_after.iter().enumerate() {
        println!("  [{}] {} -> {}", i, entry.title, entry.href);
        assert!(
            !entry.href.is_empty(),
            "TOC entry [{}] '{}' should have a non-empty href",
            i,
            entry.title
        );
        assert!(
            seen_hrefs.insert(entry.href.clone()),
            "TOC entry [{}] '{}' has duplicate href '{}'",
            i,
            entry.title,
            entry.href
        );
    }

    // Verify the MOBI file has reasonable size (content wasn't lost)
    let mobi_size = std::fs::metadata(mobi_path).unwrap().len();
    assert!(
        mobi_size > 1000,
        "MOBI file should be non-trivial, got {} bytes",
        mobi_size
    );
    println!("MOBI file size: {} bytes", mobi_size);
}

#[test]
fn mobi_export_preserves_all_chapters() {
    let mut book = Book::open("tests/fixtures/test_book.epub").unwrap();
    let spine_before = book.spine().to_vec();
    assert!(!spine_before.is_empty(), "Source EPUB should have chapters");

    let mobi_path = "/tmp/test_mobi_chapters.mobi";
    {
        let mut cursor = Cursor::new(Vec::new());
        book.export(Format::Mobi, &mut cursor).unwrap();
        std::fs::write(mobi_path, cursor.into_inner()).unwrap();
    }

    // Verify the MOBI file was written successfully and can be opened
    let mobi = Book::open(mobi_path).unwrap();
    let metadata = mobi.metadata();
    assert!(!metadata.title.is_empty(), "MOBI should have a title");

    // Verify spine is preserved
    let spine_after = mobi.spine().to_vec();
    assert_eq!(
        spine_after.len(),
        spine_before.len(),
        "MOBI should preserve chapter count. Got {}, expected {}",
        spine_after.len(),
        spine_before.len()
    );

    println!(
        "Chapters preserved: {} before, {} after",
        spine_before.len(),
        spine_after.len()
    );
}
