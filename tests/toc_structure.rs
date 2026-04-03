#![allow(clippy::many_single_char_names)]

use boko::Book;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Original EPUB
    println!("=== Step 1: Original EPUB TOC ===");
    let mut ep1 = Book::open("tests/fixtures/test_book.epub")?;
    let toc1 = ep1.toc().to_vec();
    println!("EPUB TOC: {} entries", toc1.len());
    for (i, e) in toc1.iter().enumerate() {
        println!("  [{}] {} ({} children) -> {}", i, e.title, e.children.len(), e.href);
    }

    // EPUB → MOBI
    println!("\n=== Step 2: EPUB → MOBI ===");
    let mobi_path = "/tmp/test_toc.mobi";
    {
        use std::io::Cursor;
        let mut cursor = Cursor::new(Vec::new());
        ep1.export(boko::Format::Mobi, &mut cursor)?;
        std::fs::write(&mobi_path, cursor.into_inner())?;
    }
    let mut mobi = Book::open(mobi_path)?;
    let toc2 = mobi.toc().to_vec();
    println!("MOBI TOC: {} entries", toc2.len());
    for (i, e) in toc2.iter().enumerate() {
        println!("  [{}] {} ({} children) -> {}", i, e.title, e.children.len(), e.href);
    }

    // MOBI → EPUB
    println!("\n=== Step 3: MOBI → EPUB ===");
    let epub2_path = "/tmp/test_toc_epub.epub";
    {
        use std::io::Cursor;
        let mut cursor = Cursor::new(Vec::new());
        mobi.export(boko::Format::Epub, &mut cursor)?;
        std::fs::write(&epub2_path, cursor.into_inner())?;
    }
    let mut epub2 = Book::open(epub2_path)?;
    let toc3 = epub2.toc().to_vec();
    println!("Roundtrip EPUB TOC: {} entries", toc3.len());
    for (i, e) in toc3.iter().enumerate() {
        println!("  [{}] {} ({} children) -> {}", i, e.title, e.children.len(), e.href);
    }

    // Comparison
    println!("\n=== Step 4: Comparison ===");
    println!("Original EPUB: {} entries", toc1.len());
    println!("MOBI:        {} entries", toc2.len());
    println!("Roundtrip EPUB: {} entries", toc3.len());

    // Check hierarchy
    println!("\n=== Step 5: Hierarchy Check ===");
    println!("Original EPUB entries with children:");
    for (i, e) in toc1.iter().enumerate() {
        if !e.children.is_empty() {
            println!("  [{}] {}", i, e.title);
        }
    }
    println!("\nRoundtrip EPUB entries with children:");
    for (i, e) in toc3.iter().enumerate() {
        if !e.children.is_empty() {
            println!("  [{}] {}", i, e.title);
        }
    }

    // Check href preservation
    println!("\n=== Step 6: Href Check ===");
    let orig_hrefs: Vec<_> = toc1.iter().map(|e| e.href.clone()).collect();
    let roundtrip_hrefs: Vec<_> = toc3.iter().map(|e| e.href.clone()).collect();
    println!("Original hrefs count: {}", orig_hrefs.len());
    println!("Roundtrip hrefs count: {}", roundtrip_hrefs.len());
    for (i, (orig, round)) in orig_hrefs.iter().zip(roundtrip_hrefs.iter()).enumerate() {
        println!("  [{}] orig='{}' roundtrip='{}'", i, orig, round);
    }

    Ok(())
}
