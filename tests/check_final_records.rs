//! Check records 142-144

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut file = File::open("tests/fixtures/test_book.mobi")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let num_records = u16::from_be_bytes([data[76], data[77]]) as usize;
    println!("Total records: {}", num_records);

    // Parse record offsets
    let records_start = 78;
    let mut record_offsets = Vec::new();
    for i in 0..num_records {
        let pos = records_start + i * 8;
        let offset = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        record_offsets.push(offset);
    }

    // Check records 142-144
    for idx in 142..num_records {
        let start = record_offsets[idx] as usize;
        let end = if idx + 1 < num_records {
            record_offsets[idx + 1] as usize
        } else {
            data.len()
        };
        let size = end - start;

        println!("\n=== Record {} ===", idx);
        println!("Offset: {}", start);
        println!("Size: {} bytes", size);

        // Check magic bytes
        if start + 4 <= data.len() {
            let magic = &data[start..start + 4];
            println!("First 4 bytes: {:?}", magic);

            // Check for known types
            if magic == b"INDX" {
                println!("Type: INDX");
            } else if data[start] == 0xFF && data[start + 1] == 0xD8 {
                println!("Type: JPEG");
            } else if &data[start..start + 4] == b"\x89PNG" {
                println!("Type: PNG");
            }
        }

        // Print first 64 bytes
        println!("First 64 bytes (hex):");
        for i in 0..std::cmp::min(64, size) {
            if i % 16 == 0 {
                print!("{:04x}: ", i);
            }
            print!("{:02x} ", data[start + i]);
            if i % 16 == 15 {
                println!();
            }
        }
        if size % 16 != 0 {
            println!();
        }
    }

    // Also print record count breakdown
    println!("\n=== Record Count Summary ===");
    println!("Record 0: MOBI header");
    println!("Records 1-116: Text records (116 records)");
    println!("Record 117: INDX header");
    println!("Record 118: INDX entries");
    println!("Record 119: CNCX strings");
    println!("Records 120-141: Images (22 records)");
    println!("Records 142-144: ??? (3 records)");
    println!("\nTotal: 1 + 116 + 2 + 1 + 22 + 3 = 145 ✓");

    Ok(())
}
