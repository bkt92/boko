//! Shared image processing utilities.
//!
//! This module provides image processing functionality used across multiple exporters:
//! - MOBI 6: Downscale and convert for legacy device compatibility
//! - EPUB/AZW3: Optimize images to reduce file size (future)
//!
//! Design principles:
//! - Format-agnostic: No format-specific logic
//! - Configurable: All settings via ImageConfig
//! - Reusable: Return processed data + warnings

pub mod convert;

pub use convert::{ImageConfig, ImageFormat, detect_format, is_supported_format, process_image};
