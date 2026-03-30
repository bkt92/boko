//! Debug PalmDoc compression to find the issue

use std::fs::File;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut ref_data = Vec::new();
    let mut gen_data = Vec::new();

    File::open("tests/fixtures/test_book.mobi")?.read_to_end(&mut ref_data)?;
    File::open("tests/fixtures/test_final.mobi")?.read_to_end(&mut gen_data)?;

    let ref_mobi_off = u32::from_be_bytes([ref_data[78], ref_data[79], ref_data[80], ref_data[81]]) as usize;
    let gen_mobi_off = u32::from_be_bytes([gen_data[78], gen_data[79], gen_data[80], gen_data[81]]) as usize;

    let ref_text_len = u32::from_be_bytes([
        ref_data[ref_mobi_off + 4],
        ref_data[ref_mobi_off + 5],
        ref_data[ref_mobi_off + 6],
        ref_data[ref_mobi_off + 7],
    ]);

    let gen_text_len = u32::from_be_bytes([
        gen_data[gen_mobi_off + 4],
        gen_data[gen_mobi_off + 5],
        gen_data[gen_mobi_off + 6],
        gen_data[gen_mobi_off + 7],
    ]);

    println!("Reference text length: {}", ref_text_len);
    println!("Generated text length: {}", gen_text_len);
    println!("Difference: {}", gen_text_len as i64 - ref_text_len as i64);

    // Get text record count
    let ref_count = u16::from_be_bytes([ref_data[ref_mobi_off + 8], ref_data[ref_mobi_off + 9]]);
    let gen_count = u16::from_be_bytes([gen_data[gen_mobi_off + 8], gen_data[gen_mobi_off + 9]]);

    println!("\nReference text records: {}", ref_count);
    println!("Generated text records: {}", gen_count);

    // Calculate total compressed size
    let mut ref_compressed = 0;
    let mut gen_compressed = 0;

    for i in 1..=ref_count as usize {
        let start = u32::from_be_bytes([
            ref_data[78 + i * 8],
            ref_data[78 + i * 8 + 1],
            ref_data[78 + i * 8 + 2],
            ref_data[78 + i * 8 + 3],
        ]) as usize;

        let end = if i + 1 <= ref_count as usize {
            u32::from_be_bytes([
                ref_data[78 + (i + 1) * 8],
                ref_data[78 + (i + 1) * 8 + 1],
                ref_data[78 + (i + 1) * 8 + 2],
                ref_data[78 + (i + 1) * 8 + 3],
            ]) as usize
        } else {
            // Next record is INDX (record 117)
            u32::from_be_bytes([
                ref_data[78 + 117 * 8],
                ref_data[78 + 117 * 8 + 1],
                ref_data[78 + 117 * 8 + 2],
                ref_data[78 + 117 * 8 + 3],
            ]) as usize
        };

        ref_compressed += end - start;
    }

    for i in 1..=gen_count as usize {
        let start = u32::from_be_bytes([
            gen_data[78 + i * 8],
            gen_data[78 + i * 8 + 1],
            gen_data[78 + i * 8 + 2],
            gen_data[78 + i * 8 + 3],
        ]) as usize;

        let end = if i + 1 <= gen_count as usize {
            u32::from_be_bytes([
                gen_data[78 + (i + 1) * 8],
                gen_data[78 + (i + 1) * 8 + 1],
                gen_data[78 + (i + 1) * 8 + 2],
                gen_data[78 + (i + 1) * 8 + 3],
            ]) as usize
        } else {
            // Next record is INDX (record 117)
            u32::from_be_bytes([
                gen_data[78 + 117 * 8],
                gen_data[78 + 117 * 8 + 1],
                gen_data[78 + 117 * 8 + 2],
                gen_data[78 + 117 * 8 + 3],
            ]) as usize
        };

        gen_compressed += end - start;
    }

    println!("\nReference compressed size: {} bytes", ref_compressed);
    println!("Generated compressed size: {} bytes", gen_compressed);
    println!("Compression ratio: {:.2}%", (ref_compressed as f64 / ref_text_len as f64) * 100.0);

    // Check if there are extra records after text records
    println!("\nRecord 117 (first INDX):");
    let ref_indx_off = u32::from_be_bytes([
        ref_data[78 + 117 * 8],
        ref_data[78 + 117 * 8 + 1],
        ref_data[78 + 117 * 8 + 2],
        ref_data[78 + 117 * 8 + 3],
    ]) as usize;

    let gen_indx_off = u32::from_be_bytes([
        gen_data[78 + 117 * 8],
        gen_data[78 + 117 * 8 + 1],
        gen_data[78 + 117 * 8 + 2],
        gen_data[78 + 117 * 8 + 3],
    ]) as usize;

    println!("  Reference offset: {}", ref_indx_off);
    println!("  Generated offset: {}", gen_indx_off);

    // Calculate where text records end
    let ref_text_end = ref_indx_off;
    let gen_text_end = gen_indx_off;

    println!("\nText records end at:");
    println!("  Reference: {}", ref_text_end);
    println!("  Generated: {}", gen_text_end);

    Ok(())
}
