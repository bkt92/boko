//! Analyze INDX records from reference MOBI file

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut file = File::open("tests/fixtures/test_book.mobi")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    // Parse PDB header
    if data.len() < 78 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "File too short"));
    }

    let num_records = u16::from_be_bytes([data[76], data[77]]);
    println!("Total records: {}", num_records);

    // Parse record offsets
    let records_start = 78;
    let mut record_offsets = Vec::new();
    for i in 0..num_records as usize {
        let pos = records_start + i * 8;
        let offset = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        record_offsets.push(offset);
    }

    // Get record sizes
    let mut record_sizes = Vec::new();
    for i in 0..num_records as usize {
        let start = record_offsets[i] as usize;
        let end = if i + 1 < num_records as usize {
            record_offsets[i + 1] as usize
        } else {
            data.len()
        };
        record_sizes.push((end - start) as u32);
    }

    // Find where text records end and images start
    // Look for INDX magic
    let mut indx_records = Vec::new();
    for i in 0..num_records as usize {
        let start = record_offsets[i] as usize;
        let end = if i + 1 < num_records as usize {
            record_offsets[i + 1] as usize
        } else {
            data.len()
        };

        if start + 4 <= data.len() && &data[start..start + 4] == b"INDX" {
            indx_records.push((i, start, end - start));
        }
    }

    println!("\nFound {} INDX records:", indx_records.len());
    for (idx, start, size) in &indx_records {
        println!("  Record {}: offset={}, size={} bytes", idx, start, size);
    }

    // Analyze first INDX record structure
    if !indx_records.is_empty() {
        let (idx, start, _size) = indx_records[0];
        let indx_data = &data[start..start + 240]; // First INDX is 240 bytes

        println!("\n=== Record {} (INDX) detailed analysis ===", idx);
        println!("Magic: {:?}", String::from_utf8_lossy(&indx_data[0..4]));
        println!(
            "Header length: {}",
            u32::from_be_bytes([indx_data[4], indx_data[5], indx_data[6], indx_data[7]])
        );
        println!(
            "Header type: {}",
            u32::from_be_bytes([indx_data[8], indx_data[9], indx_data[10], indx_data[11]])
        );
        println!(
            "IDXT start: {}",
            u32::from_be_bytes([indx_data[20], indx_data[21], indx_data[22], indx_data[23]])
        );
        println!(
            "Entry count: {}",
            u32::from_be_bytes([indx_data[24], indx_data[25], indx_data[26], indx_data[27]])
        );
        println!(
            "Encoding: {}",
            u32::from_be_bytes([indx_data[28], indx_data[29], indx_data[30], indx_data[31]])
        );
        println!(
            "Total entries: {}",
            u32::from_be_bytes([indx_data[36], indx_data[37], indx_data[38], indx_data[39]])
        );
        println!(
            "ORDT offset: {}",
            u32::from_be_bytes([indx_data[40], indx_data[41], indx_data[42], indx_data[43]])
        );
        println!(
            "LIGT offset: {}",
            u32::from_be_bytes([indx_data[44], indx_data[45], indx_data[46], indx_data[47]])
        );
        println!(
            "Num LIGT: {}",
            u32::from_be_bytes([indx_data[48], indx_data[49], indx_data[50], indx_data[51]])
        );
        println!(
            "Num CNCX: {}",
            u32::from_be_bytes([indx_data[52], indx_data[53], indx_data[54], indx_data[55]])
        );
        println!(
            "TAGX offset: {}",
            u32::from_be_bytes([
                indx_data[180],
                indx_data[181],
                indx_data[182],
                indx_data[183]
            ])
        );

        // Check if TAGX is present
        let tagx_offset = u32::from_be_bytes([
            indx_data[180],
            indx_data[181],
            indx_data[182],
            indx_data[183],
        ]) as usize;
        if tagx_offset > 0 && tagx_offset + 12 <= 240 {
            println!("\nTAGX section at offset {}:", tagx_offset);
            let tagx_data = &indx_data[tagx_offset..];
            println!("  Magic: {:?}", String::from_utf8_lossy(&tagx_data[0..4]));
            println!(
                "  First entry offset: {}",
                u32::from_be_bytes([tagx_data[4], tagx_data[5], tagx_data[6], tagx_data[7]])
            );
            println!(
                "  Control byte count: {}",
                u32::from_be_bytes([tagx_data[8], tagx_data[9], tagx_data[10], tagx_data[11]])
            );
        }

        // Print hex dump of first 64 bytes
        println!("\nFirst 64 bytes (hex):");
        for i in 0..64 {
            if i % 16 == 0 {
                print!("{:04x}: ", i);
            }
            print!("{:02x} ", indx_data[i]);
            if i % 16 == 15 {
                println!();
            }
        }
    }

    // Analyze second INDX record if present
    if indx_records.len() >= 2 {
        let (idx, start, _size) = indx_records[1];
        let indx_data = &data[start..start + 368]; // Second INDX is 368 bytes

        println!("\n=== Record {} (INDX) detailed analysis ===", idx);
        println!("Magic: {:?}", String::from_utf8_lossy(&indx_data[0..4]));
        println!(
            "Header length: {}",
            u32::from_be_bytes([indx_data[4], indx_data[5], indx_data[6], indx_data[7]])
        );
        println!(
            "Header type: {}",
            u32::from_be_bytes([indx_data[8], indx_data[9], indx_data[10], indx_data[11]])
        );
        println!(
            "IDXT start: {}",
            u32::from_be_bytes([indx_data[20], indx_data[21], indx_data[22], indx_data[23]])
        );
        println!(
            "Entry count: {}",
            u32::from_be_bytes([indx_data[24], indx_data[25], indx_data[26], indx_data[27]])
        );

        // Print hex dump of first 64 bytes
        println!("\nFirst 64 bytes (hex):");
        for i in 0..64 {
            if i % 16 == 0 {
                print!("{:04x}: ", i);
            }
            print!("{:02x} ", indx_data[i]);
            if i % 16 == 15 {
                println!();
            }
        }
    }

    // Analyze records 119-122 (the extra records)
    println!("\n=== Records 119-122 (non-INDX records) ===");
    for idx in 119..123 {
        if idx >= num_records as usize {
            break;
        }

        let start = record_offsets[idx] as usize;
        let end = if idx + 1 < num_records as usize {
            record_offsets[idx + 1] as usize
        } else {
            data.len()
        };
        let size = end - start;

        println!("\nRecord {}: offset={}, size={} bytes", idx, start, size);

        // Print magic bytes if present
        if start + 4 <= data.len() {
            let magic = String::from_utf8_lossy(&data[start..start + 4]);
            println!("  First 4 bytes: {:?}", magic);
        }

        // Print first 64 bytes
        let record_data = &data[start..std::cmp::min(start + 64, end)];
        println!("  First 64 bytes (hex):");
        for i in 0..record_data.len() {
            if i % 16 == 0 {
                print!("    {:04x}: ", i);
            }
            print!("{:02x} ", record_data[i]);
            if i % 16 == 15 {
                println!();
            }
        }
        if record_data.len() % 16 != 0 {
            println!();
        }
    }

    // Check where images start
    println!("\n=== Finding image records ===");
    let mut image_count = 0;
    for idx in 0..num_records as usize {
        let start = record_offsets[idx] as usize;
        if start + 4 > data.len() {
            break;
        }

        // Check for JPEG magic
        if data[start] == 0xFF && data[start + 1] == 0xD8 {
            image_count += 1;
            if image_count <= 3 {
                println!("Record {}: JPEG image at offset {}", idx, start);
            }
        }
        // Check for PNG magic
        else if &data[start..start + 4] == b"\x89PNG" {
            image_count += 1;
            if image_count <= 3 {
                println!("Record {}: PNG image at offset {}", idx, start);
            }
        }
    }
    println!("Total images found: {}", image_count);

    Ok(())
}
