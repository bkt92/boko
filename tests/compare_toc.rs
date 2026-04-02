//! Compare 1.mobi vs 1_calibre.mobi focusing on TOC/NCX structure

use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let mut cal = Vec::new();
    let mut boko = Vec::new();
    File::open("tests/fixtures/1_calibre.mobi")?.read_to_end(&mut cal)?;
    File::open("tests/fixtures/1.mobi")?.read_to_end(&mut boko)?;

    println!("Calibre MOBI: {} bytes", cal.len());
    println!("Boko MOBI:    {} bytes", boko.len());
    println!();

    // Parse PDB header
    let cal_name = String::from_utf8_lossy(&cal[0..32]);
    let boko_name = String::from_utf8_lossy(&boko[0..32]);
    println!("PDB name cal:  {:?}", cal_name.trim_end_matches('\0'));
    println!("PDB name boko: {:?}", boko_name.trim_end_matches('\0'));

    let cal_nrec = u16::from_be_bytes([cal[76], cal[77]]) as usize;
    let boko_nrec = u16::from_be_bytes([boko[76], boko[77]]) as usize;
    println!("\nRecords cal:  {}", cal_nrec);
    println!("Records boko: {}", boko_nrec);

    // Extract record offsets
    let cal_offsets: Vec<u32> = (0..cal_nrec)
        .map(|i| u32::from_be_bytes([cal[78 + i * 8], cal[78 + i * 8 + 1], cal[78 + i * 8 + 2], cal[78 + i * 8 + 3]]))
        .collect();
    let boko_offsets: Vec<u32> = (0..boko_nrec)
        .map(|i| u32::from_be_bytes([boko[78 + i * 8], boko[78 + i * 8 + 1], boko[78 + i * 8 + 2], boko[78 + i * 8 + 3]]))
        .collect();

    // Get record data helper
    let rec_data = |data: &[u8], offsets: &[u32], idx: usize| -> Vec<u8> {
        let start = offsets[idx] as usize;
        let end = if idx + 1 < offsets.len() {
            offsets[idx + 1] as usize
        } else {
            data.len()
        };
        data[start..end].to_vec()
    };

    // Parse Record 0 MOBI header
    let cal_r0 = rec_data(&cal, &cal_offsets, 0);
    let boko_r0 = rec_data(&boko, &boko_offsets, 0);

    println!("\n=== Record 0 (MOBI Header) ===");
    println!("Cal size: {} bytes", cal_r0.len());
    println!("Boko size: {} bytes", boko_r0.len());

    // PalmDOC header
    let cal_text_recs = u16::from_be_bytes([cal_r0[8], cal_r0[9]]);
    let boko_text_recs = u16::from_be_bytes([boko_r0[8], boko_r0[9]]);
    println!("Text records cal:  {}", cal_text_recs);
    println!("Text records boko: {}", boko_text_recs);

    let cal_text_len = u32::from_be_bytes([cal_r0[4], cal_r0[5], cal_r0[6], cal_r0[7]]);
    let boko_text_len = u32::from_be_bytes([boko_r0[4], boko_r0[5], boko_r0[6], boko_r0[7]]);
    println!("Text length cal:  {}", cal_text_len);
    println!("Text length boko: {}", boko_text_len);

    // MOBI header starts at offset 16
    let cal_hdr_len = u32::from_be_bytes([cal_r0[20], cal_r0[21], cal_r0[22], cal_r0[23]]);
    let boko_hdr_len = u32::from_be_bytes([boko_r0[20], boko_r0[21], boko_r0[22], boko_r0[23]]);
    println!("\nMOBI header length cal:  {} (0x{:X})", cal_hdr_len, cal_hdr_len);
    println!("MOBI header length boko: {} (0x{:X})", boko_hdr_len, boko_hdr_len);

    let cal_codepage = u32::from_be_bytes([cal_r0[28], cal_r0[29], cal_r0[30], cal_r0[31]]);
    let boko_codepage = u32::from_be_bytes([boko_r0[28], boko_r0[29], boko_r0[30], boko_r0[31]]);
    println!("Codepage cal:  {} (65001=UTF8, 1252=CP1252)", cal_codepage);
    println!("Codepage boko: {}", boko_codepage);

    // First non-text record (offset 0x40 from MOBI = byte 80)
    let cal_first_non = u32::from_be_bytes([cal_r0[80], cal_r0[81], cal_r0[82], cal_r0[83]]);
    let boko_first_non = u32::from_be_bytes([boko_r0[80], boko_r0[81], boko_r0[82], boko_r0[83]]);
    println!("\nFirst non-text record cal:  {}", cal_first_non);
    println!("First non-text record boko: {}", boko_first_non);

    // EXTH flags (offset 0x60 from MOBI = byte 112 for MOBI6)
    // Actually at MOBI offset 0x70 = byte 128
    let cal_exth = u32::from_be_bytes([cal_r0[128], cal_r0[129], cal_r0[130], cal_r0[131]]);
    let boko_exth = u32::from_be_bytes([boko_r0[128], boko_r0[129], boko_r0[130], boko_r0[131]]);
    println!("EXTH flags cal:  0x{:08X}", cal_exth);
    println!("EXTH flags boko: 0x{:08X}", boko_exth);

    // NCX index record (MOBI offset 0xE4 = byte 244)
    if cal_r0.len() > 248 {
        let cal_ncx = u32::from_be_bytes([cal_r0[244], cal_r0[245], cal_r0[246], cal_r0[247]]);
        println!("\nNCX index record cal:  {} (0xFFFFFFFF=none)", cal_ncx);
    }
    if boko_r0.len() > 248 {
        let boko_ncx = u32::from_be_bytes([boko_r0[244], boko_r0[245], boko_r0[246], boko_r0[247]]);
        println!("NCX index record boko: {} (0xFFFFFFFF=none)", boko_ncx);
    }

    // First content / last content record (MOBI offset 0xB0-B3 and 0xB4-B7 = byte 192-195 and 196-199)
    let cal_first_content = u16::from_be_bytes([cal_r0[192], cal_r0[193]]);
    let boko_first_content = u16::from_be_bytes([boko_r0[192], boko_r0[193]]);
    let cal_last_content = u16::from_be_bytes([cal_r0[194], cal_r0[195]]);
    let boko_last_content = u16::from_be_bytes([boko_r0[194], boko_r0[195]]);
    println!("\nFirst content record cal:  {}", cal_first_content);
    println!("First content record boko: {}", boko_first_content);
    println!("Last content record cal:  {}", cal_last_content);
    println!("Last content record boko: {}", boko_last_content);

    // FCIS/FLIS record (MOBI offset 0xB8 and 0xC0 = byte 200 and 208)
    let cal_fcis = u32::from_be_bytes([cal_r0[200], cal_r0[201], cal_r0[202], cal_r0[203]]);
    let boko_fcis = u32::from_be_bytes([boko_r0[200], boko_r0[201], boko_r0[202], boko_r0[203]]);
    let cal_flis = u32::from_be_bytes([cal_r0[208], cal_r0[209], cal_r0[210], cal_r0[211]]);
    let boko_flis = u32::from_be_bytes([boko_r0[208], boko_r0[209], boko_r0[210], boko_r0[211]]);
    println!("\nFCIS record cal:  {}", cal_fcis);
    println!("FCIS record boko: {}", boko_fcis);
    println!("FLIS record cal:  {}", cal_flis);
    println!("FLIS record boko: {}", boko_flis);

    // Extra data flags (MOBI offset 0xE0 = byte 240)
    let cal_extra = u32::from_be_bytes([cal_r0[240], cal_r0[241], cal_r0[242], cal_r0[243]]);
    let boko_extra = u32::from_be_bytes([boko_r0[240], boko_r0[241], boko_r0[242], boko_r0[243]]);
    println!("\nExtra data flags cal:  0x{:08X}", cal_extra);
    println!("Extra data flags boko: 0x{:08X}", boko_extra);

    // Record layout
    println!("\n=== Record Layout ===");
    for (label, data, offsets, nrec) in
        [("Cal", &cal, &cal_offsets, cal_nrec), ("Boko", &boko, &boko_offsets, boko_nrec)]
    {
        println!("\n{} ({} records):", label, nrec);
        for i in 0..nrec {
            let start = offsets[i] as usize;
            let end = if i + 1 < offsets.len() { offsets[i + 1] as usize } else { data.len() };
            let size = end - start;
            let magic = if size >= 4 {
                String::from_utf8_lossy(&data[start..start + 4.min(size)])
            } else {
                std::borrow::Cow::Borrowed("")
            };
            println!("  Rec {:3}: offset={:8} size={:6} magic={:?}", i, start, size, magic);
        }
    }

    // EXTH header comparison
    println!("\n=== EXTH Header ===");
    for (label, r0, hdr_len) in [("Cal", &cal_r0, cal_hdr_len), ("Boko", &boko_r0, boko_hdr_len)] {
        let exth_start = 16 + hdr_len as usize;
        if exth_start + 12 <= r0.len() && &r0[exth_start..exth_start + 4] == b"EXTH" {
            let exth_len = u32::from_be_bytes([r0[exth_start + 4], r0[exth_start + 5], r0[exth_start + 6], r0[exth_start + 7]]);
            let exth_count = u32::from_be_bytes([r0[exth_start + 8], r0[exth_start + 9], r0[exth_start + 10], r0[exth_start + 11]]);
            println!("\n{} EXTH: len={}, count={}", label, exth_len, exth_count);
            let mut pos = exth_start + 12;
            for _ in 0..exth_count {
                if pos + 8 > r0.len() { break; }
                let rtype = u32::from_be_bytes([r0[pos], r0[pos + 1], r0[pos + 2], r0[pos + 3]]);
                let rlen = u32::from_be_bytes([r0[pos + 4], r0[pos + 5], r0[pos + 6], r0[pos + 7]]) as usize;
                if rlen < 8 || pos + rlen > r0.len() { break; }
                let data = &r0[pos + 8..pos + rlen];
                let data_str = String::from_utf8_lossy(data);
                let display = if data_str.len() > 80 {
                    format!("{}...", &data_str[..80])
                } else {
                    data_str.to_string()
                };
                println!("  Record {:4}: {} bytes -> {}", rtype, rlen - 8, display);
                pos += rlen;
            }
        }
    }

    // INDX record details (for NCX/TOC)
    println!("\n=== INDX Records (TOC/NCX) ===");
    for (label, data, offsets, nrec) in
        [("Cal", &cal, &cal_offsets, cal_nrec), ("Boko", &boko, &boko_offsets, boko_nrec)]
    {
        println!("\n{}:", label);
        for i in 0..nrec {
            let start = offsets[i] as usize;
            let end = if i + 1 < offsets.len() { offsets[i + 1] as usize } else { data.len() };
            if end - start >= 4 && &data[start..start + 4] == b"INDX" {
                let rec = &data[start..end];
                let indx_type = if rec.len() > 12 {
                    u32::from_be_bytes([rec[12], rec[13], rec[14], rec[15]])
                } else {
                    0
                };
                println!("  Rec {} at offset {}: INDX record, {} bytes, type={}", i, start, end - start, indx_type);
                // Print first 32 bytes hex
                let hex_bytes: Vec<String> = rec[..32.min(rec.len())].iter().map(|b| format!("{:02X}", b)).collect();
                println!("    Header: {}", hex_bytes.join(" "));
            }
        }
    }

    Ok(())
}
