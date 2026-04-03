//! Check MOBI extra_flags field

use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let mut ref_data = Vec::new();
    let mut gen_data = Vec::new();

    File::open("tests/fixtures/test_book.mobi")?.read_to_end(&mut ref_data)?;
    File::open("/tmp/test.mobi")?.read_to_end(&mut gen_data)?;

    let ref_mobi_off =
        u32::from_be_bytes([ref_data[78], ref_data[79], ref_data[80], ref_data[81]]) as usize;
    let gen_mobi_off =
        u32::from_be_bytes([gen_data[78], gen_data[79], gen_data[80], gen_data[81]]) as usize;

    println!("=== MOBI Extra Flags Comparison ===\n");

    // Extra flags are at offset 0xF2 from MOBI header start
    let ref_flags =
        u16::from_be_bytes([ref_data[ref_mobi_off + 0xF2], ref_data[ref_mobi_off + 0xF3]]);

    let gen_flags =
        u16::from_be_bytes([gen_data[gen_mobi_off + 0xF2], gen_data[gen_mobi_off + 0xF3]]);

    println!("Reference extra_flags: 0x{:04X}", ref_flags);
    println!("Generated extra_flags: 0x{:04X}", gen_flags);

    if ref_flags != gen_flags {
        println!("\n⚠️  FLAGS DIFFER!");
        println!("   Reference has trailing data: {}", ref_flags != 0);
        println!("   Generated has trailing data: {}", gen_flags != 0);
        println!("\nThis means text records may have extra data appended");
        println!("The decompressor needs to strip this before decompression");
    } else {
        println!("\n✓ Flags match - no trailing data");
    }

    Ok(())
}
