use image::{Rgb, RgbImage};
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let img = RgbImage::from_fn(10, 10, |_, _| Rgb([255, 0, 0]));
    let file = File::create("tests/fixtures/image/test.jpg")?;
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
        BufWriter::new(file),
        85
    );
    encoder.encode(
        img.as_raw(),
        10,
        10,
        image::ExtendedColorType::Rgb8
    )?;
    println!("Created test.jpg");
    Ok(())
}
