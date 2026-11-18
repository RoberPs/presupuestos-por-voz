//! Normalización del logo antes de incrustarlo en el PDF (research.md, R6).

use std::io::Cursor;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageError, ImageFormat};

/// Lado mayor del logo incrustado, en píxeles.
const MAX_SIDE_PX: u32 = 640;

/// Logo reescrito como PNG RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedLogo {
    pub png: Vec<u8>,
    pub width_px: u32,
    pub height_px: u32,
}

/// Decodifica un PNG o JPG, lo reduce si hace falta (nunca lo amplía) y conserva la transparencia.
pub fn normalize(bytes: &[u8]) -> Result<NormalizedLogo, ImageError> {
    let decoded = image::load_from_memory(bytes)?;
    let (width, height) = decoded.dimensions();
    let resized = if width.max(height) > MAX_SIDE_PX {
        decoded.resize(MAX_SIDE_PX, MAX_SIDE_PX, FilterType::Lanczos3)
    } else {
        decoded
    };

    let rgba = DynamicImage::ImageRgba8(resized.to_rgba8());
    let mut png = Cursor::new(Vec::new());
    rgba.write_to(&mut png, ImageFormat::Png)?;
    Ok(NormalizedLogo {
        png: png.into_inner(),
        width_px: rgba.width(),
        height_px: rgba.height(),
    })
}
