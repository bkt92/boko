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
    if data.len() >= 12
        && &data[0..4] == b"RIFF"
        && &data[8..12] == b"WEBP"
    {
        // WebP will be converted to JPEG or PNG
        return Some(ImageFormat::Jpeg);
    }

    None
}

/// Check if image format is natively supported (no conversion needed)
pub fn is_supported_format(data: &[u8]) -> bool {
    detect_format(data).is_some()
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
}
