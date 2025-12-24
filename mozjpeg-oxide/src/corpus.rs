//! Utilities for locating test corpus images.
//!
//! This module provides cross-platform utilities for finding test images,
//! whether from the local corpus directory, environment variables, or
//! the bundled test images.

use std::path::{Path, PathBuf};

/// Returns the path to the corpus directory.
///
/// Checks in order:
/// 1. `MOZJPEG_CORPUS_DIR` environment variable
/// 2. `CODEC_CORPUS_DIR` environment variable
/// 3. `./corpus/` relative to project root
/// 4. `../codec-corpus/` sibling directory (legacy)
///
/// Returns `None` if no corpus directory is found.
pub fn corpus_dir() -> Option<PathBuf> {
    // Check environment variables first
    if let Ok(dir) = std::env::var("MOZJPEG_CORPUS_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return Some(path);
        }
    }

    if let Ok(dir) = std::env::var("CODEC_CORPUS_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return Some(path);
        }
    }

    // Check ./corpus/ relative to project root
    let project_root = project_root()?;
    let corpus = project_root.join("corpus");
    if corpus.is_dir() {
        return Some(corpus);
    }

    // Legacy: check sibling codec-corpus directory
    let sibling = project_root.parent()?.join("codec-corpus");
    if sibling.is_dir() {
        return Some(sibling);
    }

    None
}

/// Returns the path to the Kodak test images.
pub fn kodak_dir() -> Option<PathBuf> {
    let corpus = corpus_dir()?;
    let kodak = corpus.join("kodak");
    if kodak.is_dir() {
        Some(kodak)
    } else {
        None
    }
}

/// Returns the path to the CLIC validation images.
pub fn clic_validation_dir() -> Option<PathBuf> {
    let corpus = corpus_dir()?;

    // Try clic2025/validation first
    let clic = corpus.join("clic2025").join("validation");
    if clic.is_dir() {
        return Some(clic);
    }

    // Try just clic2025
    let clic = corpus.join("clic2025");
    if clic.is_dir() {
        return Some(clic);
    }

    None
}

/// Returns paths to all available corpus directories.
pub fn all_corpus_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(kodak) = kodak_dir() {
        dirs.push(kodak);
    }

    if let Some(clic) = clic_validation_dir() {
        dirs.push(clic);
    }

    dirs
}

/// Returns the path to the bundled test images (always available).
///
/// These are small test images bundled with the crate for CI.
pub fn bundled_test_images_dir() -> Option<PathBuf> {
    let project_root = project_root()?;
    let test_images = project_root.join("mozjpeg-oxide").join("tests").join("images");
    if test_images.is_dir() {
        Some(test_images)
    } else {
        None
    }
}

/// Returns a specific bundled test image path.
pub fn bundled_test_image(name: &str) -> Option<PathBuf> {
    let dir = bundled_test_images_dir()?;
    let path = dir.join(name);
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

/// Returns the project root directory.
fn project_root() -> Option<PathBuf> {
    // Try CARGO_MANIFEST_DIR first (works in tests/examples)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        // If we're in mozjpeg-oxide/, go up one level
        if manifest_path.file_name()?.to_str()? == "mozjpeg-oxide" {
            return manifest_path.parent().map(|p| p.to_path_buf());
        }
        return Some(manifest_path);
    }

    // Fall back to current directory
    std::env::current_dir().ok()
}

/// Returns PNG files from a directory, sorted by name.
pub fn png_files_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

/// Loads an image from a path, returning RGB data, width, and height.
///
/// Supports PNG files. Returns `None` on error or unsupported format.
#[cfg(feature = "png")]
pub fn load_png_as_rgb(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
    use std::fs::File;

    let file = File::open(path).ok()?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let bytes = &buf[..info.buffer_size()];

    let width = info.width;
    let height = info.height;

    let rgb_data = match info.color_type {
        png::ColorType::Rgb => bytes.to_vec(),
        png::ColorType::Rgba => bytes
            .chunks(4)
            .flat_map(|c| [c[0], c[1], c[2]])
            .collect(),
        png::ColorType::Grayscale => bytes.iter().flat_map(|&g| [g, g, g]).collect(),
        png::ColorType::GrayscaleAlpha => bytes
            .chunks(2)
            .flat_map(|c| [c[0], c[0], c[0]])
            .collect(),
        _ => return None,
    };

    Some((rgb_data, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_images_available() {
        let dir = bundled_test_images_dir();
        assert!(dir.is_some(), "Bundled test images directory should exist");

        let image = bundled_test_image("1.png");
        assert!(image.is_some(), "Bundled 1.png should exist");
    }

    #[test]
    fn test_project_root() {
        let root = project_root();
        assert!(root.is_some(), "Should find project root");

        let root = root.unwrap();
        assert!(
            root.join("Cargo.toml").is_file(),
            "Project root should contain Cargo.toml"
        );
    }
}
