//! Parse EXTH header from MOBI file

use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Usage: parse_exth <mobi_file>");
        std::process::exit(1);
    }

    let path = &args[0];
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;

    // Get MOBI header offset
    let mobi_offset = u32::from_be_bytes([data[78], data[79], data[80], data[81]]) as usize;

    // Find EXTH signature (should be after MOBI header at 232 bytes)
    let exth_search_start = mobi_offset + 232;
    let mut exth_offset = None;

    // Search for "EXTH" signature
    for i in 0..20 {
        let pos = exth_search_start + i;
        if pos + 4 <= data.len() {
            let sig = String::from_utf8_lossy(&data[pos..pos + 4]);
            if sig == "EXTH" {
                exth_offset = Some(pos);
                break;
            }
        }
    }

    let exth_offset = match exth_offset {
        Some(offset) => offset,
        None => {
            eprintln!("EXTH signature not found!");
            return Ok(());
        }
    };

    println!("=== EXTH Header Analysis: {} ===\n", path);
    println!("MOBI header offset: 0x{:04X}", mobi_offset);
    println!("EXTH signature offset: 0x{:04X} ({} bytes from MOBI header)",
        exth_offset, exth_offset - mobi_offset);

    // Parse EXTH header
    if exth_offset + 12 > data.len() {
        eprintln!("Not enough data for EXTH header");
        return Ok(());
    }

    let exth_len = u32::from_be_bytes([
        data[exth_offset + 4],
        data[exth_offset + 5],
        data[exth_offset + 6],
        data[exth_offset + 7],
    ]) as usize;

    let exth_count = u32::from_be_bytes([
        data[exth_offset + 8],
        data[exth_offset + 9],
        data[exth_offset + 10],
        data[exth_offset + 11],
    ]);

    println!("EXTH header length: {} bytes (0x{:04X})", exth_len, exth_len);
    println!("EXTH record count: {} records\n", exth_count);

    // Parse records
    let mut pos = exth_offset + 12;
    for i in 0..exth_count {
        if pos + 8 > data.len() {
            println!("ERROR: Not enough data for record header at offset 0x{:04X}", pos);
            break;
        }

        let rec_type = u32::from_be_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
        ]);

        let rec_len = u32::from_be_bytes([
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]) as usize;

        if rec_len < 8 || pos + rec_len > data.len() {
            println!("ERROR: Invalid record length {} at offset 0x{:04X}", rec_len, pos);
            break;
        }

        let rec_data = &data[pos + 8..pos + rec_len];
        let data_str = String::from_utf8_lossy(rec_data);

        // Known record types
        let rec_name = match rec_type {
            100 => "Author",
            101 => "Publisher",
            102 => "Imprint",
            103 => "Description",
            104 => "ISBN",
            105 => "Subject",
            106 => "Published date",
            107 => "Review",
            108 => "Contributor",
            109 => "Rights",
            110 => "Subject code",
            111 => "Book type",
            112 => "Source",
            113 => "ASIN",
            114 => "ADSR",
            115 => "Language",
            116 => "Parent ISBN",
            117 => "Parent ASIN",
            118 => "Abbr",
            119 => "Title",
            120 => "Subtitle",
            121 => "Author sort",
            122 => "Author data",
            125 => "Primary writing mode",
            126 => "Pagination",
            127 => "Page progress",
            128 => "Cover URI",
            129 => "Thumb offset",
            130 => "Cover URI default",
            131 => "Thumb offset default",
            200 => "Dictionary short name",
            201 => "Cover image",
            202 => "Thumb image",
            203 => "Sample URL",
            300 => "Orientation",
            301 => "Resolution",
            302 => "Color",
            303 => "Shaping",
            304 => "Font",
            401 => "Reference type",
            402 => "Column info",
            403 => "Subset info",
            404 => "Region info",
            405 => "Map info",
            406 => "CDE type",
            501 => "Enhancement",
            502 => "Board info",
            503 => "Title (in EXTH)",
            504 => "Creator (in EXTH)",
            505 => "Publisher (in EXTH)",
            506 => "Date (type)",
            507 => "Resource count",
            508 => "Resource URI",
            509 => "Container info",
            510 => "K8 metadata boundary",
            511 => "K8 metadata series",
            512 => "Fixed layout",
            513 => "Book type (K8)",
            514 => "Orientation lock",
            515 => "Page progression",
            516 => "Overflow",
            517 => "Region MAG",
            518 => "K8 metadata series",
            519 => "K8 metadata secondary",
            520 => "K8 metadata tertiary",
            521 => "K8 metadata quaternary",
            522 => "K8 metadata quinary",
            523 => "K8 reading order",
            524 => "Language",
            525 => "Secondary title",
            526 => "Resource set",
            527 => "K8 metadata companion",
            528 => "K8 metadata update",
            529 => "K8 metadata delta",
            530 => "K8 metadata scheme",
            531 => "K8 metadata locale",
            532 => "K8 rendering direction",
            533 => "K8 allowable zoom",
            534 => "K8 zero width",
            535 => "K8 zero height",
            536 => "K8 resource container",
            537 => "K8 orientation lock",
            538 => "K8 original resolution",
            539 => "K8 metadata primary",
            540 => "K8 metadata secondary",
            541 => "K8 fixed layout",
            542 => "K8 overflow",
            543 => "K8 region MAG",
            544 => "K8 viewport aspect",
            545 => "K8 imperial aspect",
            546 => "Page state",
            547 => "River whitening",
            548 => "Count of resources",
            549 => "In-memory version",
            550 => "Page map",
            551 => "Page scrape",
            552 => "Scrape key",
            _ => "Unknown",
        };

        let preview = if data_str.len() > 100 {
            format!("{}...", &data_str[..100])
        } else {
            data_str.to_string()
        };

        println!("Record {}: type={} ({}), len={}, data={:?}",
            i, rec_type, rec_name, rec_len, preview);

        pos += rec_len;
    }

    // Show what's after EXTH
    if pos < data.len() && pos < mobi_offset + 12000 {
        println!("\n=== Data after EXTH ===");
        println!("Offset 0x{:04X} (0x{:04X} from MOBI header):", pos, pos - mobi_offset);

        // Look for title (null-terminated string)
        let mut title_end = pos;
        for i in pos..(pos + 500).min(data.len()) {
            if data[i] == 0 {
                title_end = i;
                break;
            }
        }

        if title_end > pos {
            let title = String::from_utf8_lossy(&data[pos..title_end]);
            println!("Title ({} bytes): {:?}", title_end - pos, title);
        }

        println!("Next 100 bytes:");
        for i in 0..100.min(data.len() - pos) {
            if i % 16 == 0 {
                println!("  {:04X}: ", pos + i);
            }
            print!("{:02X} ", data[pos + i]);
            if (i + 1) % 16 == 0 {
                println!();
            }
        }
        if data.len() - pos > 100 {
            println!("  ... ({} more bytes)", data.len() - pos - 100);
        }
    }

    Ok(())
}
