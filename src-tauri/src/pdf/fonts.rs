//! Tipografías incluidas en el ejecutable y medida del texto (research.md, R3 y R4).

use rustybuzz::{shape, Face, UnicodeBuffer};

use crate::domain::quote_doc::layout::{FontWeight, TextMeasurer};

pub static REGULAR: &[u8] = include_bytes!("../../assets/fonts/LiberationSans-Regular.ttf");
pub static BOLD: &[u8] = include_bytes!("../../assets/fonts/LiberationSans-Bold.ttf");

/// Mide cada texto con su propia tipografía, con el mismo motor con el que se compone el PDF.
pub struct PdfFonts {
    regular: Face<'static>,
    bold: Face<'static>,
}

impl PdfFonts {
    pub fn load() -> Option<Self> {
        Some(Self {
            regular: Face::from_slice(REGULAR, 0)?,
            bold: Face::from_slice(BOLD, 0)?,
        })
    }

    /// Indica si las dos tipografías del documento pueden mostrar un carácter (FR-018b).
    pub fn supports(&self, character: char) -> bool {
        self.regular.glyph_index(character).is_some() && self.bold.glyph_index(character).is_some()
    }

    fn face(&self, weight: FontWeight) -> &Face<'static> {
        match weight {
            FontWeight::Regular => &self.regular,
            FontWeight::Bold => &self.bold,
        }
    }
}

impl TextMeasurer for PdfFonts {
    fn width(&self, text: &str, weight: FontWeight, size: f32) -> f32 {
        let face = self.face(weight);
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        let shaped = shape(face, &[], buffer);
        let advance: i32 = shaped
            .glyph_positions()
            .iter()
            .map(|position| position.x_advance)
            .sum();
        advance as f32 / face.units_per_em() as f32 * size
    }
}
