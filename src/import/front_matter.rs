//! YAML front matter parsing and serialization for Markdown import/export.
//!
//! This module provides types and functions for:
//! - Extracting YAML front matter from Markdown files
//! - Converting front matter to/from `Metadata` structs
//! - Serializing front matter for export

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{CollectionInfo, Contributor, Metadata};

// ============================================================================
// Data Types
// ============================================================================

/// Front matter contributor with role and sort name.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FrontMatterContributor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "file-as")]
    pub file_as: Option<String>,
}

/// Front matter collection/series information.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FrontMatterCollection {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "type")]
    pub collection_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub position: Option<f64>,
}

/// YAML front matter extracted from Markdown files.
///
/// This struct mirrors the `Metadata` struct but with field names and defaults
/// optimized for Markdown front matter.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FrontMatter {
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub title: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub authors: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty", default = "default_language")]
    pub language: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub identifier: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub subjects: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rights: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub contributors: Vec<FrontMatterContributor>,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "title-sort")]
    pub title_sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "author-sort")]
    pub author_sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub collection: Option<FrontMatterCollection>,
}

fn default_language() -> String {
    "en".to_string()
}

// ============================================================================
// Extraction and Conversion
// ============================================================================

/// Extract YAML front matter from the beginning of content.
///
/// Front matter must start with `---\n` and end with `\n---`. The remaining
/// content after the closing `---` is returned via the offset.
///
/// Returns `None` if no valid front matter is found at the start of the content.
///
/// # Example
///
/// ```rust
/// use boko::import::front_matter::extract_front_matter;
///
/// let content = r#"---
/// title: My Book
/// author: John Doe
/// ---
///
/// # Chapter 1
/// "#;
///
/// let (fm, offset) = extract_front_matter(content).unwrap();
/// assert_eq!(fm.title, "My Book");
/// ```
pub fn extract_front_matter(content: &str) -> Option<(FrontMatter, usize)> {
    // Check if content starts with front matter delimiter
    if !content.starts_with("---\n") {
        return None;
    }

    // Find the closing delimiter (must start with newline)
    let after_open = &content[4..]; // Skip opening "---\n"
    let close_pos = after_open.find("\n---")?;

    // Extract YAML content (between delimiters)
    let yaml_content = &after_open[..close_pos];

    // Parse YAML
    let fm: FrontMatter = serde_yaml::from_str(yaml_content).ok()?;

    // Calculate offset: the closing delimiter starts at position close_pos + 4
    // The actual offset should be after "\n---" (4 characters)
    // So: 4 (opening) + close_pos + 4 (closing delimiter length) = 8 + close_pos
    let after_close_offset = 4 + close_pos + 4;

    // Skip any leading newlines after the closing delimiter
    let content_after_fm = &content[after_close_offset..];
    let leading_newlines = content_after_fm.chars().take_while(|c| *c == '\n').count();
    let final_offset = after_close_offset + leading_newlines;

    Some((fm, final_offset))
}

/// Convert front matter to metadata.
///
/// The cover path is resolved relative to the base directory.
pub fn front_matter_to_metadata(fm: &FrontMatter, base_dir: &Path) -> Metadata {
    let language = if fm.language.is_empty() {
        "en".to_string()
    } else {
        fm.language.clone()
    };

    // Resolve cover path relative to base_dir
    let cover_image = fm.cover.as_ref().map(|cover_path| {
        let resolved = base_dir.join(cover_path);
        // Convert to canonical path
        resolved
            .canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| cover_path.clone())
    });

    Metadata {
        title: fm.title.clone(),
        authors: fm.authors.clone(),
        language,
        identifier: fm.identifier.clone(),
        publisher: fm.publisher.clone(),
        description: fm.description.clone(),
        subjects: fm.subjects.clone(),
        date: fm.date.clone(),
        rights: fm.rights.clone(),
        cover_image,
        modified_date: fm.modified.clone(),
        contributors: fm
            .contributors
            .iter()
            .map(|c| Contributor {
                name: c.name.clone(),
                file_as: c.file_as.clone(),
                role: c.role.clone(),
            })
            .collect(),
        title_sort: fm.title_sort.clone(),
        author_sort: fm.author_sort.clone(),
        collection: fm.collection.as_ref().map(|c| CollectionInfo {
            name: c.name.clone(),
            collection_type: c.collection_type.clone(),
            position: c.position,
        }),
    }
}

/// Convert metadata to front matter.
///
/// The `cover_relative` parameter specifies the relative cover path for the output
/// file, which may differ from the absolute path stored in metadata.
pub fn metadata_to_front_matter(meta: &Metadata, cover_relative: Option<&str>) -> FrontMatter {
    let language = if meta.language.is_empty() {
        "en".to_string()
    } else {
        meta.language.clone()
    };

    FrontMatter {
        title: meta.title.clone(),
        authors: meta.authors.clone(),
        language,
        identifier: meta.identifier.clone(),
        publisher: meta.publisher.clone(),
        description: meta.description.clone(),
        subjects: meta.subjects.clone(),
        date: meta.date.clone(),
        rights: meta.rights.clone(),
        cover: cover_relative.map(|s| s.to_string()),
        modified: meta.modified_date.clone(),
        contributors: meta
            .contributors
            .iter()
            .map(|c| FrontMatterContributor {
                name: c.name.clone(),
                file_as: c.file_as.clone(),
                role: c.role.clone(),
            })
            .collect(),
        title_sort: meta.title_sort.clone(),
        author_sort: meta.author_sort.clone(),
        collection: meta.collection.as_ref().map(|c| FrontMatterCollection {
            name: c.name.clone(),
            collection_type: c.collection_type.clone(),
            position: c.position,
        }),
    }
}

/// Serialize front matter to YAML with delimiters.
///
/// Returns `None` if all fields are empty or default values.
///
/// # Example
///
/// ```rust
/// use boko::import::front_matter::{FrontMatter, serialize_front_matter};
///
/// let fm = FrontMatter {
///     title: "My Book".to_string(),
///     authors: vec!["John Doe".to_string()],
///     ..Default::default()
/// };
///
/// let yaml = serialize_front_matter(&fm).unwrap();
/// assert!(yaml.starts_with("---\n"));
/// assert!(yaml.ends_with("---\n\n"));
/// ```
pub fn serialize_front_matter(fm: &FrontMatter) -> Option<String> {
    // Check if front matter is essentially empty
    // Note: Empty language is equivalent to default "en"
    let is_empty = fm.title.is_empty()
        && fm.authors.is_empty()
        && (fm.language.is_empty() || fm.language == "en")
        && fm.identifier.is_empty()
        && fm.publisher.is_none()
        && fm.description.is_none()
        && fm.subjects.is_empty()
        && fm.date.is_none()
        && fm.rights.is_none()
        && fm.cover.is_none()
        && fm.modified.is_none()
        && fm.contributors.is_empty()
        && fm.title_sort.is_none()
        && fm.author_sort.is_none()
        && fm.collection.is_none();

    if is_empty {
        return None;
    }

    let yaml = serde_yaml::to_string(fm).ok()?;
    Some(format!("---\n{}\n---\n\n", yaml))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_front_matter_basic() {
        let content = r#"---
title: My Book
authors:
  - Alice
  - Bob
language: en
---

# Chapter 1
"#;

        let (fm, offset) = extract_front_matter(content).unwrap();
        assert_eq!(fm.title, "My Book");
        assert_eq!(fm.authors, vec!["Alice", "Bob"]);
        assert_eq!(fm.language, "en");
        assert_eq!(offset, 64);
        assert_eq!(&content[offset..], "# Chapter 1\n");
    }

    #[test]
    fn test_extract_front_matter_no_front_matter() {
        let content = "# Chapter 1\nNo front matter here.";
        assert!(extract_front_matter(content).is_none());
    }

    #[test]
    fn test_extract_front_matter_cover_field() {
        let content = r#"---
title: My Book
cover: images/cover.jpg
---

Content here.
"#;

        let (fm, offset) = extract_front_matter(content).unwrap();
        assert_eq!(fm.title, "My Book");
        assert_eq!(fm.cover, Some("images/cover.jpg".to_string()));
        assert_eq!(offset, 48);
        assert_eq!(&content[offset..], "Content here.\n");
    }

    #[test]
    fn test_roundtrip_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        // Create original metadata
        let original = Metadata {
            title: "Test Book".to_string(),
            authors: vec!["Author One".to_string()],
            language: "en".to_string(),
            identifier: "test-id".to_string(),
            publisher: Some("Test Publisher".to_string()),
            description: Some("A test book".to_string()),
            subjects: vec!["Fiction".to_string()],
            date: Some("2024-01-01".to_string()),
            rights: Some("Copyright 2024".to_string()),
            cover_image: Some(base_dir.join("cover.jpg").to_string_lossy().to_string()),
            modified_date: Some("2024-01-02".to_string()),
            contributors: vec![Contributor {
                name: "Editor One".to_string(),
                file_as: Some("One, Editor".to_string()),
                role: Some("edt".to_string()),
            }],
            title_sort: Some("Test Book, The".to_string()),
            author_sort: Some("One, Author".to_string()),
            collection: Some(CollectionInfo {
                name: "Test Series".to_string(),
                collection_type: Some("series".to_string()),
                position: Some(1.0),
            }),
        };

        // Convert to front matter
        let fm = metadata_to_front_matter(&original, Some("cover.jpg"));

        // Convert back to metadata
        let roundtrip = front_matter_to_metadata(&fm, base_dir);

        assert_eq!(roundtrip.title, original.title);
        assert_eq!(roundtrip.authors, original.authors);
        assert_eq!(roundtrip.language, original.language);
        assert_eq!(roundtrip.identifier, original.identifier);
        assert_eq!(roundtrip.publisher, original.publisher);
        assert_eq!(roundtrip.description, original.description);
        assert_eq!(roundtrip.subjects, original.subjects);
        assert_eq!(roundtrip.date, original.date);
        assert_eq!(roundtrip.rights, original.rights);
        // Cover path should be resolved to absolute path
        assert!(roundtrip.cover_image.is_some());
        assert!(roundtrip
            .cover_image
            .unwrap()
            .ends_with("cover.jpg"));
        assert_eq!(roundtrip.modified_date, original.modified_date);
        assert_eq!(roundtrip.contributors.len(), original.contributors.len());
        assert_eq!(roundtrip.contributors[0].name, original.contributors[0].name);
        assert_eq!(
            roundtrip.contributors[0].file_as,
            original.contributors[0].file_as
        );
        assert_eq!(roundtrip.contributors[0].role, original.contributors[0].role);
        assert_eq!(roundtrip.title_sort, original.title_sort);
        assert_eq!(roundtrip.author_sort, original.author_sort);
        assert_eq!(roundtrip.collection.as_ref().map(|c| &c.name), original.collection.as_ref().map(|c| &c.name));
        assert_eq!(roundtrip.collection.as_ref().map(|c| &c.collection_type), original.collection.as_ref().map(|c| &c.collection_type));
        assert_eq!(roundtrip.collection.as_ref().map(|c| &c.position), original.collection.as_ref().map(|c| &c.position));
    }

    #[test]
    fn test_serialize_empty_front_matter() {
        let fm = FrontMatter::default();
        assert!(serialize_front_matter(&fm).is_none());
    }

    #[test]
    fn test_serialize_minimal_front_matter() {
        let fm = FrontMatter {
            title: "My Book".to_string(),
            ..Default::default()
        };

        let yaml = serialize_front_matter(&fm).unwrap();
        assert!(yaml.starts_with("---\n"));
        assert!(yaml.ends_with("---\n\n"));
        assert!(yaml.contains("title: My Book"));
    }

    #[test]
    fn test_extract_front_matter_collection() {
        let content = r#"---
title: Book in Series
collection:
  name: My Series
  type: series
  position: 3.0
---

Content here.
"#;

        let (fm, offset) = extract_front_matter(content).unwrap();
        assert_eq!(fm.title, "Book in Series");
        assert!(fm.collection.is_some());
        let collection = fm.collection.unwrap();
        assert_eq!(collection.name, "My Series");
        assert_eq!(collection.collection_type, Some("series".to_string()));
        assert_eq!(collection.position, Some(3.0));
        assert_eq!(offset, 92);
        assert_eq!(&content[offset..], "Content here.\n");
    }

    #[test]
    fn test_extract_front_matter_contributors() {
        let content = r#"---
title: Edited Book
contributors:
  - name: Editor One
    role: edt
    file-as: One, Editor
  - name: Translator Two
    role: trl
---

Content here.
"#;

        let (fm, offset) = extract_front_matter(content).unwrap();
        assert_eq!(fm.title, "Edited Book");
        assert_eq!(fm.contributors.len(), 2);
        assert_eq!(fm.contributors[0].name, "Editor One");
        assert_eq!(fm.contributors[0].role, Some("edt".to_string()));
        assert_eq!(fm.contributors[0].file_as, Some("One, Editor".to_string()));
        assert_eq!(fm.contributors[1].name, "Translator Two");
        assert_eq!(fm.contributors[1].role, Some("trl".to_string()));
        assert!(fm.contributors[1].file_as.is_none());
        assert_eq!(offset, 141);
        assert_eq!(&content[offset..], "Content here.\n");
    }

    #[test]
    fn test_extract_front_matter_with_extra_newlines() {
        let content = r#"---
title: My Book
---


# Chapter 1
"#;

        let (fm, offset) = extract_front_matter(content).unwrap();
        assert_eq!(fm.title, "My Book");
        // Offset should skip extra newlines after closing delimiter
        assert_eq!(offset, 25);
        assert_eq!(&content[offset..], "# Chapter 1\n");
    }

    #[test]
    fn test_front_matter_to_metadata_default_language() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        let fm = FrontMatter {
            title: "Test".to_string(),
            language: String::new(), // Empty language
            ..Default::default()
        };

        let meta = front_matter_to_metadata(&fm, base_dir);
        assert_eq!(meta.language, "en");
    }

    #[test]
    fn test_metadata_to_front_matter_default_language() {
        let meta = Metadata {
            title: "Test".to_string(),
            language: String::new(),
            ..Default::default()
        };

        let fm = metadata_to_front_matter(&meta, None);
        assert_eq!(fm.language, "en");
    }

    #[test]
    fn test_serialize_omits_empty_vectors() {
        let fm = FrontMatter {
            title: "Test".to_string(),
            authors: Vec::new(),
            subjects: Vec::new(),
            contributors: Vec::new(),
            ..Default::default()
        };

        let yaml = serialize_front_matter(&fm).unwrap();
        // Empty vectors should not appear in serialized output
        assert!(!yaml.contains("authors"));
        assert!(!yaml.contains("subjects"));
        assert!(!yaml.contains("contributors"));
    }

    #[test]
    fn test_extract_front_matter_without_trailing_newline() {
        let content = r#"---
title: My Book
---
Content here."#;

        let (fm, offset) = extract_front_matter(content).unwrap();
        assert_eq!(fm.title, "My Book");
        assert_eq!(&content[offset..], "Content here.");
    }

    #[test]
    fn test_front_matter_all_fields() {
        let content = r#"---
title: Complete Book
authors:
  - Author One
  - Author Two
language: fr
identifier: isbn-123
publisher: Big Publisher
description: A very complete book
subjects:
  - Fiction
  - Adventure
date: 2024-01-01
rights: Copyright 2024
cover: images/cover.png
modified: 2024-01-15
contributors:
  - name: Editor One
    role: edt
title-sort: Complete, The Book
author-sort: One, Author
collection:
  name: Great Series
  type: series
  position: 5.0
---

Book content.
"#;

        let (fm, offset) = extract_front_matter(content).unwrap();
        assert_eq!(fm.title, "Complete Book");
        assert_eq!(fm.authors.len(), 2);
        assert_eq!(fm.language, "fr");
        assert_eq!(fm.identifier, "isbn-123");
        assert_eq!(fm.publisher, Some("Big Publisher".to_string()));
        assert_eq!(fm.description, Some("A very complete book".to_string()));
        assert_eq!(fm.subjects.len(), 2);
        assert_eq!(fm.date, Some("2024-01-01".to_string()));
        assert_eq!(fm.rights, Some("Copyright 2024".to_string()));
        assert_eq!(fm.cover, Some("images/cover.png".to_string()));
        assert_eq!(fm.modified, Some("2024-01-15".to_string()));
        assert_eq!(fm.contributors.len(), 1);
        assert_eq!(fm.title_sort, Some("Complete, The Book".to_string()));
        assert_eq!(fm.author_sort, Some("One, Author".to_string()));
        assert!(fm.collection.is_some());
        assert_eq!(offset, 452);
        assert_eq!(&content[offset..], "Book content.\n");
    }
}
