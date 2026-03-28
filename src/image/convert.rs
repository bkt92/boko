//! Image processing functions for format conversion and optimization.

/// Image format options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    /// Keep original format if supported
    Auto,
    /// JPEG format (photos, gradients)
    Jpeg,
    /// PNG format (graphics, transparency)
    Png,
    /// GIF format (animated images)
    Gif,
}

/// Configuration for image processing
#[derive(Clone, Debug)]
pub struct ImageConfig {
    /// Maximum dimensions (width, height)
    pub max_dimensions: (u32, u32),
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Output format preference
    pub output_format: ImageFormat,
    /// JPEG quality (1-100, default 85)
    pub jpeg_quality: u8,
    /// PNG compression level (0-9, default 6)
    pub png_compression: u8,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            max_dimensions: (2048, 2048),
            max_file_size: 10 * 1024 * 1024, // 10MB
            output_format: ImageFormat::Auto,
            jpeg_quality: 85,
            png_compression: 6,
        }
    }
}

/// Detect image format from magic bytes
pub fn detect_format(data: &[u8]) -> Option<ImageFormat> {
    if data.len() < 4 {
        return None;
    }

    // JPEG: FF D8 FF
    if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        return Some(ImageFormat::Jpeg);
    }

    // PNG: 89 50 4E 47
    if data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47 {
        return Some(ImageFormat::Png);
    }

    // GIF: 47 49 46 38
    if data[0] == 0x47 && data[1] == 0x49 && data[2] == 0x46 && data[3] == 0x38 {
        return Some(ImageFormat::Gif);
    }

    // WebP: RIFF...WEBP
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        // WebP will be converted to JPEG or PNG
        return Some(ImageFormat::Jpeg);
    }

    None
}

/// Check if image format is natively supported (no conversion needed)
pub fn is_supported_format(data: &[u8]) -> bool {
    detect_format(data).is_some()
}

use image::GenericImageView;
use std::io::{self, Cursor};

/// Process image data according to configuration
///
/// Returns (processed_image_data, warnings)
/// Returns Ok(Vec::new()) if image should be skipped
pub fn process_image(data: &[u8], config: &ImageConfig) -> io::Result<(Vec<u8>, Vec<String>)> {
    let mut warnings = Vec::new();

    // Detect format
    let format = match detect_format(data) {
        Some(f) => f,
        None => {
            // Unknown format - try to load with image crate
            return Ok((Vec::new(), vec!["Unsupported image format".to_string()]));
        }
    };

    // Load image
    let img = match load_image(data, format) {
        Ok(img) => img,
        Err(e) => {
            warnings.push(format!("Failed to load image: {}", e));
            return Ok((Vec::new(), warnings));
        }
    };

    // Check dimensions and downsample if needed
    let (width, height) = img.dimensions();
    let img = if width > config.max_dimensions.0 || height > config.max_dimensions.1 {
        warnings.push(format!(
            "Image too large ({}x{}), downsampling to max {:?}",
            width, height, config.max_dimensions
        ));

        // Actually downsample the image
        use image::imageops::FilterType;
        img.resize(
            config.max_dimensions.0.min(width),
            config.max_dimensions.1.min(height),
            FilterType::Lanczos3,
        )
    } else {
        img
    };

    // Determine output format
    let output_format = match config.output_format {
        ImageFormat::Auto => format,
        f => f,
    };

    // Encode image
    let encoded = match encode_image(&img, output_format, config) {
        Ok(data) => data,
        Err(e) => {
            warnings.push(format!("Failed to encode image: {}", e));
            return Ok((Vec::new(), warnings));
        }
    };

    // Check file size
    if encoded.len() as u64 > config.max_file_size {
        warnings.push(format!(
            "Image too large ({} bytes), consider quality reduction",
            encoded.len()
        ));
    }

    Ok((encoded, warnings))
}

fn load_image(data: &[u8], format: ImageFormat) -> Result<image::DynamicImage, io::Error> {
    use image::{ImageFormat as ImgFormat, ImageReader};

    let img_format = match format {
        ImageFormat::Jpeg => ImgFormat::Jpeg,
        ImageFormat::Png => ImgFormat::Png,
        ImageFormat::Gif => ImgFormat::Gif,
        ImageFormat::Auto => {
            // Try to detect from data
            let cursor = Cursor::new(data);
            let jpeg_result = ImageReader::with_format(cursor, ImgFormat::Jpeg).decode();
            if jpeg_result.is_ok() {
                return jpeg_result.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
            }
            let cursor = Cursor::new(data);
            let png_result = ImageReader::with_format(cursor, ImgFormat::Png).decode();
            if png_result.is_ok() {
                return png_result.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
            }
            let cursor = Cursor::new(data);
            return ImageReader::with_format(cursor, ImgFormat::Gif)
                .decode()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
        }
    };

    let cursor = Cursor::new(data);
    ImageReader::with_format(cursor, img_format)
        .decode()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn encode_image(
    img: &image::DynamicImage,
    format: ImageFormat,
    config: &ImageConfig,
) -> Result<Vec<u8>, io::Error> {
    use image::codecs::gif::GifEncoder;
    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::png::{CompressionType as PngCompression, PngEncoder};
    use image::{ExtendedColorType, Frame, ImageEncoder};

    let mut output = Vec::new();

    match format {
        ImageFormat::Jpeg => {
            let encoder = JpegEncoder::new_with_quality(&mut output, config.jpeg_quality);
            encoder
                .write_image(
                    img.to_rgb8().as_raw(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::Rgb8,
                )
                .map_err(io::Error::other)?;
        }
        ImageFormat::Png => {
            let encoder = PngEncoder::new_with_quality(
                &mut output,
                PngCompression::Fast,
                image::codecs::png::FilterType::NoFilter,
            );
            encoder
                .write_image(
                    img.to_rgb8().as_raw(),
                    img.width(),
                    img.height(),
                    ExtendedColorType::Rgb8,
                )
                .map_err(io::Error::other)?;
        }
        ImageFormat::Gif => {
            let mut encoder = GifEncoder::new(&mut output);
            let frame = Frame::new(img.to_rgba8());
            encoder.encode_frame(frame).map_err(io::Error::other)?;
        }
        ImageFormat::Auto => {
            // Shouldn't happen - format should be resolved by now
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Auto format not supported for encoding",
            ));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    // JPEG magic bytes: FF D8 FF
    #[test]
    fn test_detect_jpeg() {
        let data = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        assert_eq!(detect_format(data), Some(ImageFormat::Jpeg));
    }

    // PNG magic bytes: 89 50 4E 47
    #[test]
    fn test_detect_png() {
        let data = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_format(data), Some(ImageFormat::Png));
    }

    // GIF magic bytes: 47 49 46 38
    #[test]
    fn test_detect_gif() {
        let data = &[0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
        assert_eq!(detect_format(data), Some(ImageFormat::Gif));
    }

    // WebP magic bytes: 52 49 46 46 ... 57 45 42 50
    #[test]
    fn test_detect_webp() {
        // WebP format: RIFF (4 bytes) + file_size (4 bytes) + WEBP (4 bytes)
        let mut data = vec![0x52, 0x49, 0x46, 0x46]; // "RIFF"
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // file size (placeholder)
        data.extend_from_slice(b"WEBP"); // "WEBP"
        assert_eq!(detect_format(&data), Some(ImageFormat::Jpeg)); // WebP converts to JPEG
    }

    #[test]
    fn test_detect_unknown() {
        let data = &[0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_format(data), None);
    }

    #[test]
    fn test_process_jpeg_no_conversion_needed() {
        // Create a minimal JPEG in memory using the image crate
        use image::{ImageEncoder, Rgb, RgbImage};

        // Create a simple 10x10 red image
        let img = RgbImage::from_fn(10, 10, |_, _| Rgb([255, 0, 0]));

        // Encode as JPEG
        let mut jpeg_data = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_data, 85);
        encoder
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgb8,
            )
            .expect("Failed to encode test JPEG");

        let config = ImageConfig {
            max_dimensions: (100, 100),
            max_file_size: 1024 * 1024,
            output_format: ImageFormat::Auto,
            ..Default::default()
        };

        let (result, warnings) = process_image(&jpeg_data, &config).unwrap();

        // Should preserve JPEG unchanged (no downsampling needed)
        assert!(!result.is_empty());
        assert_eq!(warnings.len(), 0);
        assert_eq!(detect_format(&result), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn test_process_jpeg_downsampling() {
        // Create a JPEG that's larger than max dimensions
        use image::{ImageEncoder, Rgb, RgbImage};

        let img = RgbImage::from_fn(200, 200, |_, _| Rgb([255, 0, 0]));

        let mut jpeg_data = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_data, 85);
        encoder
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgb8,
            )
            .expect("Failed to encode test JPEG");

        let config = ImageConfig {
            max_dimensions: (100, 100),
            max_file_size: 1024 * 1024,
            output_format: ImageFormat::Auto,
            ..Default::default()
        };

        let (result, warnings) = process_image(&jpeg_data, &config).unwrap();

        // Should downsample and warn
        assert!(!result.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("downsampling"));
        assert_eq!(detect_format(&result), Some(ImageFormat::Jpeg));
    }
}
