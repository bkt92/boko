# Contributing to Boko

Thank you for your interest in contributing to Boko! This guide will help you get started.

## Table of Contents

- [Development Setup](#development-setup)
- [Running Tests](#running-tests)
- [Code Style](#code-style)
- [Submitting Changes](#submitting-changes)
- [Adding Format Support](#adding-format-support)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)

## Development Setup

### Prerequisites

- Rust 2024 edition or later
- Git

### Clone Repository

```bash
git clone https://github.com/zacharydenton/boko.git
cd boko
```

### Build Project

```bash
cargo build --release
```

### Run Tests

```bash
cargo test --verbose
```

## Running Tests

### All Tests

```bash
cargo test
```

### Specific Test

```bash
cargo test test_name
```

### Specific Crate

```bash
cargo test -p boko
```

### With Output

```bash
cargo test -- --nocapture
```

### Run Tests in Parallel

```bash
cargo test -- --test-threads=1
```

## Code Style

### Rustfmt

Format code with `rustfmt`:

```bash
cargo fmt
```

Check formatting without modifying:

```bash
cargo fmt -- --check
```

### Clippy

Check code with Clippy:

```bash
cargo clippy -- -D warnings
```

Fix Clippy warnings automatically:

```bash
cargo clippy --fix --allow-dirty --allow-staged
```

## Submitting Changes

### Workflow

1. Fork the repository
2. Create a branch: `git checkout -b feature/your-feature`
3. Make changes and commit
4. Run tests: `cargo test`
5. Run Clippy: `cargo clippy -- -D warnings`
6. Format code: `cargo fmt`
7. Push to fork: `git push origin feature/your-feature`
8. Create pull request

### Commit Messages

Use clear, descriptive commit messages:

```
Add KFX export support

- Implement KFX exporter
- Add Ion serialization
- Generate style registry
- Write tests for KFX output
```

### Pull Request Guidelines

- Describe what your PR does
- Reference related issues
- Include tests for new features
- Update documentation if needed
- Ensure CI passes

## Adding Format Support

### Step 1: Implement Importer

Create `src/import/your_format.rs`:

```rust
use crate::import::{Importer, ChapterId, SpineEntry};
use crate::model::{Chapter, Metadata, TocEntry, Landmark};
use std::io;
use std::path::Path;

pub struct YourFormatImporter {
    // Format-specific state
}

impl Importer for YourFormatImporter {
    fn open(path: &Path) -> io::Result<Self>
    where
        Self: Sized,
    {
        // Parse file and initialize importer
        todo!()
    }

    fn metadata(&self) -> &Metadata {
        // Return metadata
        todo!()
    }

    fn toc(&self) -> &[TocEntry] {
        // Return table of contents
        todo!()
    }

    fn landmarks(&self) -> &[Landmark] {
        // Return landmarks
        todo!()
    }

    fn spine(&self) -> &[SpineEntry] {
        // Return reading order
        todo!()
    }

    fn source_id(&self, id: ChapterId) -> Option<&str> {
        // Return source path for chapter
        todo!()
    }

    fn load_raw(&mut self, id: ChapterId) -> io::Result<Vec<u8>> {
        // Return raw chapter bytes
        todo!()
    }

    fn list_assets(&self) -> &[std::path::PathBuf] {
        // Return list of assets
        todo!()
    }

    fn load_asset(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        // Load asset by path
        todo!()
    }

    // Optional: override load_chapter for custom parsing
    fn load_chapter(&mut self, id: ChapterId) -> io::Result<Chapter> {
        // Default implementation uses load_raw + compile_html
        // Override for format-specific optimization
        self.load_chapter_default(id)
    }
}
```

### Step 2: Implement Exporter

Create `src/export/your_format.rs`:

```rust
use crate::export::Exporter;
use crate::model::Book;
use std::io::{self, Seek, Write};

pub struct YourFormatExporter {
    // Configuration
}

impl YourFormatExporter {
    pub fn new() -> Self {
        Self {
            // Default configuration
        }
    }
}

impl Exporter for YourFormatExporter {
    fn export<W: Write + Seek>(
        &self,
        book: &mut Book,
        writer: &mut W,
    ) -> io::Result<()> {
        // Export book to format
        todo!()
    }
}
```

### Step 3: Wire Up

Update `src/model/book.rs`:

```rust
impl Format {
    pub fn from_path(path: impl AsRef<Path>) -> Option<Self> {
        path.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "epub" => Some(Format::Epub),
                "azw3" => Some(Format::Azw3),
                "mobi" => Some(Format::Mobi),
                "kfx" => Some(Format::Kfx),
                "your_ext" => Some(Format::YourFormat),  // Add this
                _ => None,
            })
    }
}

impl Book {
    pub fn open_format(path: &Path, format: Format) -> io::Result<Self> {
        let backend: Box<dyn Importer> = match format {
            Format::Epub => Box::new(EpubImporter::open(path)?),
            Format::Azw3 => Box::new(Azw3Importer::open(path)?),
            Format::Mobi => Box::new(MobiImporter::open(path)?),
            Format::Kfx => Box::new(KfxImporter::open(path)?),
            Format::YourFormat => Box::new(YourFormatImporter::open(path)?),  // Add this
            Format::Markdown => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Markdown format is export-only",
                ))
            }
        };
        Ok(Self {
            backend,
            ir_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn export<W>(&mut self, format: Format, writer: &mut W) -> io::Result<()>
    where
        W: Write + Seek,
    {
        match format {
            Format::Epub => EpubExporter::new().export(self, writer),
            Format::Azw3 => Azw3Exporter::new().export(self, writer),
            Format::Markdown => MarkdownExporter::new().export(self, writer),
            Format::Kfx => KfxExporter::new().export(self, writer),
            Format::YourFormat => YourFormatExporter::new().export(self, writer),  // Add this
            Format::Mobi => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "MOBI export is not supported",
            )),
        }
    }
}
```

### Step 4: Add to Modules

Update `src/import/mod.rs`:

```rust
pub use your_format::YourFormatImporter;
```

Update `src/export/mod.rs`:

```rust
pub use your_format::{YourFormatExporter, YourFormatConfig};
```

## Testing Guidelines

### Unit Tests

Write unit tests in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata() {
        let metadata = parse_metadata(/* ... */);
        assert_eq!(metadata.title, "Test Book");
    }
}
```

### Integration Tests

Add tests in `tests/` directory:

```rust
// tests/format_test.rs
use boko::Book;

#[test]
fn test_round_trip() {
    let mut book = Book::open("test.epub").unwrap();
    // ... test round-trip conversion
}
```

### Property Tests

Use `proptest` for property-based testing:

```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_parse_roundtrip(css in "[a-z]+\\s*:\\s*[a-z]+;") {
            let stylesheet = Stylesheet::parse(&css);
            // ... test round-trip
        }
    }
}
```

### Test Fixtures

Place test fixtures in `tests/fixtures/`:

```
tests/
├── fixtures/
│   ├── simple.epub
│   ├── complex.epub
│   └── metadata-test.epub
└── format_test.rs
```

### Use Fixtures in Tests

```rust
#[test]
fn test_simple_epub() {
    let path = "tests/fixtures/simple.epub";
    let mut book = Book::open(path).unwrap();
    assert_eq!(book.metadata().title, "Simple Book");
}
```

## Documentation

### Code Documentation

Document public APIs:

```rust
/// Parse a CSS string into a Stylesheet.
///
/// # Arguments
///
/// * `css` - The CSS source code to parse
///
/// # Returns
///
/// A `Stylesheet` containing parsed rules
///
/// # Example
///
/// ```
/// use boko::dom::Stylesheet;
///
/// let css = "p { color: blue; }";
/// let sheet = Stylesheet::parse(css);
/// ```
pub fn parse_stylesheet(css: &str) -> Stylesheet {
    // ...
}
```

### Module Documentation

Add module-level documentation:

```rust
//! EPUB format support
//!
//! This module provides EPUB import and export functionality.
//!
//! ## Features
//!
//! - EPUB 2 and 3 support
//! - Extended metadata (contributors, collections)
//! - Proper ZIP archive structure
//!
//! ## Example
//!
//! ```no_run
//! use boko::Book;
//!
//! let mut book = Book::open("book.epub").unwrap();
//! println!("Title: {}", book.metadata().title);
//! ```

pub mod parser {
    // ...
}
```

### Update Documentation

When adding features, update:
- This `CONTRIBUTING.md` if workflow changes
- `docs/` files if architecture changes
- `README.md` if user-facing features change
- `CLAUDE.md` if development patterns change

## Performance Guidelines

### Profiling

Use `cargo bench` for benchmarks:

```bash
cargo bench --bench conversion
```

### Optimization Tips

1. **Use raw export when possible**: Faster than IR synthesis
2. **Cache IR chapters**: Use `load_chapter_cached()`
3. **Avoid allocations**: Reuse buffers where possible
4. **Profile before optimizing**: Measure first
5. **Consider WASM**: Keep code WASM-compatible

### Memory Usage

- Use `Arc` for shared data
- Consider `bytes::Bytes` for large byte arrays
- Clear caches when done: `book.clear_cache()`

## Debugging

### Logging

Enable logging:

```rust
env_logger::init();
```

Run with logging:

```bash
RUST_LOG=debug cargo test
```

### Dump IR

Use CLI to inspect IR:

```bash
# Dump structure
boko dump book.epub --structure

# Dump styles
boko dump book.epub --styles-only

# Dump as JSON
boko dump book.epub --json
```

### Debug Format Issues

For format-specific issues:

```rust
// Enable format-specific debugging
let importer = MobiImporter::open_with_debug(path, true)?;
```

## Common Issues

### Build Errors

**Problem**: Build fails with "unsupported crate attribute"

**Solution**: Ensure Rust 2024 edition is installed:
```bash
rustup update stable
```

### Test Failures

**Problem**: Tests fail with "file not found"

**Solution**: Run tests from repository root:
```bash
cargo test
```

### Clippy Warnings

**Problem**: Clippy reports warnings

**Solution**: Fix or suppress with justification:
```rust
#[allow(clippy::too_many_arguments)]  // Required by format spec
fn function_with_many_args(/* ... */) {
    // ...
}
```

## Getting Help

- Check existing [GitHub Issues](https://github.com/zacharydenton/boko/issues)
- Read [Documentation](index.md)
- Ask questions in issues (label: `question`)

## Code Review Process

1. Submit pull request
2. Address review feedback
3. Ensure CI passes
4. Request review from maintainers
5. Merge after approval

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
