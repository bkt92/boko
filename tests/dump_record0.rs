//! Dump Record 0 content for comparison

use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Usage: dump_record0 <mobi_file>");
        std::process::exit(1);
    }

    let path = &args[0];
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;

    // Get Record 0 info
    let mobi_offset = u32::from_be_bytes([data[78], data[79], data[80], data[81]]) as usize;
    let rec1_offset = u32::from_be_bytes([data[78 + 8], data[78 + 9], data[78 + 8 + 2], data[78 + 8 + 3]]) as usize;

    let rec0_size = rec1_offset - mobi_offset;
    let rec0_data = &data[mobi_offset..rec1_offset];

    println!("=== Record 0 Analysis: {} ===\n", path);
    println!("Record 0 offset: 0x{:04X} ({} bytes)", mobi_offset, mobi_offset);
    println!("Record 1 offset: 0x{:04X} ({} bytes)", rec1_offset, rec1_offset);
    println!("Record 0 size: {} bytes (0x{:04X})\n", rec0_size, rec0_size);

    // Parse MOBI header
    println!("=== MOBI Header Fields ===");
    println!("MOBI signature (first 4 bytes): {:?}", &rec0_data[..4]);
    println!("Compression: {}", u16::from_be_bytes([rec0_data[0], rec0_data[1]]));

    let text_length = u32::from_be_bytes([rec0_data[4], rec0_data[5], rec0_data[6], rec0_data[7]]);
    println!("Text length: {} bytes", text_length);

    let header_len = u32::from_be_bytes([rec0_data[20], rec0_data[21], rec0_data[22], rec0_data[23]]);
    println!("Header length: {} bytes (0x{:04X})", header_len, header_len);

    let mobi_version = u32::from_be_bytes([rec0_data[104], rec0_data[105], rec0_data[106], rec0_data[107]]);
    println!("MOBI version: {}", mobi_version);

    // EXTH flags at 0x80
    let exth_flags = u32::from_be_bytes([rec0_data[0x80], rec0_data[0x81], rec0_data[0x82], rec0_data[0x83]]);
    println!("EXTH flags: 0x{:08X}", exth_flags);

    // Extra flags at 0xF2
    let extra_flags = u16::from_be_bytes([rec0_data[0xF2], rec0_data[0xF3]]);
    println!("Extra flags: 0x{:04X}", extra_flags);

    // Check for EXTH
    if exth_flags & 0x40 != 0 {
        let exth_offset = header_len as usize;
        println!("\n=== EXTH Header (at offset 0x{:04X}) ===", exth_offset);

        if exth_offset + 12 <= rec0_data.len() {
            let sig = String::from_utf8_lossy(&rec0_data[exth_offset..exth_offset + 4]);
            println!("Signature: {:?}", sig);

            if sig == "EXTH" {
                let exth_len = u32::from_be_bytes([
                    rec0_data[exth_offset + 4],
                    rec0_data[exth_offset + 5],
                    rec0_data[exth_offset + 6],
                    rec0_data[exth_offset + 7],
                ]);
                let exth_count = u32::from_be_bytes([
                    rec0_data[exth_offset + 8],
                    rec0_data[exth_offset + 9],
                    rec0_data[exth_offset + 10],
                    rec0_data[exth_offset + 11],
                ]);
                println!("EXTH length: {} bytes", exth_len);
                println!("EXTH count: {} records", exth_count);

                // Show first few records
                let mut pos = exth_offset + 12;
                for i in 0..exth_count.min(10) {
                    if pos + 8 <= rec0_data.len() {
                        let rec_type = u32::from_be_bytes([
                            rec0_data[pos],
                            rec0_data[pos + 1],
                            rec0_data[pos + 2],
                            rec0_data[pos + 3],
                        ]);
                        let rec_len = u32::from_be_bytes([
                            rec0_data[pos + 4],
                            rec0_data[pos + 5],
                            rec0_data[pos + 6],
                            rec0_data[pos + 7],
                        ]);

                        if pos + 8 + (rec_len as usize) <= rec0_data.len() {
                            let rec_data = &rec0_data[pos + 8..pos + 8 + (rec_len as usize) - 8];
                            println!("  Record {}: type={}, len={}, data={:?}",
                                i, rec_type, rec_len,
                                String::from_utf8_lossy(rec_data).chars().take(50).collect::<String>());
                            pos += rec_len as usize;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Show what's after MOBI header + EXTH
    let after_exth = header_len as usize;
    if after_exth < rec0_data.len() {
        println!("\n=== Data after MOBI header ===");
        println!("Offset 0x{:04X}: First 100 bytes:", after_exth);
        for i in 0..100.min(rec0_data.len() - after_exth) {
            if i % 16 == 0 {
                print!("  {:04X}: ", after_exth + i);
            }
            print!("{:02X} ", rec0_data[after_exth + i]);
            if (i + 1) % 16 == 0 {
                println!();
            }
        }
        if rec0_data.len() - after_exth > 100 {
            println!("  ... ({} more bytes)", rec0_data.len() - after_exth - 100);
        }
    }

    Ok(())
}
