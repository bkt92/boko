//! Compare HTML content between EPUB and generated MOBI

use std::fs::File;
use std::io::{self, Read};
use std::process::Command;

fn main() -> io::Result<()> {
    // Extract HTML from original EPUB
    let output = Command::new("unzip")
        .args(&["-p", "tests/fixtures/test_book.epub", "OEBPS/content.html"])
        .output()
        .expect("Failed to run unzip");

    let original_html = String::from_utf8_lossy(&output.stdout).to_string();

    // Extract HTML from generated MOBI (via round-trip)
    // This will fail but we can see what HTML we're generating
    println!("Original EPUB HTML length: {} bytes", original_html.len());
    println!(
        "Original HTML (first 500 chars):\n{}\n",
        &original_html[..500.min(original_html.len())]
    );

    // Count image tags
    let img_count = original_html.matches("<img").count();
    println!("Image count in original: {}", img_count);

    // Look for specific patterns
    let has_html_body = original_html.contains("<html>");
    let has_head = original_html.contains("<head>");
    let has_body = original_html.contains("<body>");

    println!("\nHTML structure tags:");
    println!("  Has <html>: {}", has_html_body);
    println!("  Has <head>: {}", has_head);
    println!("  Has <body>: {}", has_body);

    // Count the 420 bytes difference
    println!("\nDifference analysis:");
    println!("  We're adding 420 bytes somehow");
    println!("  Image recindex conversion might add bytes");

    // Check first image tag
    if let Some(pos) = original_html.find("<img") {
        let end = original_html[pos..].find('>').unwrap_or(100);
        println!("\nFirst image tag: {}", &original_html[pos..pos + end]);
    }

    Ok(())
}
