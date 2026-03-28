# MOBI Export Test Fixtures

This directory contains test EPUB files for MOBI 6 export testing.

## Fixtures Needed

- `simple.epub` - Basic text content, no images, simple structure
- `with_images.epub` - Contains PNG/JPEG images
- `complex.epub` - Tables, nested lists, internal links

## Creating Fixtures

Fixtures can be created using any EPUB authoring tool or by converting from other formats.

Example using pandoc:
```bash
pandoc input.md -o simple.epub
```
