//! Simple IR round-trip test

use std::fs::File;
use std::io::{self, Read};
use std::process::Command;

fn main() -> io::Result<()> {
    println!("=== IR Round-trip Test ===\n");

    // Step 1: EPUB → MOBI
    println!("Step 1: Converting EPUB to MOBI...");
    let output1 = Command::new("target/release/bokocli")
        .args(&["convert", "tests/fixtures/test_book.epub", "/tmp/test.mobi"])
        .output()?;

    if !output1.status.success() {
        eprintln!("Error: {}", String::from_utf8_lossy(&output1.stderr));
        return Err(io::Error::new(io::ErrorKind::Other, "Conversion failed"));
    }
    println!("✓ EPUB → MOBI successful\n");

    // Step 2: MOBI → EPUB (round-trip)
    println!("Step 2: Converting MOBI back to EPUB...");
    let output2 = Command::new("target/release/bokocli")
        .args(&["convert", "/tmp/test.mobi", "/tmp/roundtrip.epub"])
        .output()?;

    if !output2.status.success() {
        eprintln!("Error: {}", String::from_utf8_lossy(&output2.stderr));
        println!("\n❌ Round-trip failed (as expected)");
        println!("This confirms the PalmDoc decompression issue");
    } else {
        println!("✓ Round-trip successful");
    }

    // Step 3: Analyze the generated MOBI
    println!("\n=== Analyzing Generated MOBI ===");
    let mut mobi_data = Vec::new();
    File::open("/tmp/test.mobi")?.read_to_end(&mut mobi_data)?;

    let num_records = u16::from_be_bytes([mobi_data[76], mobi_data[77]]);
    let mobi_off = u32::from_be_bytes([mobi_data[78], mobi_data[79], mobi_data[80], mobi_data[81]]) as usize;

    let text_len = u32::from_be_bytes([
        mobi_data[mobi_off + 4],
        mobi_data[mobi_off + 5],
        mobi_data[mobi_off + 6],
        mobi_data[mobi_off + 7],
    ]);

    println!("Number of records: {}", num_records);
    println!("Text length field: {} bytes", text_len);

    // Get original HTML size
    let html_output = Command::new("unzip")
        .args(&["-p", "tests/fixtures/test_book.epub", "OEBPS/content.html"])
        .output()?;

    let html_len = html_output.stdout.len();
    println!("Original HTML size: {} bytes", html_len);
    println!("Difference: {} bytes", text_len as i64 - html_len as i64);

    // The issue
    if text_len as i64 > html_len as i64 {
        println!("\n⚠️  PROBLEM FOUND:");
        println!("   Text length field is {} bytes larger than actual HTML", text_len - html_len as u32);
        println!("   PalmDoc decompressor will expect {} bytes", text_len);
        println!("   But compressed data only contains {} bytes", html_len);
        println!("   This causes 'unexpected end of input' error");
    }

    Ok(())
}
