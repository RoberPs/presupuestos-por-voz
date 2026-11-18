//! Maquetación del documento con un medidor simulado (FR-007, FR-012, FR-017 a FR-019 y SC-005).

use app_lib::domain::quote_doc::layout::{
    layout_document, wrap_text, Element, FontWeight, LaidOutDocument, Page, TextItem, TextMeasurer,
    TextRole, CONTENT_RIGHT, MARGIN, PAGE_HEIGHT,
};
use app_lib::domain::quote_doc::{
    DocumentBusiness, DocumentClient, DocumentLine, DocumentTotals, LogoSize, QuoteDocument,
};

/// Cada carácter mide la mitad del tamaño de letra.
struct HalfEm;

impl TextMeasurer for HalfEm {
    fn width(&self, text: &str, _weight: FontWeight, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.5
    }
}

fn line(concept: &str) -> DocumentLine {
    DocumentLine {
        concept: concept.into(),
        quantity: "3".into(),
        unit: "metro".into(),
        price: "25,00 €".into(),
        amount: "75,00 €".into(),
    }
}

fn document(lines: Vec<DocumentLine>) -> QuoteDocument {
    QuoteDocument {
        number: "2026-001".into(),
        issue_date: "18/11/2026".into(),
        valid_until: "18/12/2026".into(),
        business: DocumentBusiness {
            trade_name: "Fontanería López".into(),
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
        lines,
        totals: DocumentTotals {
            base: "135,00 €".into(),
            vat_label: "IVA (21 %)".into(),
            vat: "28,35 €".into(),
            total: "163,35 €".into(),
        },
        logo: None,
    }
}

fn lines(n: usize) -> Vec<DocumentLine> {
    (0..n).map(|i| line(&format!("Concepto {i}"))).collect()
}

fn laid_out(doc: &QuoteDocument) -> LaidOutDocument {
    layout_document(doc, &HalfEm)
}

fn texts(page: &Page) -> Vec<&TextItem> {
    page.elements
        .iter()
        .filter_map(|element| match element {
            Element::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

fn with_role(layout: &LaidOutDocument, role: TextRole) -> Vec<String> {
    layout
        .pages
        .iter()
        .flat_map(texts)
        .filter(|text| text.role == role)
        .map(|text| text.text.clone())
        .collect()
}

fn all_text(layout: &LaidOutDocument) -> String {
    layout
        .pages
        .iter()
        .flat_map(texts)
        .map(|text| text.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn pages_of_line(layout: &LaidOutDocument, index: usize) -> Vec<usize> {
    layout
        .pages
        .iter()
        .enumerate()
        .filter(|(_, page)| {
            texts(page).iter().any(|text| {
                text.role == (TextRole::Concept { line: index })
                    || text.role == (TextRole::Cell { line: index })
            })
        })
        .map(|(number, _)| number)
        .collect()
}

#[test]
fn the_header_shows_the_required_business_data() {
    let text = all_text(&laid_out(&document(lines(1))));
    for expected in [
        "Fontanería López",
        "12345678Z",
        "600 123 456",
        "Calle Mayor 1",
    ] {
        assert!(text.contains(expected), "falta {expected}");
    }
}

#[test]
fn the_header_shows_the_optional_business_data_when_present() {
    let mut doc = document(lines(1));
    doc.business.legal_name = Some("José López García".into());
    doc.business.email = Some("jose@lopez.es".into());
    doc.business.website = Some("www.lopez.es".into());
    let text = all_text(&laid_out(&doc));
    for expected in ["José López García", "jose@lopez.es", "www.lopez.es"] {
        assert!(text.contains(expected), "falta {expected}");
    }
}

#[test]
fn the_header_shows_the_number_and_the_dates() {
    let text = all_text(&laid_out(&document(lines(1))));
    for expected in ["PRESUPUESTO", "2026-001", "18/11/2026", "18/12/2026"] {
        assert!(text.contains(expected), "falta {expected}");
    }
}

#[test]
fn text_wraps_at_spaces() {
    assert_eq!(
        wrap_text("uno dos tres", 40.0, FontWeight::Regular, 10.0, &HalfEm),
        ["uno dos", "tres"]
    );
}

#[test]
fn a_word_that_does_not_fit_is_split_by_characters() {
    assert_eq!(
        wrap_text("abcdefghij", 20.0, FontWeight::Regular, 10.0, &HalfEm),
        ["abcd", "efgh", "ij"]
    );
}

#[test]
fn a_long_word_starts_on_its_own_line() {
    assert_eq!(
        wrap_text("uno abcdefghij", 40.0, FontWeight::Regular, 10.0, &HalfEm),
        ["uno", "abcdefgh", "ij"]
    );
}

#[test]
fn wrapping_always_advances_even_if_nothing_fits() {
    assert_eq!(
        wrap_text("abc", 1.0, FontWeight::Regular, 10.0, &HalfEm),
        ["a", "b", "c"]
    );
}

#[test]
fn an_empty_text_has_no_lines() {
    assert!(wrap_text("", 40.0, FontWeight::Regular, 10.0, &HalfEm).is_empty());
}

#[test]
fn no_row_is_split_between_pages() {
    let long = "palabra ".repeat(20);
    let doc = document((0..120).map(|_| line(long.trim())).collect());
    let layout = laid_out(&doc);
    assert!(layout.pages.len() > 1);
    for index in 0..doc.lines.len() {
        assert_eq!(pages_of_line(&layout, index).len(), 1, "línea {index}");
    }
}

#[test]
fn the_table_header_is_repeated_on_every_page_with_lines() {
    let layout = laid_out(&document(lines(120)));
    assert!(layout.pages.len() > 1);
    for (number, page) in layout.pages.iter().enumerate() {
        let items = texts(page);
        let has_lines = items
            .iter()
            .any(|text| matches!(text.role, TextRole::Concept { .. }));
        let headers = items
            .iter()
            .filter(|text| text.role == TextRole::TableHeader && text.text == "Concepto")
            .count();
        assert_eq!(headers, usize::from(has_lines), "página {number}");
    }
}

#[test]
fn following_pages_identify_the_quote() {
    let layout = laid_out(&document(lines(120)));
    let headers = with_role(&layout, TextRole::ContinuationHeader);
    assert_eq!(headers.len(), layout.pages.len() - 1);
    assert!(headers[0].contains("Presupuesto 2026-001"));
    assert!(headers[0].contains("Fontanería López"));
}

#[test]
fn totals_appear_once_and_together_at_any_length() {
    for n in 1..=90 {
        let layout = laid_out(&document(lines(n)));
        let pages: Vec<usize> = layout
            .pages
            .iter()
            .enumerate()
            .filter(|(_, page)| {
                texts(page)
                    .iter()
                    .any(|text| text.role == TextRole::TotalsValue)
            })
            .map(|(number, _)| number)
            .collect();
        assert_eq!(pages, [layout.pages.len() - 1], "{n} líneas");
        assert_eq!(
            with_role(&layout, TextRole::TotalsValue),
            ["135,00 €", "28,35 €", "163,35 €"],
            "{n} líneas"
        );
    }
}

#[test]
fn the_totals_come_after_the_last_line() {
    let layout = laid_out(&document(lines(3)));
    let items = texts(&layout.pages[0]);
    let last_line = items
        .iter()
        .filter(|text| matches!(text.role, TextRole::Concept { .. }))
        .map(|text| text.y)
        .fold(f32::MIN, f32::max);
    let first_total = items
        .iter()
        .filter(|text| text.role == TextRole::TotalsValue)
        .map(|text| text.y)
        .fold(f32::MAX, f32::min);
    assert!(first_total > last_line);
}

#[test]
fn every_page_is_numbered() {
    let layout = laid_out(&document(lines(120)));
    let total = layout.pages.len();
    let expected: Vec<String> = (1..=total)
        .map(|n| format!("Página {n} de {total}"))
        .collect();
    assert_eq!(with_role(&layout, TextRole::PageNumber), expected);
}

#[test]
fn a_single_page_is_numbered_too() {
    let layout = laid_out(&document(lines(1)));
    assert_eq!(with_role(&layout, TextRole::PageNumber), ["Página 1 de 1"]);
}

fn logo_box(layout: &LaidOutDocument) -> Option<(f32, f32, f32, f32)> {
    layout.pages[0]
        .elements
        .iter()
        .find_map(|element| match element {
            Element::Logo(logo) => Some((logo.x, logo.y, logo.width, logo.height)),
            _ => None,
        })
}

fn business_x(layout: &LaidOutDocument) -> f32 {
    texts(&layout.pages[0])
        .iter()
        .find(|text| text.role == TextRole::BusinessName)
        .unwrap()
        .x
}

#[test]
fn without_a_logo_no_gap_is_left() {
    let layout = laid_out(&document(lines(1)));
    assert!(logo_box(&layout).is_none());
    assert_eq!(business_x(&layout), MARGIN);
}

#[test]
fn the_business_data_never_overlaps_the_logo() {
    let mut doc = document(lines(1));
    doc.logo = Some(LogoSize {
        width_px: 800,
        height_px: 400,
    });
    doc.business.address = "Dirección muy larga ".repeat(12);
    let layout = laid_out(&doc);
    let (x, _, width, _) = logo_box(&layout).unwrap();
    for text in texts(&layout.pages[0])
        .iter()
        .filter(|text| matches!(text.role, TextRole::BusinessName | TextRole::Business))
    {
        assert!(text.x >= x + width, "«{}» invade el logo", text.text);
    }
}

#[test]
fn the_logo_fits_its_box_and_keeps_its_proportions() {
    let mut doc = document(lines(1));
    doc.logo = Some(LogoSize {
        width_px: 1000,
        height_px: 200,
    });
    let (_, _, width, height) = logo_box(&laid_out(&doc)).unwrap();
    assert!((width - 150.0).abs() < 0.01);
    assert!((height - 30.0).abs() < 0.01);

    doc.logo = Some(LogoSize {
        width_px: 200,
        height_px: 1000,
    });
    let (_, _, width, height) = logo_box(&laid_out(&doc)).unwrap();
    assert!((height - 70.0).abs() < 0.01);
    assert!((width - 14.0).abs() < 0.01);
}

#[test]
fn a_small_logo_is_never_enlarged() {
    let mut doc = document(lines(1));
    doc.logo = Some(LogoSize {
        width_px: 40,
        height_px: 40,
    });
    let (_, _, width, height) = logo_box(&laid_out(&doc)).unwrap();
    assert!((width - 30.0).abs() < 0.01, "40 px son 30 pt");
    assert!((height - 30.0).abs() < 0.01);
}

#[test]
fn empty_client_data_leaves_no_lines_or_labels() {
    let layout = laid_out(&document(lines(1)));
    assert_eq!(with_role(&layout, TextRole::Client), ["Ana Ruiz"]);
}

#[test]
fn filled_client_data_is_shown() {
    let mut doc = document(lines(1));
    doc.client.tax_id = Some("B12345678".into());
    doc.client.address = Some("Calle Luna 2".into());
    doc.client.phone = Some("611 222 333".into());
    doc.client.email = Some("ana@correo.es".into());
    let client = with_role(&laid_out(&doc), TextRole::Client).join("\n");
    for expected in [
        "Ana Ruiz",
        "B12345678",
        "Calle Luna 2",
        "611 222 333",
        "ana@correo.es",
    ] {
        assert!(client.contains(expected), "falta {expected}");
    }
}

fn amount_items(layout: &LaidOutDocument) -> Vec<TextItem> {
    texts(&layout.pages[0])
        .into_iter()
        .filter(|text| text.role == TextRole::Cell { line: 0 } && text.x > MARGIN + 429.0)
        .cloned()
        .collect()
}

#[test]
fn a_wide_amount_is_reduced_before_wrapping() {
    let mut row = line("Concepto");
    row.amount = "1".repeat(18);
    let items = amount_items(&laid_out(&document(vec![row])));
    assert_eq!(items.len(), 1);
    assert!(items[0].size < 9.0 && items[0].size >= 7.0);
    assert_eq!(items[0].text, "1".repeat(18));
}

#[test]
fn an_amount_that_still_does_not_fit_uses_several_lines_at_7_points() {
    let mut row = line("Concepto");
    let amount = "1".repeat(30);
    row.amount = amount.clone();
    let items = amount_items(&laid_out(&document(vec![row])));
    assert!(items.len() > 1);
    assert!(items.iter().all(|item| item.size == 7.0));
    let joined: String = items.iter().map(|item| item.text.as_str()).collect();
    assert_eq!(joined, amount);
}

#[test]
fn two_hundred_long_lines_lose_no_text() {
    let concept = |i: usize| {
        format!("{i:03} {}", "concepto largo ".repeat(11))[..160]
            .trim()
            .to_string()
    };
    let doc = document((0..200).map(|i| line(&concept(i))).collect());
    let layout = laid_out(&doc);

    for index in 0..200 {
        let shown: String = with_role(&layout, TextRole::Concept { line: index }).concat();
        let without_spaces = |text: &str| text.replace(' ', "");
        assert_eq!(
            without_spaces(&shown),
            without_spaces(&concept(index)),
            "línea {index}"
        );
        assert_eq!(pages_of_line(&layout, index).len(), 1, "línea {index}");
    }
}

#[test]
fn nothing_leaves_the_printable_area() {
    let concept = "concepto largo ".repeat(11);
    let doc = document((0..200).map(|_| line(concept.trim())).collect());
    let layout = laid_out(&doc);
    for page in &layout.pages {
        for text in texts(page) {
            let right = text.x + HalfEm.width(&text.text, text.weight, text.size);
            assert!(
                text.x >= MARGIN - 0.01,
                "«{}» sale por la izquierda",
                text.text
            );
            assert!(
                right <= CONTENT_RIGHT + 0.01,
                "«{}» sale por la derecha",
                text.text
            );
            assert!(
                text.y <= PAGE_HEIGHT - MARGIN + 0.01,
                "«{}» sale por abajo",
                text.text
            );
            assert!(text.y >= MARGIN, "«{}» sale por arriba", text.text);
        }
    }
}

#[test]
fn rows_do_not_overlap() {
    let long = "palabra ".repeat(20);
    let doc = document((0..30).map(|_| line(long.trim())).collect());
    let layout = laid_out(&doc);
    for page in &layout.pages {
        let items = texts(page);
        let bounds: Vec<(f32, f32)> = (0..doc.lines.len())
            .filter_map(|index| {
                let ys: Vec<f32> = items
                    .iter()
                    .filter(|text| text.role == (TextRole::Concept { line: index }))
                    .map(|text| text.y)
                    .collect();
                (!ys.is_empty()).then(|| {
                    (
                        ys.iter().copied().fold(f32::MAX, f32::min),
                        ys.iter().copied().fold(f32::MIN, f32::max),
                    )
                })
            })
            .collect();
        for pair in bounds.windows(2) {
            assert!(pair[1].0 > pair[0].1, "filas solapadas: {pair:?}");
        }
    }
}
