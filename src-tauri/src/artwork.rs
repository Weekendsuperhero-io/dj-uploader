//! Cover-art normalization.
//!
//! Both platforms require **square** artwork, and SoundCloud rejects images
//! smaller than 800×800. When the source cover doesn't conform (not square, too
//! small, or unreasonably large), it is center-cropped to a square and resized
//! to `TARGET`×`TARGET`, then re-encoded as JPEG. A conforming image is passed
//! through untouched so its quality is preserved.

use anyhow::{Context, Result};
use std::path::Path;

/// Output size used when converting a non-conforming cover.
const TARGET: u32 = 1024;
/// SoundCloud's minimum artwork dimension.
const MIN: u32 = 800;
/// Above this we downscale, to keep the upload a sane size.
const MAX: u32 = 2400;

pub struct Artwork {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub mime: &'static str,
}

/// Return upload-ready cover art for `path`, converting to a square
/// `TARGET`×`TARGET` JPEG only if the source doesn't already meet the
/// platforms' requirements.
pub fn prepare(path: &Path) -> Result<Artwork> {
    let original = std::fs::read(path).context("Failed to read image file")?;

    let img = image::load_from_memory(&original).context("Unsupported or corrupt image file")?;
    let (w, h) = (img.width(), img.height());

    // Already square and a sensible size → upload as-is (no re-encode).
    if w == h && (MIN..=MAX).contains(&w) {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("cover")
            .to_string();
        return Ok(Artwork {
            bytes: original,
            file_name,
            mime: mime_for(path),
        });
    }

    // Center-crop to the largest square, then resize to TARGET×TARGET.
    let side = w.min(h);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    let square = img.crop_imm(x, y, side, side).resize_exact(
        TARGET,
        TARGET,
        image::imageops::FilterType::Lanczos3,
    );

    // Encode as JPEG (drops any alpha channel).
    let rgb = square.to_rgb8();
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90)
        .encode(rgb.as_raw(), TARGET, TARGET, image::ExtendedColorType::Rgb8)
        .context("Failed to encode resized artwork")?;

    println!("  Resized artwork {w}×{h} → {TARGET}×{TARGET} (square requirement)");

    Ok(Artwork {
        bytes,
        file_name: "cover.jpg".to_string(),
        mime: "image/jpeg",
    })
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_png(name: &str, w: u32, h: u32) -> std::path::PathBuf {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 40, 40]));
        let path = std::env::temp_dir().join(name);
        image::DynamicImage::ImageRgb8(img).save(&path).unwrap();
        path
    }

    #[test]
    fn non_square_is_converted_to_target_square_jpeg() {
        let path = write_png("dj-art-nonsquare.png", 1600, 900);
        let art = prepare(&path).unwrap();
        assert_eq!(art.mime, "image/jpeg");
        assert_eq!(art.file_name, "cover.jpg");
        let decoded = image::load_from_memory(&art.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (TARGET, TARGET));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn too_small_square_is_upscaled_to_target() {
        let path = write_png("dj-art-small.png", 400, 400);
        let art = prepare(&path).unwrap();
        let decoded = image::load_from_memory(&art.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (TARGET, TARGET));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn conforming_square_passes_through_unchanged() {
        let path = write_png("dj-art-ok.png", 1000, 1000);
        let art = prepare(&path).unwrap();
        assert_eq!(art.mime, "image/png"); // untouched: original bytes + mime
        let decoded = image::load_from_memory(&art.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (1000, 1000));
        let _ = std::fs::remove_file(&path);
    }
}
