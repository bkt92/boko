//! Decompress and inspect reference MOBI text records

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut data = Vec::new();
    File::open("tests/fixtures/test_book.mobi")?.read_to_end(&mut data)?;

    let num_records = u16::from_be_bytes([data[76], data[77]]) as usize;
    let mobi_off = u32::from_be_bytes([data[78], data[79], data[80], data[81]]) as usize;

    let text_len = u32::from_be_bytes([
        data[mobi_off + 4],
        data[mobi_off + 5],
        data[mobi_off + 6],
        data[mobi_off + 7],
    ]) as usize;

    let text_count = u16::from_be_bytes([
        data[mobi_off + 8],
        data[mobi_off + 9],
    ]) as usize;

    println!("Text length: {} bytes", text_len);
    println!("Text record count: {}", text_count);

    // Decompress all text records
    use boko::mobi::palmdoc;

    let mut decompressed = Vec::new();
    for i in 1..=text_count {
        let start = u32::from_be_bytes([
            data[78 + i * 8],
            data[78 + i * 8 + 1],
            data[78 + i * 8 + 2],
            data[78 + i * 8 + 3],
        ]) as usize;

        let end = if i + 1 <= text_count {
            u32::from_be_bytes([
                data[78 + (i + 1) * 8],
                data[78 + (i + 1) * 8 + 1],
                data[78 + (i + 1) * 8 + 2],
                data[78 + (i + 1) * 8 + 3],
            ]) as usize
        } else {
            // Next record is INDX (record 117)
            u32::from_be_bytes([
                data[78 + 117 * 8],
                data[78 + 117 * 8 + 1],
                data[78 + 117 * 8 + 2],
                data[78 + 117 * 8 + 3],
            ]) as usize
        };

        let record_data = &data[start..end];
        let record_decompressed = palmdoc::decompress(record_data).unwrap();
        decompressed.extend_from_slice(&record_decompressed);
    }

    println!("\nDecompressed size: {} bytes", decompressed.len());
    println!("Matches text length: {}", decompressed.len() == text_len);

    // Show first 500 chars
    println!("\nFirst 500 chars of decompressed text:");
    let preview = String::from_utf8_lossy(&decompressed[..500.min(decompressed.len())]);
    println!("{}", preview);

    // Check for HTML tags
    let has_html_tag = decompressed.windows(6).any(|w| w == b"<html>");
    let has_head_tag = decompressed.windows(6).any(|w| w == b"<head>");
    let has_body_tag = decompressed.windows(6).any(|w| w == b"<body>");

    println!("\nHTML tags present:");
    println!("  <html>: {}", has_html_tag);
    println!("  <head>: {}", has_head_tag);
    println!("  <body>: {}", has_body_tag);

    Ok(())
}
