#![allow(clippy::many_single_char_names)]

use boko::{Book, Format};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Read EPUB TOC
    println!("=== Step 1: EPUB TOC ===");
    let mut book_epub = Book::open("tests/fixtures/test_book.epub")?;
    let toc_epub = book_epub.toc().to_vec();
    println!("EPUB TOC: {} entries", toc_epub.len());
    fn print_toc(entries: &[boko::TocEntry], depth: usize) {
        for (i, entry) in entries.iter().enumerate() {
            let indent = "  ".repeat(depth);
            println!("{}[{}] title='{}' href='{}' target={:?}",
                indent, i, entry.title, entry.href, entry.target);
            print_toc(&entry.children, depth + 1);
        }
    }
    print_toc(&toc_epub, 0);

    // Step 2: Export to MOBI
    println!("\n=== Step 2: EPUB → MOBI ===");
    let mobi_path = "/tmp/test_toc_roundtrip.mobi";
    {
        let mut f = std::fs::File::create(mobi_path)?;
        book_epub.export(Format::Mobi, &mut f)?;
    }
    println!("MOBI written: {} bytes", std::fs::metadata(&mobi_path)?.len());

    // Step 3: Read MOBI TOC
    println!("\n=== Step 3: MOBI TOC ===");
    let mut book_mobi = Book::open(mobi_path)?;
    let toc_mobi = book_mobi.toc().to_vec();
    println!("MOBI TOC: {} entries", toc_mobi.len());
    print_toc(&toc_mobi, 0);

    // Step 4: Resolve MOBI links
    println!("\n=== Step 4: MOBI resolved links ===");
    let resolved_mobi = book_mobi.resolve_links()?;
    println!("MOBI links resolved, {} broken",
        resolved_mobi.broken_links().len());

    // Step 5: Export MOBI to EPUB
    println!("\n=== Step 5: MOBI → EPUB ===");
    let epub_path2 = "/tmp/test_toc_roundtrip.epub";
    {
        let mut f = std::fs::File::create(epub_path2)?;
        book_mobi.export(Format::Epub, &mut f)?;
    }
    println!("EPUB written: {} bytes", std::fs::metadata(&epub_path2)?.len());

    // Step 6: Read roundtrip EPUB TOC
    println!("\n=== Step 6: Roundtrip EPUB TOC ===");
    let mut book_epub2 = Book::open(epub_path2)?;
    let toc_epub2 = book_epub2.toc().to_vec();
    println!("Roundtrip EPUB TOC: {} entries", toc_epub2.len());
    print_toc(&toc_epub2, 0);

    // Step 7: Compare TOCs
    println!("\n=== Comparison ===");
    println!("Original EPUB: {} entries", toc_epub.len());
    println!("MOBI: {} entries", toc_mobi.len());
    println!("Roundtrip EPUB: {} entries", toc_epub2.len());

    println!("\nOriginal EPUB titles:");
    for entry in &toc_epub {
        println!("  - {}", entry.title);
    }

    println!("\nMOBI titles:");
    for entry in &toc_mobi {
        println!("  - {}", entry.title);
    }

    println!("\nRoundtrip EPUB titles:");
    for entry in &toc_epub2 {
        println!("  - {}", entry.title);
    }

    println!("\nRoundtrip EPUB hrefs:");
    for entry in &toc_epub2 {
        println!("  - {} (target: {:?})", entry.href, entry.target);
    }

    Ok(())
}
