//! Estructura del PDF generado (FR-006, FR-015 y FR-016).

use std::io::Cursor;

use app_lib::domain::quote_doc::{
    DocumentBusiness, DocumentClient, DocumentLine, DocumentTotals, LogoSize, QuoteDocument,
};
use app_lib::pdf::logo::{normalize, NormalizedLogo};
use app_lib::pdf::renderer::{render_document, RenderedPdf};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb, Rgba};
use lopdf::{Document, Object};
use time::{Date, Month};

fn issue_date() -> Date {
    Date::from_calendar_date(2026, Month::November, 18).unwrap()
}

fn document(line_count: usize, logo: Option<LogoSize>) -> QuoteDocument {
    QuoteDocument {
        number: "2026-001".into(),
        issue_date: "18/11/2026".into(),
        valid_until: "18/12/2026".into(),
        business: DocumentBusiness {
            trade_name: "Fontanería Núñez".into(),
            legal_name: None,
            tax_id: "12345678Z".into(),
            phone: "600 123 456".into(),
            address: "Calle Mayor 1, 28001 Madrid (Madrid)".into(),
            email: None,
            website: None,
        },
        client: DocumentClient {
            name: "Ana Ruiz".into(),
            ..DocumentClient::default()
        },
        lines: (0..line_count)
            .map(|i| DocumentLine {
                concept: format!("Instalación de cañería número {i} ¿sí? ¡ya!"),
                quantity: "1,5".into(),
                unit: "metro".into(),
                price: "25,00 €".into(),
                amount: "37,50 €".into(),
            })
            .collect(),
        totals: DocumentTotals {
            base: "135,00 €".into(),
            vat_label: "IVA (21 %)".into(),
            vat: "28,35 €".into(),
            total: "163,35 €".into(),
        },
        logo,
    }
}

fn render(doc: &QuoteDocument, logo: Option<&NormalizedLogo>) -> RenderedPdf {
    render_document(doc, issue_date(), logo).unwrap()
}

fn parsed(pdf: &RenderedPdf) -> Document {
    Document::load_mem(&pdf.bytes).unwrap()
}

fn dictionaries(doc: &Document) -> Vec<&lopdf::Dictionary> {
    doc.objects
        .values()
        .filter_map(|object| match object {
            Object::Dictionary(dict) => Some(dict),
            Object::Stream(stream) => Some(&stream.dict),
            _ => None,
        })
        .collect()
}

fn images(doc: &Document) -> Vec<&lopdf::Dictionary> {
    dictionaries(doc)
        .into_iter()
        .filter(|dict| {
            dict.get(b"Subtype")
                .and_then(Object::as_name)
                .is_ok_and(|name| name == b"Image")
        })
        .collect()
}

fn encode(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

fn transparent_png() -> Vec<u8> {
    let image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(64, 32, |x, _| {
        Rgba([200, 30, 30, if x < 32 { 0 } else { 255 }])
    });
    encode(DynamicImage::ImageRgba8(image), ImageFormat::Png)
}

fn jpg() -> Vec<u8> {
    let image: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(64, 32, |x, y| Rgb([(x * 3) as u8, (y * 5) as u8, 120]));
    encode(DynamicImage::ImageRgb8(image), ImageFormat::Jpeg)
}

fn logo_size(logo: &NormalizedLogo) -> LogoSize {
    LogoSize {
        width_px: logo.width_px,
        height_px: logo.height_px,
    }
}

#[test]
fn pages_are_a4_portrait() {
    let pdf = render(&document(2, None), None);
    let parsed = parsed(&pdf);
    for page_id in parsed.get_pages().into_values() {
        let page = parsed.get_dictionary(page_id).unwrap();
        let media_box = page.get(b"MediaBox").unwrap().as_array().unwrap();
        let value = |i: usize| media_box[i].as_float().unwrap();
        assert!((value(2) - value(0) - 595.28).abs() < 0.01);
        assert!((value(3) - value(1) - 841.89).abs() < 0.01);
    }
}

#[test]
fn fonts_are_embedded() {
    let parsed = parsed(&render(&document(2, None), None));
    let descriptors: Vec<_> = dictionaries(&parsed)
        .into_iter()
        .filter(|dict| {
            dict.get(b"Type")
                .and_then(Object::as_name)
                .is_ok_and(|name| name == b"FontDescriptor")
        })
        .collect();
    assert_eq!(descriptors.len(), 2, "Regular y Bold");
    for descriptor in descriptors {
        assert!(
            descriptor.has(b"FontFile2") || descriptor.has(b"FontFile3"),
            "tipografía sin incrustar"
        );
    }
}

#[test]
fn the_document_declares_pdf_a_2b() {
    let pdf = render(&document(2, None), None);
    let text = String::from_utf8_lossy(&pdf.bytes);
    assert!(text.contains("pdfaid:part"), "sin marca PDF/A");
    assert!(
        text.contains("<pdfaid:part>2</pdfaid:part>") || text.contains("pdfaid:part=\"2\""),
        "no es la parte 2"
    );
    assert!(
        text.contains("<pdfaid:conformance>B</pdfaid:conformance>")
            || text.contains("pdfaid:conformance=\"B\""),
        "no es el nivel B"
    );
}

#[test]
fn the_title_and_the_language_are_declared() {
    let pdf = render(&document(2, None), None);
    let text = String::from_utf8_lossy(&pdf.bytes);
    assert!(text.contains("Presupuesto 2026-001"), "sin título");
    assert!(text.contains("es-ES"), "sin idioma");
}

#[test]
fn the_page_count_matches_the_file() {
    for lines in [2, 120] {
        let pdf = render(&document(lines, None), None);
        assert_eq!(parsed(&pdf).get_pages().len(), pdf.page_count);
    }
    assert!(render(&document(120, None), None).page_count > 1);
}

#[test]
fn a_png_keeps_its_transparency() {
    let logo = normalize(&transparent_png()).unwrap();
    let doc = document(2, Some(logo_size(&logo)));
    let parsed = parsed(&render(&doc, Some(&logo)));
    assert!(
        images(&parsed).iter().any(|image| image.has(b"SMask")),
        "el logo ha perdido su máscara"
    );
}

#[test]
fn a_jpg_is_embedded() {
    let logo = normalize(&jpg()).unwrap();
    let doc = document(2, Some(logo_size(&logo)));
    let parsed = parsed(&render(&doc, Some(&logo)));
    assert!(!images(&parsed).is_empty(), "falta el logo");
}

#[test]
fn a_document_without_a_logo_has_no_images() {
    let parsed = parsed(&render(&document(2, None), None));
    assert!(images(&parsed).is_empty());
}

#[test]
fn images_are_not_interpolated() {
    let logo = normalize(&transparent_png()).unwrap();
    let doc = document(2, Some(logo_size(&logo)));
    let parsed = parsed(&render(&doc, Some(&logo)));
    for image in images(&parsed) {
        let interpolate = image
            .get(b"Interpolate")
            .and_then(Object::as_bool)
            .unwrap_or(false);
        assert!(!interpolate, "PDF/A no admite la interpolación");
    }
}

#[test]
fn large_logos_are_reduced_and_small_ones_are_not_enlarged() {
    let big: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(1280, 320, Rgb([1, 2, 3]));
    let logo = normalize(&encode(DynamicImage::ImageRgb8(big), ImageFormat::Png)).unwrap();
    assert_eq!((logo.width_px, logo.height_px), (640, 160));

    let small = normalize(&transparent_png()).unwrap();
    assert_eq!((small.width_px, small.height_px), (64, 32));
}

#[test]
fn an_unreadable_logo_is_rejected() {
    assert!(normalize(b"esto no es una imagen").is_err());
}
