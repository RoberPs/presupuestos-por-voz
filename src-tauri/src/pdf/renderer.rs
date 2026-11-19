//! Dibujo del documento maquetado en PDF/A-2b (research.md, R1 y R2).
//!
//! Solo dibuja el resultado de `domain/quote_doc/layout.rs`. Los registros técnicos no incluyen
//! ningún dato del documento (FR-028b).

use krilla::color::rgb;
use krilla::configure::{Archival, ConfigurationBuilder};
use krilla::error::KrillaError;
use krilla::geom::{PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image;
use krilla::metadata::{DateTime, Metadata};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};
use krilla::{Document, SerializeSettings};
use time::Date;

use crate::domain::quote_doc::layout::{
    layout_document, Element, FontWeight, LogoBox, RuleItem, TextItem, PAGE_HEIGHT, PAGE_WIDTH,
};
use crate::domain::quote_doc::QuoteDocument;
use crate::error::AppError;
use crate::pdf::fonts::{PdfFonts, BOLD, REGULAR};
use crate::pdf::logo::NormalizedLogo;

const DOCUMENT_LANGUAGE: &str = "es-ES";
const RULE_GRAY: u8 = 150;

/// PDF ya compuesto, listo para guardar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPdf {
    pub bytes: Vec<u8>,
    pub page_count: usize,
}

fn render_failed(reason: &str) -> AppError {
    log::error!("No se ha podido componer el PDF: {reason}");
    AppError::pdf_render()
}

fn describe(error: &KrillaError) -> &'static str {
    match error {
        KrillaError::Font(..) => "tipografía",
        KrillaError::Validation(_) => "validación PDF/A",
        KrillaError::Limit(_) => "límite del formato",
        _ => "error interno",
    }
}

fn fill(gray: u8) -> Fill {
    Fill {
        paint: rgb::Color::new(gray, gray, gray).into(),
        opacity: NormalizedF32::ONE,
        rule: FillRule::default(),
    }
}

fn draw_text(surface: &mut Surface<'_>, item: &TextItem, regular: &Font, bold: &Font) {
    let font = match item.weight {
        FontWeight::Regular => regular,
        FontWeight::Bold => bold,
    };
    surface.draw_text(
        Point::from_xy(item.x, item.y),
        font.clone(),
        item.size,
        &item.text,
        false,
        TextDirection::Auto,
    );
}

fn draw_rule(surface: &mut Surface<'_>, rule: &RuleItem) {
    let Some(rect) = Rect::from_xywh(rule.x, rule.y, rule.width, rule.thickness) else {
        return;
    };
    let mut builder = PathBuilder::new();
    builder.push_rect(rect);
    if let Some(path) = builder.finish() {
        surface.set_fill(Some(fill(RULE_GRAY)));
        surface.draw_path(&path);
        surface.set_fill(Some(fill(0)));
    }
}

fn draw_logo(surface: &mut Surface<'_>, logo: &LogoBox, image: &Image) {
    let Some(size) = Size::from_wh(logo.width, logo.height) else {
        return;
    };
    surface.push_transform(&Transform::from_translate(logo.x, logo.y));
    surface.draw_image(image.clone(), size);
    surface.pop();
}

/// Maqueta y dibuja el documento. Si no cumple PDF/A-2b, falla en lugar de dar un archivo dudoso.
pub fn render_document(
    document: &QuoteDocument,
    issue_date: Date,
    logo: Option<&NormalizedLogo>,
) -> Result<RenderedPdf, AppError> {
    let measurer = PdfFonts::load().ok_or_else(|| render_failed("tipografía"))?;
    let layout = layout_document(document, &measurer);

    let regular =
        Font::new(REGULAR.into(), 0).ok_or_else(|| render_failed("tipografía regular"))?;
    let bold = Font::new(BOLD.into(), 0).ok_or_else(|| render_failed("tipografía negrita"))?;
    // La interpolación está desactivada porque PDF/A no la admite.
    let image = logo
        .map(|logo| Image::from_png(logo.png.clone().into(), false))
        .transpose()
        .map_err(|_| render_failed("logo"))?;

    let configuration = ConfigurationBuilder::new()
        .with_archival_validator(Archival::A2_B)
        .finish()
        .map_err(|_| render_failed("configuración PDF/A"))?;
    let mut pdf = Document::new_with(SerializeSettings {
        configuration,
        ..SerializeSettings::default()
    });
    pdf.set_metadata(
        Metadata::new()
            .title(format!("Presupuesto {}", document.number))
            .language(DOCUMENT_LANGUAGE.to_string())
            .creation_date(
                DateTime::new(issue_date.year() as u16)
                    .month(u8::from(issue_date.month()))
                    .day(issue_date.day()),
            ),
    );

    let settings =
        PageSettings::from_wh(PAGE_WIDTH, PAGE_HEIGHT).ok_or_else(|| render_failed("página"))?;
    for page in &layout.pages {
        let mut pdf_page = pdf.start_page_with(settings.clone());
        let mut surface = pdf_page.surface();
        surface.set_fill(Some(fill(0)));
        for element in &page.elements {
            match element {
                Element::Text(item) => draw_text(&mut surface, item, &regular, &bold),
                Element::Rule(rule) => draw_rule(&mut surface, rule),
                Element::Logo(logo) => {
                    if let Some(image) = &image {
                        draw_logo(&mut surface, logo, image);
                    }
                }
            }
        }
        surface.finish();
        pdf_page.finish();
    }

    let bytes = pdf
        .finish()
        .map_err(|error| render_failed(describe(&error)))?;
    Ok(RenderedPdf {
        bytes,
        page_count: layout.pages.len(),
    })
}
