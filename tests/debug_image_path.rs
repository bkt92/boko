//! Debug image path matching

use std::collections::HashMap;

fn main() {
    let html = r#"<html><body><img src="images/image_0015.jpg" width="800"></body></html>"#;

    let mut image_map: HashMap<String, u32> = HashMap::new();
    image_map.insert("OEBPS/images/image_0015.jpg".to_string(), 1);
    image_map.insert("images/image_0002.jpg".to_string(), 2);

    println!("HTML: {}", html);
    println!("\nImage map:");
    for (path, idx) in &image_map {
        println!("  {} -> {}", path, idx);
    }

    println!("\n--- Testing matches ---");

    for (image_path, &record_index) in &image_map {
        println!("\nChecking: {} (record {})", image_path, record_index);

        // Check if HTML contains this path
        if html.contains(image_path) {
            println!("  ✓ HTML contains full path");
        } else {
            println!("  ✗ HTML does NOT contain full path");
        }

        // Try just filename
        if let Some(name) = std::path::Path::new(image_path).file_name() {
            let filename = name.to_string_lossy();
            let file_path = format!("src=\"{}\"", filename);
            println!("  Trying filename: {}", file_path);
            if html.contains(&file_path) {
                println!("  ✓ HTML contains filename!");
            } else {
                println!("  ✗ HTML does NOT contain filename");
            }
        }

        // Try just path without filename
        if let Some(parent) = std::path::Path::new(image_path).parent() {
            if let Some(name) = parent.file_name() {
                let dirname = name.to_string_lossy();
                if let Some(filename) = std::path::Path::new(image_path).file_name() {
                    let filename = filename.to_string_lossy();
                    let relative_path = format!("{}/{}", dirname, filename);
                    let file_path = format!("src=\"{}\"", relative_path);
                    println!("  Trying relative path: {}", file_path);
                    if html.contains(&file_path) {
                        println!("  ✓ HTML contains relative path!");
                    } else {
                        println!("  ✗ HTML does NOT contain relative path");
                    }
                }
            }
        }
    }
}
