//! Detailed MOBI structure analysis

use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Usage: analyze_mobi_structure <mobi_file>");
        std::process::exit(1);
    }

    let path = &args[0];
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;

    println!("=== Analyzing: {} ===\n", path);
    println!("File size: {} bytes\n", data.len());

    // PalmDB header
    let num_records = u16::from_be_bytes([data[76], data[77]]);
    let mobi_offset = u32::from_be_bytes([data[78], data[79], data[80], data[81]]) as usize;

    println!("=== PalmDB Header ===");
    println!(
        "Database name: {:?}",
        String::from_utf8_lossy(&data[0..32]).trim_end_matches('\0')
    );
    println!("Number of records: {}", num_records);
    println!(
        "MOBI header offset: 0x{:04X} ({} bytes)\n",
        mobi_offset, mobi_offset
    );

    // MOBI header
    println!("=== MOBI Header (at offset 0x{:04X}) ===", mobi_offset);
    let compression = u16::from_be_bytes([data[mobi_offset], data[mobi_offset + 1]]);
    let text_length = u32::from_be_bytes([
        data[mobi_offset + 4],
        data[mobi_offset + 5],
        data[mobi_offset + 6],
        data[mobi_offset + 7],
    ]);
    let text_record_count = u16::from_be_bytes([data[mobi_offset + 8], data[mobi_offset + 9]]);
    let text_record_size = u16::from_be_bytes([data[mobi_offset + 10], data[mobi_offset + 11]]);
    let mobi_header_len = u32::from_be_bytes([
        data[mobi_offset + 20],
        data[mobi_offset + 21],
        data[mobi_offset + 22],
        data[mobi_offset + 23],
    ]);
    let mobi_version = u32::from_be_bytes([
        data[mobi_offset + 104],
        data[mobi_offset + 105],
        data[mobi_offset + 106],
        data[mobi_offset + 107],
    ]);

    println!(
        "Compression: {} ({}=PalmDoc)",
        compression,
        if compression == 2 { "2" } else { "?" }
    );
    println!("Text length: {} bytes", text_length);
    println!("Text record count: {}", text_record_count);
    println!("Text record size: {} bytes", text_record_size);
    println!("MOBI header length: {} bytes", mobi_header_len);
    println!("MOBI version: {}\n", mobi_version);

    // EXTH flags
    let exth_flags = u32::from_be_bytes([
        data[mobi_offset + 0x80],
        data[mobi_offset + 0x81],
        data[mobi_offset + 0x82],
        data[mobi_offset + 0x83],
    ]);

    // Extra flags
    let extra_flags = u16::from_be_bytes([data[mobi_offset + 0xF2], data[mobi_offset + 0xF3]]);

    println!("EXTH flags: 0x{:08X}", exth_flags);
    println!("Extra flags (0xF2): 0x{:04X}\n", extra_flags);

    // Check for EXTH header
    if exth_flags & 0x40 != 0 {
        let exth_offset = mobi_offset + mobi_header_len as usize;
        if exth_offset + 4 < data.len() {
            let exth_sig = String::from_utf8_lossy(&data[exth_offset..exth_offset + 4]);
            println!("EXTH signature: {:?}", exth_sig);

            if exth_sig == "EXTH" {
                let exth_len = u32::from_be_bytes([
                    data[exth_offset + 4],
                    data[exth_offset + 5],
                    data[exth_offset + 6],
                    data[exth_offset + 7],
                ]);
                let exth_count = u32::from_be_bytes([
                    data[exth_offset + 8],
                    data[exth_offset + 9],
                    data[exth_offset + 10],
                    data[exth_offset + 11],
                ]);
                println!("EXTH length: {} bytes", exth_len);
                println!("EXTH record count: {}\n", exth_count);
            }
        }
    }

    // Record info list analysis
    println!("=== Record Info List ===");
    println!("Record offsets start at byte 78\n");

    let mut last_end = mobi_offset;
    for i in 0..(num_records as usize).min(20) {
        let offset = u32::from_be_bytes([
            data[78 + i * 8],
            data[78 + i * 8 + 1],
            data[78 + i * 8 + 2],
            data[78 + i * 8 + 3],
        ]) as usize;

        let attrs = data[78 + i * 8 + 4];
        let unique_id = u32::from_be_bytes([
            0,
            data[78 + i * 8 + 5],
            data[78 + i * 8 + 6],
            data[78 + i * 8 + 7],
        ]);

        let size = if i + 1 < (num_records as usize) {
            let next_offset = u32::from_be_bytes([
                data[78 + (i + 1) * 8],
                data[78 + (i + 1) * 8 + 1],
                data[78 + (i + 1) * 8 + 2],
                data[78 + (i + 1) * 8 + 3],
            ]) as usize;
            next_offset - offset
        } else {
            data.len() - offset
        };

        println!(
            "Record {}: offset=0x{:06X} ({}), size={}, attrs=0x{:02X}, id={}",
            i, offset, offset, size, attrs, unique_id
        );

        if i < 10 {
            // Show first few bytes
            let preview_len = 16.min(data.len() - offset);
            let preview = &data[offset..offset + preview_len];
            println!("  First bytes: {:?}\n", preview);
        }

        last_end = offset + size;
    }

    println!("Total calculated size: {} bytes", last_end);
    println!("Actual file size: {} bytes", data.len());

    Ok(())
}
