//! Compare IR before and after round-trip to find data loss

use std::fs::File;
use std::io::{self, Read};
use std::process::Command;

fn main() -> io::Result<()> {
    println!("=== IR Round-trip Comparison ===\n");

    // Step 1: Import EPUB to get original IR
    println!("Step 1: EPUB → IR (original)");
    let epub_to_ir = Command::new("target/release/bokocli")
        .args(&[
            "convert",
            "tests/fixtures/test_book.epub",
            "/tmp/original.mobi",
        ])
        .output()
        .expect("Failed to convert EPUB to MOBI");

    if !epub_to_ir.status.success() {
        eprintln!("Error: {}", String::from_utf8_lossy(&epub_to_ir.stderr));
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "EPUB to MOBI conversion failed",
        ));
    };

    // Now convert that MOBI back to EPUB (this should work)
    println!("Step 2: MOBI → EPUB (round-trip)");
    let mobi_epub_result = Command::new("target/release/bokocli")
        .args(&["convert", "/tmp/original.mobi", "/tmp/roundtrip.epub"])
        .output();

    let mobi_to_epub = match mobi_epub_result {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Failed to execute: {}", e);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Command execution failed",
            ));
        }
    };

    if !mobi_to_epub.status.success() {
        eprintln!("Error: {}", String::from_utf8_lossy(&mobi_to_epub.stderr));
        println!("\n❌ Round-trip failed as expected");
        println!("This confirms the PalmDoc decompression issue");
    } else {
        println!("✅ Round-trip succeeded");
    };

    // Now convert that MOBI back to EPUB (this should work)
    println!("Step 2: MOBI → EPUB (round-trip)");
    let mobi_to_epub = Command::new("target/release/bokocli")
        .args(&["convert", "/tmp/original.mobi", "/tmp/roundtrip.epub"])
        .output();

    let mobi_epub_result = mobi_to_epub;
    if !mobi_epub_result.status.success() {
        eprintln!(
            "Error: {}",
            String::from_utf8_lossy(&mobi_epub_result.stderr)
        );
        println!("\n❌ Round-trip failed as expected");
        println!("This confirms the PalmDoc decompression issue");
    } else {
        println!("✅ Round-trip succeeded");
    }

    // Compare file sizes
    let mut epub_data = Vec::new();
    File::open("tests/fixtures/test_book.epub")?.read_to_end(&mut epub_data);

    let mut mobi_data = Vec::new();
    File::open("/tmp/original.mobi")?.read_to_end(&mut mobi_data);

    println!("\n=== File Size Comparison ===");
    println!("Original EPUB: {} bytes", epub_data.len());
    println!("Generated MOBI: {} bytes", mobi_data.len());
    println!(
        "Size ratio: {:.2}%",
        (mobi_data.len() as f64 / epub_data.len() as f64) * 100.0
    );

    // Extract and compare HTML content
    println!("\n=== HTML Content Comparison ===");

    let original_html = Command::new("unzip")
        .args(&["-p", "tests/fixtures/test_book.epub", "OEBPS/content.html"])
        .output()
        .expect("Failed to extract original HTML");

    let original_html_len = original_html.stdout.len();
    println!("Original HTML: {} bytes", original_html_len);

    // Check MOBI metadata
    println!("\n=== MOBI Metadata ===");
    let info = Command::new("target/release/bokocli")
        .args(&["info", "/tmp/original.mobi"])
        .output()
        .expect("Failed to get MOBI info");

    let info_str = String::from_utf8_lossy(&info.stdout);
    for line in info_str.lines().take(15) {
        println!("  {}", line);
    }

    // Analyze the text length field
    let num_records = u16::from_be_bytes([mobi_data[76], mobi_data[77]]);
    let mobi_off =
        u32::from_be_bytes([mobi_data[78], mobi_data[79], mobi_data[80], mobi_data[81]]) as usize;
    let text_len = u32::from_be_bytes([
        mobi_data[mobi_off + 4],
        mobi_data[mobi_off + 5],
        mobi_data[mobi_off + 6],
        mobi_data[mobi_off + 7],
    ]);

    println!("\n=== MOBI Text Length Analysis ===");
    println!("Number of records: {}", num_records);
    println!("Text length field: {} bytes", text_len);
    println!("Original HTML: {} bytes", original_html_len);
    println!(
        "Difference: {} bytes",
        text_len as i64 - original_html_len as i64
    );

    // Calculate what the text length should be
    let _expected_len = if text_len > original_html_len as u32 {
        println!(
            "\n⚠️  Text length is {} bytes larger than HTML",
            text_len - original_html_len as u32
        );
        println!("   This will cause PalmDoc decompression to fail");
        println!("   The decompressor expects more bytes than available");
    } else {
        println!("\n✓ Text length is appropriate");
    };

    Ok(())
}
