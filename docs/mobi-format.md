# MOBI/AZW3 Format Documentation

MOBI is a legacy ebook format originally developed for PalmOS devices, later adopted by Amazon for Kindle. AZW3 (also called KF8) is an enhanced version of MOBI that supports EPUB-like content.

## Overview

MOBI files are PalmDB databases containing:
- Header records with metadata
- Text content (compressed with HUFF/CDIC)
- Image resources
- Index structures for navigation
- EXTH header for extended metadata

## File Structure

```
book.mobi / book.azw3
├── PalmDB Header
│   ├── Database name
│   ├── Number of records
│   └── Record info list
├── MOBI Header
│   ├── Compression type
│   ├── Text length
│   └── Format version
├── EXTH Header (optional)
│   ├── Author
│   ├── Title
│   ├── Publisher
│   └── ... extended metadata
├── Text Records
│   └── Compressed text content
├── Image Records
│   └── Image data (JPEG, PNG, GIF)
└── Index Records
    ├── NCX (navigation)
    ├── INDX (index)
    └── FDST (flow data)
```

## MOBI vs AZW3

| Feature | MOBI (KF7) | AZW3 (KF8) |
|---------|-----------|------------|
| Content | Plain text | HTML + CSS |
| Images | Limited support | Full support |
| Layout | Basic formatting | Advanced CSS |
| Metadata | MOBI header | EXTH header |
| Compression | HUFF/CDIC | LZ77 or none |
| Navigation | NCX index | NCX + tags |

## PalmDB Structure

### PalmDB Header

```rust
pub struct PdbHeader {
    pub name: [u8; 32],
    pub attributes: u16,
    pub version: u16,
    pub create_time: u32,
    pub modify_time: u32,
    pub backup_time: u32,
    pub modification_number: u32,
    pub app_info_id: u32,
    pub sort_info_id: u32,
    pub type: [u8; 4],
    pub creator: [u8; 4],
    pub unique_id_seed: u32,
    pub next_record_list_id: u32,
    pub num_records: u16,
}
```

### Record Info

```rust
pub struct RecordInfo {
    pub offset: u32,
    pub attributes: u8,
    pub unique_id: u32,
}
```

## MOBI Header

```rust
pub struct MobiHeader {
    pub compression: Compression,
    pub text_length: u32,
    pub text_record_count: u16,
    pub text_record_size: u32,
    pub crypto_type: u16,
    pub format: MobiFormat,
    pub orthographic_index: u32,
    pub orthographic_name_index: u32,
    pub inflection_index: u32,
    pub inflection_name_index: u32,
    pub index_names: u32,
    pub index_keys: u32,
    pub extra_index_index: u32,
    pub first_non_book_index: u32,
    pub full_name_offset: u32,
    pub full_name_length: u32,
    pub locale: u32,
    pub input_language: u32,
    pub output_language: u32,
    pub min_version: u32,
    pub image_index: u32,
    pub huff_record_index: u32,
    pub huff_record_count: u32,
    pub datp_record_index: u32,
    pub datp_record_count: u32,
    pub exth_flags: u32,
    // ... more fields
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobiFormat {
    Mobi6,
    Mobi7,
    Kf8,
    Azw3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    PalmDoc,
    Huff,
}
```

## EXTH Header

Extended metadata header:

```rust
pub struct ExthHeader {
    pub identifier: [u8; 4],  // "EXTH"
    pub header_length: u32,
    pub record_count: u32,
    pub records: Vec<ExthRecord>,
}

pub struct ExthRecord {
    pub record_type: u32,
    pub record_length: u32,
    pub data: Vec<u8>,
}
```

### EXTH Record Types

| Type | Name | Description |
|------|------|-------------|
| 100 | Author | Author name |
| 101 | Publisher | Publisher name |
| 102 | Imprint | Imprint |
| 103 | Description | Book description |
| 104 | ISBN | ISBN identifier |
| 106 | Published | Publication date |
| 108 | Title | Book title |
| 109 | ASIN | Amazon ASIN |
| 110 | Language | Language code |
| 112 | Cover | Cover image offset |
| 116 | Region | DRM region |
| 121 | ContentType | Content type (EBOK, PDOC) |
| 125 | Updated | Last update time |
| 200 | Cover_uri | Cover image URI |
| 201 | Thumb_uri | Thumbnail URI |
| 202 | Content_uri | Content URI |
| 203 | Aws_drm | AWS DRM data |
| 204 | Acs_drm_uri | ACS DRM URI |
| 205 | Acs_drm_pub | ACS DRM publisher |
| 206 | Acs_drm_text | ACS DRM text |
| 207 | Acs_drm_key | ACS DRM key |
| 208 | Acs_drm_key_fmt | ACS DRM key format |
| 209 | Acs_drm_key_len | ACS DRM key length |
| 210 | Acs_drm_crit | ACS DRM criteria |
| 501 | Pdf_seed | PDF seed |
| 502 | Doc_seed | Document seed |
| 503 | Reader_seed | Reader seed |
| 504 | Cover_offset | Cover image offset |
| 505 | Thumb_offset | Thumbnail offset |
| 506 | Has_fake_cover | Has fake cover |
| 524 | Clipping_limit | Clipping limit |
| 525 | Publisher_limit | Publisher limit |
| 526 | Tts_disabled | TTS disabled |
| 527 | Tts_locked | TTS locked |
| 528 | Tts_page_lock | TTS page lock |
| 529 | Tts_word_lock | TTS word lock |
| 530 | Tts_char_lock | TTS character lock |
| 531 | Rental_flag | Rental flag |
| 534 | Rental_duration | Rental duration |
| 535 | Rental_type | Rental type |
| 536 | Epub_type | EPUB type |
| 537 | Epub_version | EPUB version |
| 538 | Epub_source | EPUB source |
| 539 | Epub_original_source | EPUB original source |
| 540 | Epub_original_guid | EPUB original GUID |
| 541 | Epub_guid | EPUB GUID |
| 542 | Epub_asin | EPUB ASIN |
| 543 | Epub_sku | EPUB SKU |
| 544 | Eub_cover_uri | EPUB cover URI |
| 545 | Epub_thumb_uri | EPUB thumbnail URI |
| 546 | Epub_content_uri | EPUB content URI |
| 547 | Epub_metadata_uri | EPUB metadata URI |
| 548 | Epub_sourcerecommendsuri | EPUB source recommends URI |
| 549 | Epub_inarticlerecommendsuri | EPUB in-article recommends URI |
| 550 | Epub_websiterecommendsuri | EPUB website recommends URI |
| 551 | Epub_preview_uri | EPUB preview URI |
| 552 | Epub_sample_uri | EPUB sample URI |
| 553 | Epub_buy_url | EPUB buy URL |
| 554 | Epub_direct_buy_url | EPUB direct buy URL |
| 555 | Epub_relation | EPUB relation |
| 556 | Epub_resource_type | EPUB resource type |
| 557 | Epub_origin_id | EPUB origin ID |
| 558 | Epub_preview_type | EPUB preview type |
| 559 | Epub_resource_link | EPUB resource link |

## Compression

### PalmDoc Compression

Simple RLE-like compression:

```rust
pub fn decompress_palmdoc(compressed: &[u8]) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut i = 0;

    while i < compressed.len() {
        let byte = compressed[i];
        i += 1;

        if byte == 0 {
            // Literal byte
            if i < compressed.len() {
                output.push(compressed[i]);
                i += 1;
            }
        } else if byte <= 8 {
            // Run of spaces
            for _ in 0..=byte {
                output.push(b' ');
            }
        } else if byte < 128 {
            // Literal byte
            output.push(byte);
        } else if byte >= 128 {
            // Repeat sequence
            let n = (byte & 0x7F) + 3;
            if i + 1 < compressed.len() {
                let offset = u16::from_be_bytes([compressed[i], compressed[i + 1]]);
                i += 2;

                for _ in 0..n {
                    if offset as usize <= output.len() {
                        let copy = output[output.len() - offset as usize];
                        output.push(copy);
                    }
                }
            }
        }
    }

    Ok(output)
}
```

### HUFF/CDIC Compression

Huffman coding with dictionary:

```rust
pub struct HuffCdicReader {
    pub codes: Vec<HuffCode>,
    pub dictionary: Vec<String>,
}

pub struct HuffCode {
    pub code: u32,
    pub bits: u8,
    pub value: u32,
}

impl HuffCdicReader {
    pub fn decompress(&self, compressed: &[u8]) -> io::Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut bit_buffer = 0u32;
        let mut bits_available = 0;
        let mut i = 0;

        while i < compressed.len() {
            // Refill bit buffer
            while bits_available < 24 && i < compressed.len() {
                bit_buffer |= (compressed[i] as u32) << bits_available;
                bits_available += 8;
                i += 1;
            }

            // Find matching code
            for code in &self.codes {
                if bits_available >= code.bits as u32 {
                    let mask = (1u32 << code.bits) - 1;
                    let value = bit_buffer & mask;

                    if value == code.code {
                        // Output character or dictionary reference
                        if code.value < 256 {
                            output.push(code.value as u8);
                        } else {
                            let dict_index = code.value - 256;
                            if let Some(word) = self.dictionary.get(dict_index as usize) {
                                output.extend_from_slice(word.as_bytes());
                            }
                        }

                        bit_buffer >>= code.bits as u32;
                        bits_available -= code.bits as u32;
                        break;
                    }
                }
            }
        }

        Ok(output)
    }
}
```

## Index Structures

### NCX Index

Navigation control XML (similar to EPUB NCX):

```rust
pub struct NcxEntry {
    pub chapter_id: u32,
    pub offset: u32,
    pub size: u32,
    pub label: String,
    pub children: Vec<NcxEntry>,
}
```

### INDX Index

General-purpose index:

```rust
pub struct IndxHeader {
    pub index_type: u32,
    pub identifier: u32,
    pub header_length: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub encoding: u32,
    pub tag_offset: u32,
    pub tag_count: u32,
    pub idxt_offset: u32,
    pub num_entries: u32,
}
```

### FDST (Flow Data)

Page break information:

```rust
pub fn parse_fdst(data: &[u8]) -> io::Result<Vec<PageEntry>> {
    // Parse flow data entries
}

pub struct PageEntry {
    pub record_number: u32,
    pub offset: u32,
}
```

## MOBI Modules

### `mobi/parser.rs`

Main MOBI parser:

```rust
pub fn parse_mobi(data: &[u8]) -> io::Result<MobiBook> {
    // Parse PalmDB header
    // Parse MOBI header
    // Parse EXTH header
    // Parse text records
    // Parse images
    // Parse indexes
}

pub struct MobiBook {
    pub header: MobiHeader,
    pub exth: Option<ExthHeader>,
    pub text: Vec<u8>,
    pub images: Vec<Image>,
    pub ncx: Vec<NcxEntry>,
    pub metadata: Metadata,
}
```

### `mobi/huffcdic.rs`

HUFF/CDIC decompression:

```rust
pub fn parse_huffcdic(data: &[u8]) -> io::Result<HuffCdicReader> {
    // Parse HUFF header
    // Parse CDIC dictionary
    // Build code table
}
```

### `mobi/index.rs`

Index parsing:

```rust
pub fn parse_ncx_index(data: &[u8]) -> io::Result<Vec<NcxEntry>> {
    // Parse NCX index structure
}

pub fn read_index(index: &IndexRecord, data: &[u8]) -> io::Result<Vec<u8>> {
    // Read index data
}
```

### `mobi/transform.rs`

MOBI → IR transformation:

```rust
pub fn transform_mobi(mobi: MobiBook) -> io::Result<Book> {
    // Transform MOBI to IR
    // Extract metadata
    // Build TOC
    // Load chapters
}
```

### `mobi/skeleton.rs`

MOBI skeleton structure for export:

```rust
pub struct MobiSkeleton {
    pub header: MobiHeader,
    pub exth: Vec<ExthRecord>,
    pub text_records: Vec<Vec<u8>>,
    pub image_records: Vec<Vec<u8>>,
    pub index_records: Vec<Vec<u8>>,
}
```

### `mobi/writer_transform.rs`

IR → MOBI transformation for export:

```rust
pub fn transform_to_mobi(book: &Book) -> io::Result<MobiSkeleton> {
    // Transform IR to MOBI
    // Generate headers
    // Compress text
    // Build indexes
}
```

### `mobi/filepos.rs`

File position handling for link resolution:

```rust
pub struct FilePos(u32);

impl FilePos {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        // Parse variable-length integer
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Serialize to variable-length integer
    }
}
```

## MOBI Import Process

```
1. Parse PalmDB header
   ↓
2. Parse MOBI header
   ↓
3. Parse EXTH header (if present)
   ↓
4. Read text records
   ↓
5. Decompress text (HUFF/CDIC or PalmDoc)
   ↓
6. Parse images
   ↓
7. Parse NCX index for TOC
   ↓
8. Extract metadata
   ↓
9. Transform to IR
```

## MOBI Export Process

```
1. Transform IR to MOBI format
   ↓
2. Generate MOBI header
   ↓
3. Generate EXTH header
   ↓
4. Compress text (HUFF/CDIC or PalmDoc)
   ↓
5. Pack text into records
   ↓
6. Pack images into records
   ↓
7. Generate NCX index
   ↓
8. Generate INDX indexes
   ↓
9. Generate PalmDB header
   ↓
10. Write output file
```

## Kindle Base32 Encoding

Kindle uses a custom base32 encoding for resource references:

```rust
pub fn parse_base32(s: &[u8]) -> usize {
    let mut result = 0usize;
    for &b in s {
        result = result.wrapping_mul(32);
        let val = match b {
            b'0'..=b'9' => (b - b'0') as usize,
            b'A'..=b'V' => (b - b'A') as usize + 10,
            b'a'..=b'v' => (b - b'a') as usize + 10,
            _ => continue,
        };
        result = result.wrapping_add(val);
    }
    result
}
```

Used in Kindle URIs like `kindle:embed:0001`.

## MOBI Limitations

1. **Limited CSS support**: MOBI has limited formatting capabilities
2. **Proprietary format**: Only works on Kindle devices
3. **Complex compression**: HUFF/CDIC is complex to implement
4. **Legacy format**: Replaced by KFX for new content
5. **No EPUB3 features**: Limited to EPUB2-level features

## Debugging MOBI

### Dump MOBI Structure

```bash
# Dump MOBI headers
mobi-dump book.mobi

# Dump with hex output
mobi-dump --hex book.mobi

# Dump decompressed text
mobi-dump --text book.mobi
```

### Inspect HUFF/CDIC

```rust
let huff = parse_huffcdic(&data)?;
println!("Codes: {}", huff.codes.len());
println!("Dictionary: {}", huff.dictionary.len());
```

### Validate Indexes

```rust
let ncx = parse_ncx_index(&data)?;
for entry in ncx {
    println!("Chapter: {} at offset {}", entry.label, entry.offset);
}
```

## Further Reading

- [MOBI Format Specification](https://wiki.mobileread.com/wiki/MOBI)
- [PalmDB Format](https://docs.fileformat.com/database/palm-database/)
- `src/mobi/parser.rs` - MOBI parser implementation
- `src/mobi/huffcdic.rs` - HUFF/CDIC decompression
