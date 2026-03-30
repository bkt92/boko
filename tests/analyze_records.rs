//! Analyze record structure in detail

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut file = File::open("tests/fixtures/test_book.mobi")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    // Parse PDB header
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
    let mut record_info = Vec::new();
    for i in 0..num_records as usize {
        let start = record_offsets[i] as usize;
        let end = if i + 1 < num_records as usize {
            record_offsets[i + 1] as usize
        } else {
            data.len()
        };
        let size = end - start;

        let record_type = if i == 0 {
            "MOBI header"
        } else if start + 4 <= data.len() {
            let magic = &data[start..start + 4];
            if magic == b"INDX" {
                "INDX"
            } else if data[start] == 0xFF && data[start + 1] == 0xD8 {
                "JPEG"
            } else if &data[start..start + 4] == b"\x89PNG" {
                "PNG"
            } else if size < 4096 * 2 {
                "Text"
            } else {
                "Other"
            }
        } else {
            "Unknown"
        };

        record_info.push((i, start, size, record_type));
    }

    // Print summary
    println!("\n=== Record Summary ===");
    let mut text_count = 0;
    let mut indx_count = 0;
    let mut image_count = 0;
    let mut other_count = 0;

    for (idx, _start, _size, rtype) in &record_info {
        match *rtype {
            "Text" => text_count += 1,
            "INDX" => indx_count += 1,
            "JPEG" | "PNG" => image_count += 1,
            _ => other_count += 1,
        }
    }

    println!("Record 0: MOBI header");
    println!("Records 1-{:?}: {} text records", text_count, text_count);
    println!("Records {:?}-{:?}: {} INDX records",
             record_info.iter().find(|(_, _, _, t)| *t == "INDX").map(|(i, _, _, _)| i).unwrap_or(&0),
             indx_count, indx_count);
    println!("Records {:?}-{:?}: {} image records",
             record_info.iter().find(|(_, _, _, t)| *t == "JPEG" || *t == "PNG").map(|(i, _, _, _)| i).unwrap_or(&0),
             image_count, image_count);

    // Print detailed breakdown
    println!("\n=== Detailed Record Breakdown ===");
    for (idx, start, size, rtype) in &record_info {
        if *idx == 0 {
            println!("Record 0: MOBI header, offset={}, size={} bytes", start, size);
        } else if *rtype == "INDX" {
            println!("Record {}: {}, offset={}, size={} bytes", idx, rtype, start, size);
        } else if *rtype == "JPEG" || *rtype == "PNG" {
            println!("Record {}: {}, offset={}, size={} bytes", idx, rtype, start, size);
        }
    }

    // Print first and last text records
    println!("\n=== Text Records ===");
    let first_text_idx = record_info.iter().position(|(_, _, _, t)| *t == "Text").unwrap_or(1);
    let last_text_idx = record_info.iter().rposition(|(_, _, _, t)| *t == "Text").unwrap_or(1);
    println!("First text record: {}", first_text_idx);
    println!("Last text record: {}", last_text_idx);
    println!("Total text records: {}", text_count);

    // Print record 119 specifically
    println!("\n=== Record 119 (CNCX strings) ===");
    let idx = 119;
    if idx < record_info.len() {
        let (_i, start, size, _rtype) = record_info[idx];
        println!("Record 119: offset={}, size={} bytes", start, size);

        // Print as UTF-8 text
        if let Ok(text) = std::str::from_utf8(&data[start..start + size]) {
            println!("Text content: {}", text);
        }

        // Print hex dump
        println!("Hex dump (first 128 bytes):");
        for i in 0..std::cmp::min(128, size) {
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

    Ok(())
}
