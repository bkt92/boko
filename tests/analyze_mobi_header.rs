//! Analyze MOBI header size differences

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut ref_data = Vec::new();
    let mut gen_data = Vec::new();

    File::open("tests/fixtures/test_book.mobi")?.read_to_end(&mut ref_data)?;
    File::open("tests/fixtures/test_final.mobi")?.read_to_end(&mut gen_data)?;

    let ref_mobi_off =
        u32::from_be_bytes([ref_data[78], ref_data[79], ref_data[80], ref_data[81]]) as usize;
    let gen_mobi_off =
        u32::from_be_bytes([gen_data[78], gen_data[79], gen_data[80], gen_data[81]]) as usize;

    // Calculate MOBI header size
    let ref_mobi_end = ref_mobi_off
        + u32::from_be_bytes([
            ref_data[ref_mobi_off + 0x14],
            ref_data[ref_mobi_off + 0x15],
            ref_data[ref_mobi_off + 0x16],
            ref_data[ref_mobi_off + 0x17],
        ]) as usize;

    let gen_mobi_end = gen_mobi_off
        + u32::from_be_bytes([
            gen_data[gen_mobi_off + 0x14],
            gen_data[gen_mobi_off + 0x15],
            gen_data[gen_mobi_off + 0x16],
            gen_data[gen_mobi_off + 0x17],
        ]) as usize;

    println!("Reference MOBI header:");
    println!("  Start: {}", ref_mobi_off);
    println!("  End: {}", ref_mobi_end);
    println!("  Size: {} bytes", ref_mobi_end - ref_mobi_off);

    println!("\nGenerated MOBI header:");
    println!("  Start: {}", gen_mobi_off);
    println!("  End: {}", gen_mobi_end);
    println!("  Size: {} bytes", gen_mobi_end - gen_mobi_off);

    println!(
        "\nSize difference: {}",
        (gen_mobi_end - gen_mobi_off) as i64 - (ref_mobi_end - ref_mobi_off) as i64
    );

    // Check EXTH header
    let ref_has_exth = ref_data[ref_mobi_off + 0x80] & 0x40 != 0;
    let gen_has_exth = gen_data[gen_mobi_off + 0x80] & 0x40 != 0;

    println!("\nEXTH header:");
    println!("  Reference has EXTH: {}", ref_has_exth);
    println!("  Generated has EXTH: {}", gen_has_exth);

    if ref_has_exth {
        // Find EXTH header (starts after MOBI header)
        let ref_exth_start = ref_mobi_end;
        let ref_exth_magic = &ref_data[ref_exth_start..ref_exth_start + 4];
        println!(
            "  Reference EXTH magic: {:?}",
            String::from_utf8_lossy(ref_exth_magic)
        );

        if ref_exth_magic == b"EXTH" {
            let ref_exth_len = u32::from_be_bytes([
                ref_data[ref_exth_start + 4],
                ref_data[ref_exth_start + 5],
                ref_data[ref_exth_start + 6],
                ref_data[ref_exth_start + 7],
            ]);
            let ref_exth_rec_count = u32::from_be_bytes([
                ref_data[ref_exth_start + 8],
                ref_data[ref_exth_start + 9],
                ref_data[ref_exth_start + 10],
                ref_data[ref_exth_start + 11],
            ]);
            println!("  Reference EXTH length: {} bytes", ref_exth_len);
            println!("  Reference EXTH record count: {}", ref_exth_rec_count);
        }
    }

    if gen_has_exth {
        let gen_exth_start = gen_mobi_end;
        let gen_exth_magic = &gen_data[gen_exth_start..gen_exth_start + 4];
        println!(
            "  Generated EXTH magic: {:?}",
            String::from_utf8_lossy(gen_exth_magic)
        );

        if gen_exth_magic == b"EXTH" {
            let gen_exth_len = u32::from_be_bytes([
                gen_data[gen_exth_start + 4],
                gen_data[gen_exth_start + 5],
                gen_data[gen_exth_start + 6],
                gen_data[gen_exth_start + 7],
            ]);
            let gen_exth_rec_count = u32::from_be_bytes([
                gen_data[gen_exth_start + 8],
                gen_data[gen_exth_start + 9],
                gen_data[gen_exth_start + 10],
                gen_data[gen_exth_start + 11],
            ]);
            println!("  Generated EXTH length: {} bytes", gen_exth_len);
            println!("  Generated EXTH record count: {}", gen_exth_rec_count);
        }
    }

    // Compare title
    println!("\nTitle comparison:");
    let ref_title_off = ref_mobi_off
        + u32::from_be_bytes([
            ref_data[ref_mobi_off + 0x54],
            ref_data[ref_mobi_off + 0x55],
            ref_data[ref_mobi_off + 0x56],
            ref_data[ref_mobi_off + 0x57],
        ]) as usize;

    let gen_title_off = gen_mobi_off
        + u32::from_be_bytes([
            gen_data[gen_mobi_off + 0x54],
            gen_data[gen_mobi_off + 0x55],
            gen_data[gen_mobi_off + 0x56],
            gen_data[gen_mobi_off + 0x57],
        ]) as usize;

    let ref_title_len = u32::from_be_bytes([
        ref_data[ref_mobi_off + 0x58],
        ref_data[ref_mobi_off + 0x5C],
        ref_data[ref_mobi_off + 0x5A],
        ref_data[ref_mobi_off + 0x5B],
    ]);

    let gen_title_len = u32::from_be_bytes([
        gen_data[gen_mobi_off + 0x58],
        gen_data[gen_mobi_off + 0x5C],
        gen_data[gen_mobi_off + 0x5A],
        gen_data[gen_mobi_off + 0x5B],
    ]);

    println!(
        "  Reference title offset: {}, length: {}",
        ref_title_off - ref_mobi_off,
        ref_title_len
    );
    println!(
        "  Generated title offset: {}, length: {}",
        gen_title_off - gen_mobi_off,
        gen_title_len
    );

    if ref_title_len > 0 && ref_title_off + ref_title_len as usize <= ref_data.len() {
        let ref_title = String::from_utf8_lossy(
            &ref_data[ref_title_off..ref_title_off + ref_title_len as usize],
        );
        println!("  Reference title: {}", ref_title);
    }

    if gen_title_len > 0 && gen_title_off + gen_title_len as usize <= gen_data.len() {
        let gen_title = String::from_utf8_lossy(
            &gen_data[gen_title_off..gen_title_off + gen_title_len as usize],
        );
        println!("  Generated title: {}", gen_title);
    }

    Ok(())
}
