//! Compare MOBI file headers in detail

use std::fs::File;
use std::io::Read;
use std::process::Command;

fn parse_mobi_header(data: &[u8]) -> (u32, u16, u32, u32, u32, u32) {
    // Get MOBI header offset from PalmDB
    let mobi_off = u32::from_be_bytes([data[78], data[79], data[80], data[81]]) as usize;

    // Parse MOBI header fields
    let compression = u16::from_be_bytes([data[mobi_off], data[mobi_off + 1]]);
    let text_length = u32::from_be_bytes([
        data[mobi_off + 4],
        data[mobi_off + 5],
        data[mobi_off + 6],
        data[mobi_off + 7],
    ]);
    let text_record_count = u16::from_be_bytes([data[mobi_off + 8], data[mobi_off + 9]]);
    let text_record_size = u16::from_be_bytes([data[mobi_off + 10], data[mobi_off + 11]]);
    let mobi_header_len = u32::from_be_bytes([
        data[mobi_off + 20],
        data[mobi_off + 21],
        data[mobi_off + 22],
        data[mobi_off + 23],
    ]);
    let mobi_version = u32::from_be_bytes([
        data[mobi_off + 104],
        data[mobi_off + 105],
        data[mobi_off + 106],
        data[mobi_off + 107],
    ]);

    (
        mobi_off as u32,
        compression,
        text_length,
        text_record_count as u32,
        text_record_size as u32,
        mobi_version,
    )
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.len() < 2 {
        eprintln!("Usage: compare_mobi_headers <reference.mobi> <generated.mobi>");
        eprintln!("\nExample:");
        eprintln!("  compare_mobi_headers tests/fixtures/test_book.mobi /tmp/test.mobi");
        std::process::exit(1);
    }

    let ref_path = &args[0];
    let gen_path = &args[1];

    // Read both files
    let mut ref_data = Vec::new();
    let mut gen_data = Vec::new();

    File::open(ref_path)?.read_to_end(&mut ref_data)?;
    File::open(gen_path)?.read_to_end(&mut gen_data)?;

    println!("=== MOBI Header Comparison ===\n");
    println!("Reference: {}", ref_path);
    println!("Generated: {}", gen_path);
    println!("File sizes: {} vs {} bytes\n",
        ref_data.len(),
        gen_data.len()
    );

    // Parse headers
    let (ref_off, ref_comp, ref_len, ref_count, ref_size, ref_ver) = parse_mobi_header(&ref_data);
    let (gen_off, gen_comp, gen_len, gen_count, gen_size, gen_ver) = parse_mobi_header(&gen_data);

    println!("=== PalmDB Header ===");
    println!("MOBI offset: 0x{:X} vs 0x{:X}", ref_off, gen_off);

    println!("\n=== MOBI Header ===");
    println!("Compression: {} vs {}", ref_comp, gen_comp);
    println!("Text length: {} vs {}", ref_len, gen_len);
    println!("Text record count: {} vs {}", ref_count, gen_count);
    println!("Text record size: {} vs {}", ref_size, gen_size);
    println!("MOBI header length: {} vs {}", ref_len, gen_len);
    println!("MOBI version: {} vs {}", ref_ver, gen_ver);

    // Get PalmDB record count
    let ref_num_records = u16::from_be_bytes([ref_data[76], ref_data[77]]);
    let gen_num_records = u16::from_be_bytes([gen_data[76], gen_data[77]]);

    println!("\n=== PalmDB Records ===");
    println!("Record count: {} vs {}", ref_num_records, gen_num_records);

    // Check first few record offsets
    println!("\n=== First 10 Record Offsets ===");
    for i in 0..10.min(ref_num_records as usize).min(gen_num_records as usize) {
        let ref_off = u32::from_be_bytes([
            ref_data[78 + i * 8],
            ref_data[78 + i * 8 + 1],
            ref_data[78 + i * 8 + 2],
            ref_data[78 + i * 8 + 3],
        ]);

        let gen_off = u32::from_be_bytes([
            gen_data[78 + i * 8],
            gen_data[78 + i * 8 + 1],
            gen_data[78 + i * 8 + 2],
            gen_data[78 + i * 8 + 3],
        ]);

        println!("  Record {}: 0x{:06X} vs 0x{:06X}", i, ref_off, gen_off);
    }

    // Try to decompress first text record
    println!("\n=== First Text Record ===");
    let ref_rec1_off = u32::from_be_bytes([
        ref_data[78 + 8],
        ref_data[78 + 9],
        ref_data[78 + 8 + 2],
        ref_data[78 + 8 + 3],
    ]) as usize;

    let gen_rec1_off = u32::from_be_bytes([
        gen_data[78 + 8],
        gen_data[78 + 9],
        gen_data[78 + 8 + 2],
        gen_data[78 + 8 + 3],
    ]) as usize;

    let ref_rec2_off = u32::from_be_bytes([
        ref_data[78 + 16],
        ref_data[78 + 17],
        ref_data[78 + 16 + 2],
        ref_data[78 + 16 + 3],
    ]) as usize;

    let gen_rec2_off = u32::from_be_bytes([
        gen_data[78 + 16],
        gen_data[78 + 17],
        gen_data[78 + 16 + 2],
        gen_data[78 + 16 + 3],
    ]) as usize;

    let ref_rec1_data = &ref_data[ref_rec1_off..ref_rec2_off];
    let gen_rec1_data = &gen_data[gen_rec1_off..gen_rec2_off];

    println!("Record 1 size: {} vs {} bytes", ref_rec1_data.len(), gen_rec1_data.len());
    println!("Record 1 first 20 bytes: {:?} vs {:?}", &ref_rec1_data[..20.min(ref_rec1_data.len())], &gen_rec1_data[..20.min(gen_rec1_data.len())]);

    // Check EXTH header
    println!("\n=== EXTH Header Check ===");
    let ref_mobi_off = ref_off as usize;
    let gen_mobi_off = gen_off as usize;

    // EXTH flags at offset 0x80 from MOBI header
    let ref_exth_flags = u32::from_be_bytes([
        ref_data[ref_mobi_off + 0x80],
        ref_data[ref_mobi_off + 0x81],
        ref_data[ref_mobi_off + 0x82],
        ref_data[ref_mobi_off + 0x83],
    ]);

    let gen_exth_flags = u32::from_be_bytes([
        gen_data[gen_mobi_off + 0x80],
        gen_data[gen_mobi_off + 0x81],
        gen_data[gen_mobi_off + 0x82],
        gen_data[gen_mobi_off + 0x83],
    ]);

    println!("EXTH flags: 0x{:08X} vs 0x{:08X}", ref_exth_flags, gen_exth_flags);

    // Check extra_flags at 0xF2
    let ref_extra = u16::from_be_bytes([
        ref_data[ref_mobi_off + 0xF2],
        ref_data[ref_mobi_off + 0xF3],
    ]);

    let gen_extra = u16::from_be_bytes([
        gen_data[gen_mobi_off + 0xF2],
        gen_data[gen_mobi_off + 0xF3],
    ]);

    println!("Extra flags: 0x{:04X} vs 0x{:04X}", ref_extra, gen_extra);

    Ok(())
}
