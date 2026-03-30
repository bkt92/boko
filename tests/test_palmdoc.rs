//! Simple PalmDoc compression test

use boko::mobi::palmdoc;

fn main() {
    // Test 1: Simple compression/decompression
    let original = b"Hello, World! This is a test.";
    println!("Original: {} bytes", original.len());
    println!("Data: {:?}", String::from_utf8_lossy(original));

    let compressed = palmdoc::compress(original);
    println!("Compressed: {} bytes", compressed.len());

    let decompressed = palmdoc::decompress(&compressed).unwrap();
    println!("Decompressed: {} bytes", decompressed.len());
    println!("Data: {:?}", String::from_utf8_lossy(&decompressed));
    println!("Match: {}", original.to_vec() == decompressed);
    println!();

    // Test 2: Larger text
    let text = b"<html><head></head><body><p>Hello</p></body></html>";
    println!("Test 2 - Original: {} bytes", text.len());
    println!("Data: {:?}", String::from_utf8_lossy(text));

    let compressed2 = palmdoc::compress(text);
    println!("Compressed: {} bytes", compressed2.len());

    let decompressed2 = palmdoc::decompress(&compressed2).unwrap();
    println!("Decompressed: {} bytes", decompressed2.len());
    println!("Data: {:?}", String::from_utf8_lossy(&decompressed2));
    println!("Match: {}", text.to_vec() == decompressed2);
}
