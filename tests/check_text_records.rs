//! Check text record sizes in reference MOBI

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut data = Vec::new();
    File::open("tests/fixtures/test_book.mobi")?.read_to_end(&mut data)?;

    let num_records = u16::from_be_bytes([data[76], data[77]]) as usize;

    // Get MOBI header offset
    let mobi_off = u32::from_be_bytes([data[78], data[79], data[80], data[81]]) as usize;

    // Get text length
    let text_length = u32::from_be_bytes([
        data[mobi_off + 4],
        data[mobi_off + 5],
        data[mobi_off + 6],
        data[mobi_off + 7],
    ]);

    // Get text record count
    let text_record_count = u16::from_be_bytes([
        data[mobi_off + 8],
        data[mobi_off + 9],
    ]);

    // Get text record size
    let text_record_size = u16::from_be_bytes([
        data[mobi_off + 10],
        data[mobi_off + 11],
    ]);

    println!("Text length (uncompressed): {}", text_length);
    println!("Text record count: {}", text_record_count);
    println!("Text record size: {}", text_record_size);
    println!("Total uncompressed size: {}", text_record_count as u32 * text_record_size as u32);
    println!("Difference: {}", (text_record_count as u32 * text_record_size as u32) - text_length);

    // Check first few text record sizes
    println!("\nText record sizes:");
    for i in 1..=5.min(text_record_count as usize) {
        let start = u32::from_be_bytes([
            data[78 + i * 8],
            data[78 + i * 8 + 1],
            data[78 + i * 8 + 2],
            data[78 + i * 8 + 3],
        ]) as usize;

        let end = if i + 1 < num_records {
            u32::from_be_bytes([
                data[78 + (i + 1) * 8],
                data[78 + (i + 1) * 8 + 1],
                data[78 + (i + 1) * 8 + 2],
                data[78 + (i + 1) * 8 + 3],
            ]) as usize
        } else {
            data.len()
        };

        println!("  Record {}: {} bytes", i, end - start);
    }

    Ok(())
}
