//! Compare generated MOBI with reference MOBI to find differences

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut ref_data = Vec::new();
    let mut gen_data = Vec::new();

    File::open("tests/fixtures/test_book.mobi")?.read_to_end(&mut ref_data)?;
    File::open("tests/fixtures/test_final.mobi")?.read_to_end(&mut gen_data)?;

    println!("Reference MOBI: {} bytes", ref_data.len());
    println!("Generated MOBI: {} bytes", gen_data.len());

    // Compare PDB headers
    println!("\n=== PDB Header Comparison ===");
    println!(
        "Reference magic: {:?}",
        String::from_utf8_lossy(&ref_data[60..68])
    );
    println!(
        "Generated magic: {:?}",
        String::from_utf8_lossy(&gen_data[60..68])
    );

    let ref_records = u16::from_be_bytes([ref_data[76], ref_data[77]]);
    let gen_records = u16::from_be_bytes([gen_data[76], gen_data[77]]);
    println!("Reference records: {}", ref_records);
    println!("Generated records: {}", gen_records);

    // Compare record offsets
    let ref_offset_start = 78;
    let gen_offset_start = 78;

    println!("\n=== Record Offset Comparison ===");
    for i in 0..5 {
        let ref_pos = ref_offset_start + i * 8;
        let gen_pos = gen_offset_start + i * 8;

        let ref_off = u32::from_be_bytes([
            ref_data[ref_pos],
            ref_data[ref_pos + 1],
            ref_data[ref_pos + 2],
            ref_data[ref_pos + 3],
        ]);

        let gen_off = u32::from_be_bytes([
            gen_data[gen_pos],
            gen_data[gen_pos + 1],
            gen_data[gen_pos + 2],
            gen_data[gen_pos + 3],
        ]);

        println!(
            "Record {}: ref={}, gen={}, diff={}",
            i,
            ref_off,
            gen_off,
            gen_off as i64 - ref_off as i64
        );
    }

    // Compare MOBI headers (Record 0)
    let ref_mobi_start = ref_data[78..86].iter().position(|&x| x != 0).unwrap_or(0);
    let gen_mobi_start = gen_data[78..86].iter().position(|&x| x != 0).unwrap_or(0);

    let ref_mobi_off =
        u32::from_be_bytes([ref_data[78], ref_data[79], ref_data[80], ref_data[81]]) as usize;
    let gen_mobi_off =
        u32::from_be_bytes([gen_data[78], gen_data[79], gen_data[80], gen_data[81]]) as usize;

    println!("\n=== MOBI Header (Record 0) Comparison ===");
    println!("Reference MOBI starts at: {}", ref_mobi_off);
    println!("Generated MOBI starts at: {}", gen_mobi_off);

    // Compare critical MOBI header fields
    println!("\nCompression type:");
    println!(
        "  Reference: {}",
        u16::from_be_bytes([ref_data[ref_mobi_off], ref_data[ref_mobi_off + 1]])
    );
    println!(
        "  Generated: {}",
        u16::from_be_bytes([gen_data[gen_mobi_off], gen_data[gen_mobi_off + 1]])
    );

    println!("\nText length:");
    println!(
        "  Reference: {}",
        u32::from_be_bytes([
            ref_data[ref_mobi_off + 4],
            ref_data[ref_mobi_off + 5],
            ref_data[ref_mobi_off + 6],
            ref_data[ref_mobi_off + 7]
        ])
    );
    println!(
        "  Generated: {}",
        u32::from_be_bytes([
            gen_data[gen_mobi_off + 4],
            gen_data[gen_mobi_off + 5],
            gen_data[gen_mobi_off + 6],
            gen_data[gen_mobi_off + 7]
        ])
    );

    println!("\nText record count:");
    println!(
        "  Reference: {}",
        u16::from_be_bytes([ref_data[ref_mobi_off + 8], ref_data[ref_mobi_off + 9]])
    );
    println!(
        "  Generated: {}",
        u16::from_be_bytes([gen_data[gen_mobi_off + 8], gen_data[gen_mobi_off + 9]])
    );

    println!("\nText record size:");
    println!(
        "  Reference: {}",
        u16::from_be_bytes([ref_data[ref_mobi_off + 10], ref_data[ref_mobi_off + 11]])
    );
    println!(
        "  Generated: {}",
        u16::from_be_bytes([gen_data[gen_mobi_off + 10], gen_data[gen_mobi_off + 11]])
    );

    // Check MOBI version
    println!("\nMOBI version (offset 0x68):");
    println!(
        "  Reference: {}",
        u32::from_be_bytes([
            ref_data[ref_mobi_off + 0x68],
            ref_data[ref_mobi_off + 0x69],
            ref_data[ref_mobi_off + 0x6A],
            ref_data[ref_mobi_off + 0x6B]
        ])
    );
    println!(
        "  Generated: {}",
        u32::from_be_bytes([
            gen_data[gen_mobi_off + 0x68],
            gen_data[gen_mobi_off + 0x69],
            gen_data[gen_mobi_off + 0x6A],
            gen_data[gen_mobi_off + 0x6B]
        ])
    );

    // Check first image index
    println!("\nFirst image index (offset 0x6C):");
    println!(
        "  Reference: {}",
        u32::from_be_bytes([
            ref_data[ref_mobi_off + 0x6C],
            ref_data[ref_mobi_off + 0x6D],
            ref_data[ref_mobi_off + 0x6E],
            ref_data[ref_mobi_off + 0x6F]
        ])
    );
    println!(
        "  Generated: {}",
        u32::from_be_bytes([
            gen_data[gen_mobi_off + 0x6C],
            gen_data[gen_mobi_off + 0x6D],
            gen_data[gen_mobi_off + 0x6E],
            gen_data[gen_mobi_off + 0x6F]
        ])
    );

    // Check EXTH flags
    println!("\nEXTH flags (offset 0x80):");
    println!(
        "  Reference: 0x{:04X}",
        u32::from_be_bytes([
            ref_data[ref_mobi_off + 0x80],
            ref_data[ref_mobi_off + 0x81],
            ref_data[ref_mobi_off + 0x82],
            ref_data[ref_mobi_off + 0x83]
        ])
    );
    println!(
        "  Generated: 0x{:04X}",
        u32::from_be_bytes([
            gen_data[gen_mobi_off + 0x80],
            gen_data[gen_mobi_off + 0x81],
            gen_data[gen_mobi_off + 0x82],
            gen_data[gen_mobi_off + 0x83]
        ])
    );

    // Compare image data
    println!("\n=== Image Data Comparison ===");
    // Find first image in both files
    let mut ref_image_offsets = Vec::new();
    let mut gen_image_offsets = Vec::new();

    for i in 0..ref_records as usize {
        let start = u32::from_be_bytes([
            ref_data[78 + i * 8],
            ref_data[78 + i * 8 + 1],
            ref_data[78 + i * 8 + 2],
            ref_data[78 + i * 8 + 3],
        ]) as usize;

        if start + 2 < ref_data.len() && ref_data[start] == 0xFF && ref_data[start + 1] == 0xD8 {
            ref_image_offsets.push(i);
        }
    }

    for i in 0..gen_records as usize {
        let start = u32::from_be_bytes([
            gen_data[78 + i * 8],
            gen_data[78 + i * 8 + 1],
            gen_data[78 + i * 8 + 2],
            gen_data[78 + i * 8 + 3],
        ]) as usize;

        if start + 2 < gen_data.len() && gen_data[start] == 0xFF && gen_data[start + 1] == 0xD8 {
            gen_image_offsets.push(i);
        }
    }

    println!(
        "Reference images: {} (first at record {})",
        ref_image_offsets.len(),
        ref_image_offsets.first().unwrap_or(&0)
    );
    println!(
        "Generated images: {} (first at record {})",
        gen_image_offsets.len(),
        gen_image_offsets.first().unwrap_or(&0)
    );

    // Compare first 100 bytes of first image
    if !ref_image_offsets.is_empty() && !gen_image_offsets.is_empty() {
        let ref_img_start = u32::from_be_bytes([
            ref_data[78 + ref_image_offsets[0] * 8],
            ref_data[78 + ref_image_offsets[0] * 8 + 1],
            ref_data[78 + ref_image_offsets[0] * 8 + 2],
            ref_data[78 + ref_image_offsets[0] * 8 + 3],
        ]) as usize;

        let gen_img_start = u32::from_be_bytes([
            gen_data[78 + gen_image_offsets[0] * 8],
            gen_data[78 + gen_image_offsets[0] * 8 + 1],
            gen_data[78 + gen_image_offsets[0] * 8 + 2],
            gen_data[78 + gen_image_offsets[0] * 8 + 3],
        ]) as usize;

        println!("\nFirst image comparison:");
        println!("  Reference offset: {}", ref_img_start);
        println!("  Generated offset: {}", gen_img_start);

        let ref_img_data =
            &ref_data[ref_img_start..ref_img_start + 20.min(ref_data.len() - ref_img_start)];
        let gen_img_data =
            &gen_data[gen_img_start..gen_img_start + 20.min(gen_data.len() - gen_img_start)];

        println!("  Reference first 20 bytes: {:02X?}", ref_img_data);
        println!("  Generated first 20 bytes: {:02X?}", gen_img_data);

        if ref_img_data == gen_img_data {
            println!("  ✓ First image data matches");
        } else {
            println!("  ✗ First image data differs");
        }
    }

    Ok(())
}
