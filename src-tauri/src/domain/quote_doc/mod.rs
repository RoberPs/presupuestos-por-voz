//! Documento del presupuesto: tipos del dominio, sin dependencias de Tauri ni de librerías de PDF.
//!
//! Los importes llegan ya formateados desde `domain/draft/totals.rs` y `domain/money.rs`: aquí no
//! se calcula dinero (FR-009).

pub mod blockers;
pub mod dates;
pub mod details;
pub mod layout;
pub mod numbering;

/// Validez propuesta, en días (FR-014b).
pub const DEFAULT_VALIDITY_DAYS: i64 = 30;

/// Datos del presupuesto en curso, guardados junto al borrador (FR-012b).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteDetails {
    pub client_name: String,
    pub client_address: Option<String>,
    pub client_phone: Option<String>,
    pub client_email: Option<String>,
    pub client_tax_id: Option<String>,
    /// Número escrito a mano. `None` = usar el propuesto; vacío = el profesional lo ha borrado.
    pub quote_number: Option<String>,
    pub issued_number: Option<String>,
    pub issued_content_hash: Option<String>,
    pub validity_days: i64,
}

impl Default for QuoteDetails {
    fn default() -> Self {
        Self {
            client_name: String::new(),
            client_address: None,
            client_phone: None,
            client_email: None,
            client_tax_id: None,
            quote_number: None,
            issued_number: None,
            issued_content_hash: None,
            validity_days: DEFAULT_VALIDITY_DAYS,
        }
    }
}

/// Una línea del documento, con los valores ya formateados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLine {
    pub concept: String,
    pub quantity: String,
    pub unit: String,
    pub price: String,
    pub amount: String,
}

/// Totales del documento, copiados del borrador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTotals {
    pub base: String,
    pub vat_label: String,
    pub vat: String,
    pub total: String,
}

/// Datos del negocio que aparecen en la cabecera (FR-007).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentBusiness {
    pub trade_name: String,
    pub legal_name: Option<String>,
    pub tax_id: String,
    pub phone: String,
    pub address: String,
    pub email: Option<String>,
    pub website: Option<String>,
}

/// Datos del cliente que aparecen en el documento (FR-012).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentClient {
    pub name: String,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

/// Tamaño natural del logo, en píxeles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogoSize {
    pub width_px: u32,
    pub height_px: u32,
}

/// Todo lo que se imprime en el PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteDocument {
    pub number: String,
    pub issue_date: String,
    pub valid_until: String,
    pub business: DocumentBusiness,
    pub client: DocumentClient,
    pub lines: Vec<DocumentLine>,
    pub totals: DocumentTotals,
    pub logo: Option<LogoSize>,
}
