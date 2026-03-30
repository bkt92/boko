//! Debug PalmDoc compression/decompression

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut data = Vec::new();
    File::open("/tmp/test.mobi")?.read_to_end(&mut data)?;

    let mobi_off = u32::from_be_bytes([data[78], data[79], data[80], data[81]]) as usize;
    let text_count = u16::from_be_bytes([data[mobi_off + 8], data[mobi_off + 9]]) as usize;

    println!("=== PalmDoc Decompression Debug ===\n");
    println!("Text record count: {}", text_count);
    println!("Text length field: {} bytes\n",
        u32::from_be_bytes([
            data[mobi_off + 4],
            data[mobi_off + 5],
            data[mobi_off + 6],
            data[mobi_off + 7],
        ])
    );

    // Try to decompress each text record
    use boko::mobi::palmdoc;

    let mut total_decompressed = 0;
    for i in 1..=text_count.min(3) {
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
            u32::from_be_bytes([
                data[78 + 117 * 8],
                data[78 + 117 * 8 + 1],
                data[78 + 117 * 8 + 2],
                data[78 + 117 * 8 + 3],
            ]) as usize
        };

        let record_data = &data[start..end];
        println!("Record {}:", i);
        println!("  Compressed: {} bytes", record_data.len());

        match palmdoc::decompress(record_data) {
            Ok(decompressed) => {
                println!("  Decompressed: {} bytes", decompressed.len());
                total_decompressed += decomcompressed.len();

                // Show first 100 chars
                let preview = String::from_utf8_lossy(&decompressed[..100.min(decompressed.len())]);
                println!("  Preview: {}", preview);
            }
            Err(e) => {
                println!("  ERROR: {}", e);
                return Err(e);
            }
        }
        println!();
    }

    println!("Total decompressed so far: {} bytes", total_decompressed);
    println!("Expected total: {} bytes",
        u32::from_be_bytes([
            data[mobi_off + 4],
            data[mobi_off + 5],
            data[mobi_off + 6],
            data[mobi_off + 7],
        ])
    );

    Ok(())
}
