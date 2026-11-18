//! Saltos de renglón y paginación del documento (contracts/pdf-document.md).
//!
//! Código puro: recibe un medidor de texto y devuelve páginas con los elementos ya colocados. Las
//! coordenadas son puntos con origen en la esquina superior izquierda; `y` es la línea base del texto.

use crate::domain::quote_doc::{DocumentLine, LogoSize, QuoteDocument};

pub const PAGE_WIDTH: f32 = 595.28;
pub const PAGE_HEIGHT: f32 = 841.89;
pub const MARGIN: f32 = 42.0;
pub const CONTENT_RIGHT: f32 = PAGE_WIDTH - MARGIN;

const BODY_BOTTOM: f32 = PAGE_HEIGHT - MARGIN - 18.0;
const LOGO_MAX_WIDTH: f32 = 150.0;
const LOGO_MAX_HEIGHT: f32 = 70.0;
const PT_PER_PX: f32 = 0.75;
const HEADER_GAP: f32 = 12.0;
const QUOTE_META_WIDTH: f32 = 170.0;
const LEADING: f32 = 1.35;
const BODY_SIZE: f32 = 9.0;
const MIN_CELL_SIZE: f32 = 7.0;
const SIZE_STEP: f32 = 0.5;
const CELL_PADDING: f32 = 4.0;
const ROW_PADDING: f32 = 3.0;
const TOTALS_LABEL_WIDTH: f32 = 130.0;
const TOTALS_VALUE_WIDTH: f32 = 110.0;

const COL_CONCEPT: Column = Column::new(MARGIN, 235.0);
const COL_QUANTITY: Column = Column::new(MARGIN + 235.0, 56.0);
const COL_UNIT: Column = Column::new(MARGIN + 291.0, 62.0);
const COL_PRICE: Column = Column::new(MARGIN + 353.0, 76.0);
const COL_AMOUNT: Column = Column::new(MARGIN + 429.0, 82.0);

#[derive(Clone, Copy)]
struct Column {
    x: f32,
    width: f32,
}

impl Column {
    const fn new(x: f32, width: f32) -> Self {
        Self { x, width }
    }

    fn inner_width(self) -> f32 {
        self.width - 2.0 * CELL_PADDING
    }

    fn left(self) -> f32 {
        self.x + CELL_PADDING
    }

    fn right(self) -> f32 {
        self.x + self.width - CELL_PADDING
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Regular,
    Bold,
}

/// Mide el ancho de un texto en puntos.
pub trait TextMeasurer {
    fn width(&self, text: &str, weight: FontWeight, size: f32) -> f32;
}

/// Qué representa cada texto colocado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRole {
    Title,
    QuoteMeta,
    BusinessName,
    Business,
    ClientLabel,
    Client,
    ContinuationHeader,
    TableHeader,
    Concept { line: usize },
    Cell { line: usize },
    TotalsLabel,
    TotalsValue,
    PageNumber,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextItem {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub weight: FontWeight,
    pub role: TextRole,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuleItem {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub thickness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogoBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Text(TextItem),
    Rule(RuleItem),
    Logo(LogoBox),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Page {
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LaidOutDocument {
    pub pages: Vec<Page>,
}

/// Parte un texto en renglones: por espacios y, si una palabra no cabe, por caracteres.
pub fn wrap_text(
    text: &str,
    width: f32,
    weight: FontWeight,
    size: f32,
    measurer: &dyn TextMeasurer,
) -> Vec<String> {
    let fits = |candidate: &str| measurer.width(candidate, weight, size) <= width;
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if fits(&candidate) {
            current = candidate;
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if fits(word) {
            current = word.to_string();
            continue;
        }
        for character in word.chars() {
            let mut extended = current.clone();
            extended.push(character);
            if current.is_empty() || fits(&extended) {
                current = extended;
            } else {
                lines.push(std::mem::replace(&mut current, character.to_string()));
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Reduce un valor hasta 7 pt para que quepa y, si aún no cabe, lo reparte en varios renglones.
fn fit_value(
    text: &str,
    width: f32,
    weight: FontWeight,
    measurer: &dyn TextMeasurer,
) -> (f32, Vec<String>) {
    let mut size = BODY_SIZE;
    while size >= MIN_CELL_SIZE {
        if measurer.width(text, weight, size) <= width {
            return (size, vec![text.to_string()]);
        }
        size -= SIZE_STEP;
    }
    (
        MIN_CELL_SIZE,
        wrap_text(text, width, weight, MIN_CELL_SIZE, measurer),
    )
}

fn line_height(size: f32) -> f32 {
    size * LEADING
}

/// Tamaño del logo: cabe en su recuadro, conserva la proporción y nunca se amplía.
fn logo_size(logo: LogoSize) -> Option<(f32, f32)> {
    if logo.width_px == 0 || logo.height_px == 0 {
        return None;
    }
    let width = logo.width_px as f32 * PT_PER_PX;
    let height = logo.height_px as f32 * PT_PER_PX;
    let scale = (LOGO_MAX_WIDTH / width)
        .min(LOGO_MAX_HEIGHT / height)
        .min(1.0);
    Some((width * scale, height * scale))
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
}

/// Un valor ya repartido en renglones dentro de una columna.
struct Cell {
    lines: Vec<String>,
    size: f32,
    column: Column,
    align: Align,
    role: TextRole,
}

impl Cell {
    fn height(&self) -> f32 {
        self.lines.len() as f32 * line_height(self.size)
    }
}

struct Composer<'a> {
    measurer: &'a dyn TextMeasurer,
    pages: Vec<Page>,
    y: f32,
}

impl Composer<'_> {
    fn page(&mut self) -> &mut Page {
        self.pages.last_mut().expect("siempre hay una página")
    }

    fn text(&mut self, text: &str, x: f32, y: f32, size: f32, weight: FontWeight, role: TextRole) {
        self.page().elements.push(Element::Text(TextItem {
            text: text.to_string(),
            x,
            y,
            size,
            weight,
            role,
        }));
    }

    fn text_right(
        &mut self,
        text: &str,
        right: f32,
        y: f32,
        size: f32,
        weight: FontWeight,
        role: TextRole,
    ) {
        let x = right - self.measurer.width(text, weight, size);
        self.text(text, x, y, size, weight, role);
    }

    fn rule(&mut self, thickness: f32) {
        let y = self.y;
        self.page().elements.push(Element::Rule(RuleItem {
            x: MARGIN,
            y,
            width: CONTENT_RIGHT - MARGIN,
            thickness,
        }));
    }

    /// Escribe un texto en renglones a partir de `top` y devuelve dónde termina.
    #[allow(clippy::too_many_arguments)]
    fn block(
        &mut self,
        text: &str,
        x: f32,
        top: f32,
        width: f32,
        size: f32,
        weight: FontWeight,
        role: TextRole,
    ) -> f32 {
        let mut y = top;
        for line in wrap_text(text, width, weight, size, self.measurer) {
            self.text(&line, x, y + size, size, weight, role);
            y += line_height(size);
        }
        y
    }

    fn new_page(&mut self, document: &QuoteDocument) {
        self.pages.push(Page::default());
        let title = format!(
            "Presupuesto {} · {}",
            document.number, document.business.trade_name
        );
        self.y = self.block(
            &title,
            MARGIN,
            MARGIN,
            CONTENT_RIGHT - MARGIN,
            BODY_SIZE,
            FontWeight::Bold,
            TextRole::ContinuationHeader,
        ) + 4.0;
        self.rule(0.75);
        self.y += 8.0;
    }

    /// Número, fechas y título del presupuesto. Devuelve dónde termina el bloque.
    fn quote_meta(&mut self, document: &QuoteDocument) -> f32 {
        let mut y = MARGIN;
        let title_size = 16.0;
        self.text_right(
            "PRESUPUESTO",
            CONTENT_RIGHT,
            y + title_size,
            title_size,
            FontWeight::Bold,
            TextRole::Title,
        );
        y += line_height(title_size) + 2.0;

        let rows = [
            format!("Nº {}", document.number),
            format!("Fecha: {}", document.issue_date),
            format!("Válido hasta: {}", document.valid_until),
        ];
        for row in rows {
            let lines = wrap_text(
                &row,
                QUOTE_META_WIDTH,
                FontWeight::Regular,
                BODY_SIZE,
                self.measurer,
            );
            for line in lines {
                self.text_right(
                    &line,
                    CONTENT_RIGHT,
                    y + BODY_SIZE,
                    BODY_SIZE,
                    FontWeight::Regular,
                    TextRole::QuoteMeta,
                );
                y += line_height(BODY_SIZE);
            }
        }
        y
    }

    /// Logo y datos del negocio. Sin logo, los datos ocupan su lugar (FR-019).
    fn business(&mut self, document: &QuoteDocument) -> f32 {
        let mut x = MARGIN;
        let mut logo_bottom = MARGIN;
        if let Some((width, height)) = document.logo.and_then(logo_size) {
            self.page().elements.push(Element::Logo(LogoBox {
                x: MARGIN,
                y: MARGIN,
                width,
                height,
            }));
            x += width + HEADER_GAP;
            logo_bottom += height;
        }

        let width = CONTENT_RIGHT - QUOTE_META_WIDTH - HEADER_GAP - x;
        let business = &document.business;
        let mut y = self.block(
            &business.trade_name,
            x,
            MARGIN,
            width,
            12.0,
            FontWeight::Bold,
            TextRole::BusinessName,
        );

        let contact = [business.email.as_deref(), business.website.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
        let rows = [
            business.legal_name.clone().unwrap_or_default(),
            format!("NIF: {} · Tel.: {}", business.tax_id, business.phone),
            business.address.clone(),
            contact,
        ];
        for row in rows.iter().filter(|row| !row.is_empty()) {
            y = self.block(
                row,
                x,
                y,
                width,
                BODY_SIZE,
                FontWeight::Regular,
                TextRole::Business,
            );
        }
        y.max(logo_bottom)
    }

    /// Solo aparecen los datos rellenados, sin huecos ni etiquetas vacías (FR-012).
    fn client(&mut self, document: &QuoteDocument) {
        let width = CONTENT_RIGHT - MARGIN;
        let client = &document.client;
        self.y = self.block(
            "Cliente",
            MARGIN,
            self.y,
            width,
            BODY_SIZE,
            FontWeight::Bold,
            TextRole::ClientLabel,
        );
        self.y = self.block(
            &client.name,
            MARGIN,
            self.y,
            width,
            10.0,
            FontWeight::Bold,
            TextRole::Client,
        );
        let rows = [
            client.tax_id.as_ref().map(|value| format!("NIF: {value}")),
            client.address.clone(),
            client.phone.as_ref().map(|value| format!("Tel.: {value}")),
            client.email.as_ref().map(|value| format!("Email: {value}")),
        ];
        for row in rows.into_iter().flatten() {
            self.y = self.block(
                &row,
                MARGIN,
                self.y,
                width,
                BODY_SIZE,
                FontWeight::Regular,
                TextRole::Client,
            );
        }
    }

    fn table_header_height() -> f32 {
        line_height(BODY_SIZE) + 2.0 * ROW_PADDING
    }

    fn table_header(&mut self) {
        let baseline = self.y + ROW_PADDING + BODY_SIZE;
        let role = TextRole::TableHeader;
        let bold = FontWeight::Bold;
        self.text(
            "Concepto",
            COL_CONCEPT.left(),
            baseline,
            BODY_SIZE,
            bold,
            role,
        );
        self.text_right(
            "Cantidad",
            COL_QUANTITY.right(),
            baseline,
            BODY_SIZE,
            bold,
            role,
        );
        self.text("Unidad", COL_UNIT.left(), baseline, BODY_SIZE, bold, role);
        self.text_right(
            "Precio sin IVA",
            COL_PRICE.right(),
            baseline,
            BODY_SIZE,
            bold,
            role,
        );
        self.text_right(
            "Importe",
            COL_AMOUNT.right(),
            baseline,
            BODY_SIZE,
            bold,
            role,
        );
        self.y += Self::table_header_height();
        self.rule(0.75);
    }

    fn row_cells(&self, index: usize, line: &DocumentLine) -> Vec<Cell> {
        let regular = FontWeight::Regular;
        let role = TextRole::Cell { line: index };
        let value = |text: &str, column: Column| {
            let (size, lines) = fit_value(text, column.inner_width(), regular, self.measurer);
            Cell {
                lines,
                size,
                column,
                align: Align::Right,
                role,
            }
        };
        let wrapped = |text: &str, column: Column, role: TextRole| Cell {
            lines: wrap_text(
                text,
                column.inner_width(),
                regular,
                BODY_SIZE,
                self.measurer,
            ),
            size: BODY_SIZE,
            column,
            align: Align::Left,
            role,
        };
        vec![
            wrapped(
                &line.concept,
                COL_CONCEPT,
                TextRole::Concept { line: index },
            ),
            value(&line.quantity, COL_QUANTITY),
            wrapped(&line.unit, COL_UNIT, role),
            value(&line.price, COL_PRICE),
            value(&line.amount, COL_AMOUNT),
        ]
    }

    fn draw_row(&mut self, cells: &[Cell], height: f32) {
        for cell in cells {
            let mut top = self.y + ROW_PADDING;
            for line in &cell.lines {
                let baseline = top + cell.size;
                match cell.align {
                    Align::Left => self.text(
                        line,
                        cell.column.left(),
                        baseline,
                        cell.size,
                        FontWeight::Regular,
                        cell.role,
                    ),
                    Align::Right => self.text_right(
                        line,
                        cell.column.right(),
                        baseline,
                        cell.size,
                        FontWeight::Regular,
                        cell.role,
                    ),
                }
                top += line_height(cell.size);
            }
        }
        self.y += height;
        self.rule(0.25);
    }

    /// Las filas nunca se parten y la cabecera se repite en cada página con líneas (FR-017).
    fn table(&mut self, document: &QuoteDocument) {
        let mut header_on_page = false;
        for (index, line) in document.lines.iter().enumerate() {
            let cells = self.row_cells(index, line);
            let content = cells.iter().map(Cell::height).fold(0.0, f32::max);
            let height = content + 2.0 * ROW_PADDING;
            let header = if header_on_page {
                0.0
            } else {
                Self::table_header_height()
            };
            if self.y + header + height > BODY_BOTTOM {
                self.new_page(document);
                header_on_page = false;
            }
            if !header_on_page {
                self.table_header();
                header_on_page = true;
            }
            self.draw_row(&cells, height);
        }
    }

    /// Los totales aparecen una sola vez, al final y sin separarse (regla 7).
    fn totals(&mut self, document: &QuoteDocument) {
        let totals = &document.totals;
        let rows = [
            ("Base imponible", totals.base.as_str(), FontWeight::Regular),
            (
                totals.vat_label.as_str(),
                totals.vat.as_str(),
                FontWeight::Regular,
            ),
            ("TOTAL", totals.total.as_str(), FontWeight::Bold),
        ];
        let inner = TOTALS_VALUE_WIDTH - CELL_PADDING;
        let fitted: Vec<(&str, FontWeight, f32, Vec<String>)> = rows
            .iter()
            .map(|(label, value, weight)| {
                let (size, lines) = fit_value(value, inner, *weight, self.measurer);
                (*label, *weight, size, lines)
            })
            .collect();
        let row_height = |size: f32, count: usize| {
            (count as f32 * line_height(size)).max(line_height(BODY_SIZE)) + 2.0 * ROW_PADDING
        };
        let height: f32 = fitted
            .iter()
            .map(|(_, _, size, lines)| row_height(*size, lines.len()))
            .sum();

        self.y += 8.0;
        if self.y + height > BODY_BOTTOM {
            self.new_page(document);
        }

        let right = CONTENT_RIGHT - CELL_PADDING;
        let label_x = right - TOTALS_VALUE_WIDTH - TOTALS_LABEL_WIDTH;
        for (label, weight, size, lines) in fitted {
            let top = self.y + ROW_PADDING;
            self.text(
                label,
                label_x,
                top + BODY_SIZE,
                BODY_SIZE,
                weight,
                TextRole::TotalsLabel,
            );
            for (number, line) in lines.iter().enumerate() {
                let baseline = top + size + number as f32 * line_height(size);
                self.text_right(line, right, baseline, size, weight, TextRole::TotalsValue);
            }
            self.y += row_height(size, lines.len());
        }
    }

    /// `Página X de Y` en todas las páginas (regla 9).
    fn page_numbers(&mut self) {
        let total = self.pages.len();
        for number in 0..total {
            let text = format!("Página {} de {total}", number + 1);
            let size = 8.0;
            let x = CONTENT_RIGHT - self.measurer.width(&text, FontWeight::Regular, size);
            self.pages[number].elements.push(Element::Text(TextItem {
                text,
                x,
                y: PAGE_HEIGHT - MARGIN,
                size,
                weight: FontWeight::Regular,
                role: TextRole::PageNumber,
            }));
        }
    }
}

/// Coloca todo el documento en páginas A4.
pub fn layout_document(document: &QuoteDocument, measurer: &dyn TextMeasurer) -> LaidOutDocument {
    let mut composer = Composer {
        measurer,
        pages: vec![Page::default()],
        y: MARGIN,
    };

    let meta_bottom = composer.quote_meta(document);
    let business_bottom = composer.business(document);
    composer.y = meta_bottom.max(business_bottom) + 8.0;
    composer.rule(0.75);
    composer.y += 8.0;

    composer.client(document);
    composer.y += 6.0;
    composer.rule(0.75);
    composer.y += 6.0;

    composer.table(document);
    composer.totals(document);
    composer.page_numbers();

    LaidOutDocument {
        pages: composer.pages,
    }
}
