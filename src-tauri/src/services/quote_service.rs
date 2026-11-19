//! Casos de uso del PDF del presupuesto: preparar, guardar los datos y generar.
//!
//! El documento copia los importes ya calculados por el borrador (FR-009). Los registros técnicos
//! no incluyen datos del cliente, contenido del presupuesto ni rutas (FR-028b).

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{Date, OffsetDateTime};

use crate::domain::draft::totals::compute_totals;
use crate::domain::draft::{DraftLine, StoredDraft};
use crate::domain::money::{format_eur, format_quantity};
use crate::domain::profile::{self, Address, BusinessProfile, DEFAULT_VAT_BP};
use crate::domain::quote_doc::blockers::{
    self, BlockerInput, CheckedText, QuoteWarning, WarningInput,
};
use crate::domain::quote_doc::dates::{format_date, format_iso, valid_until};
use crate::domain::quote_doc::details::{validate, QuoteDetailsInput};
use crate::domain::quote_doc::numbering::{
    content_hash, parse_series_number, propose, ContentLine, Proposal,
};
use crate::domain::quote_doc::{
    DocumentBusiness, DocumentClient, DocumentLine, DocumentTotals, LogoSize, QuoteDetails,
    QuoteDocument,
};
use crate::error::{AppError, ErrorCode, FieldError, QuoteBlocker};
use crate::pdf::fonts::PdfFonts;
use crate::pdf::logo::{normalize, NormalizedLogo};
use crate::pdf::renderer::render_document;
use crate::services::draft_service::current_vat_rate_bp;
use crate::state::{AppState, GenerationGuard};
use crate::storage::{draft_repo, pdf_store, profile_repo, quote_repo};

const NO_DRAFT: &str = "Todavía no hay ningún presupuesto en curso.";
const BUSY: &str = "Ya se está generando un PDF. Espera a que termine.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotePreparationView {
    pub details: QuoteDetailsInput,
    pub issue_date_display: String,
    pub valid_until_display: String,
    pub line_count: usize,
    pub total_display: String,
    pub blockers: Vec<QuoteBlocker>,
    pub warnings: Vec<QuoteWarning>,
    pub confirmation_token: Option<String>,
}

/// Archivo ya guardado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPdf {
    pub file_name: String,
    pub full_path: PathBuf,
    pub page_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "outcome",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuoteGenerateResult {
    Saved {
        file_name: String,
        full_path: String,
        page_count: usize,
    },
    Cancelled,
}

impl From<SavedPdf> for QuoteGenerateResult {
    fn from(saved: SavedPdf) -> Self {
        QuoteGenerateResult::Saved {
            file_name: saved.file_name,
            full_path: saved.full_path.to_string_lossy().into_owned(),
            page_count: saved.page_count,
        }
    }
}

/// Todo lo que se va a emitir, calculado a partir de lo guardado.
struct Assembled {
    details: QuoteDetails,
    number: String,
    content_hash: String,
    document: QuoteDocument,
    logo: Option<NormalizedLogo>,
    blockers: Vec<QuoteBlocker>,
    warnings: Vec<QuoteWarning>,
    token: String,
}

/// Fecha local del equipo. Solo el servicio consulta el reloj (R12).
pub fn today() -> Date {
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .date()
}

fn amount_error(code: ErrorCode) -> AppError {
    AppError::field("price", code)
}

fn stored_lines(draft: &StoredDraft) -> Vec<DraftLine> {
    draft.lines.iter().map(|built| built.line.clone()).collect()
}

fn draft_content_hash(lines: &[DraftLine], vat_rate_bp: i64) -> String {
    let content: Vec<ContentLine<'_>> = lines
        .iter()
        .map(|line| ContentLine {
            concept: &line.concept,
            quantity_centi: line.quantity_centi.unwrap_or(0),
            unit: line.unit_label.as_deref().unwrap_or(""),
            price_cents: line.price_cents.unwrap_or(0),
        })
        .collect();
    content_hash(&content, vat_rate_bp)
}

/// Número que propone la app si el profesional no ha escrito otro.
fn automatic_number(
    conn: &Connection,
    details: &QuoteDetails,
    current_hash: &str,
    year: i32,
) -> Result<String, AppError> {
    Ok(propose(&Proposal {
        manual: None,
        issued_number: details.issued_number.as_deref(),
        issued_hash: details.issued_content_hash.as_deref(),
        current_hash,
        year,
        last_number: quote_repo::last_number(conn, year)?,
    }))
}

fn document_lines(lines: &[DraftLine]) -> Result<Vec<DocumentLine>, AppError> {
    lines
        .iter()
        .map(|line| {
            let amount = line.amount_cents().map_err(amount_error)?;
            Ok(DocumentLine {
                concept: line.concept.clone(),
                quantity: line.quantity_centi.map(format_quantity).unwrap_or_default(),
                unit: line.unit_label.clone().unwrap_or_default(),
                price: line.price_cents.map(format_eur).unwrap_or_default(),
                amount: amount.map(format_eur).unwrap_or_default(),
            })
        })
        .collect()
}

fn document_business(profile: Option<&BusinessProfile>) -> DocumentBusiness {
    let Some(profile) = profile else {
        return DocumentBusiness::default();
    };
    let address = &profile.address;
    DocumentBusiness {
        trade_name: profile.trade_name.clone(),
        legal_name: profile.legal_name.clone(),
        tax_id: profile.tax_id.clone(),
        phone: profile.phone.clone(),
        address: format!(
            "{}, {} {} ({})",
            address.street, address.postal_code, address.city, address.province
        ),
        email: profile.email.clone(),
        website: profile.website.clone(),
    }
}

fn document_client(details: &QuoteDetails) -> DocumentClient {
    DocumentClient {
        name: details.client_name.clone(),
        tax_id: details.client_tax_id.clone(),
        address: details.client_address.clone(),
        phone: details.client_phone.clone(),
        email: details.client_email.clone(),
    }
}

/// Bytes del logo guardado, si se puede leer.
fn read_logo(data_dir: &Path, profile: Option<&BusinessProfile>) -> Option<Vec<u8>> {
    let meta = profile?.logo.as_ref()?;
    std::fs::read(data_dir.join(&meta.file)).ok()
}

/// SHA-256 del contenido completo del documento: lo confirmado es lo que se emite (R11).
fn confirmation_token(document: &QuoteDocument, logo_bytes: Option<&[u8]>) -> String {
    let mut hasher = Sha256::new();
    let mut field = |value: &str| {
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    };
    let optional = |value: &Option<String>| value.clone().unwrap_or_default();

    field(&document.number);
    field(&document.issue_date);
    field(&document.valid_until);

    let client = &document.client;
    field(&client.name);
    field(&optional(&client.tax_id));
    field(&optional(&client.address));
    field(&optional(&client.phone));
    field(&optional(&client.email));

    let business = &document.business;
    field(&business.trade_name);
    field(&optional(&business.legal_name));
    field(&business.tax_id);
    field(&business.phone);
    field(&business.address);
    field(&optional(&business.email));
    field(&optional(&business.website));

    field(
        &logo_bytes
            .map(|bytes| hex::encode(Sha256::digest(bytes)))
            .unwrap_or_default(),
    );

    field(&document.lines.len().to_string());
    for line in &document.lines {
        field(&line.concept);
        field(&line.quantity);
        field(&line.unit);
        field(&line.price);
        field(&line.amount);
    }

    let totals = &document.totals;
    field(&totals.vat_label);
    field(&totals.base);
    field(&totals.vat);
    field(&totals.total);

    hex::encode(hasher.finalize())
}

/// Datos obligatorios del negocio que faltan. Sin perfil guardado, faltan todos.
fn missing_profile_fields(profile: Option<&BusinessProfile>) -> Vec<FieldError> {
    match profile {
        Some(profile) => profile::missing_fields(profile),
        None => profile::missing_fields(&BusinessProfile {
            trade_name: String::new(),
            legal_name: None,
            tax_id: String::new(),
            phone: String::new(),
            email: None,
            website: None,
            address: Address::default(),
            vat_rate_bp: DEFAULT_VAT_BP,
            logo: None,
        }),
    }
}

/// Todos los textos que se imprimen, con el nombre del dato que hay que corregir (FR-018b).
fn printed_texts(document: &QuoteDocument, lines: &[DraftLine]) -> Vec<(String, String, String)> {
    let entry = |field: &str, label: &str, text: &str| {
        (field.to_string(), label.to_string(), text.to_string())
    };
    let optional = |field: &str, label: &str, text: &Option<String>| {
        text.as_deref().map(|text| entry(field, label, text))
    };

    let client = &document.client;
    let business = &document.business;
    let mut texts = vec![
        entry("quoteNumber", "Número del presupuesto", &document.number),
        entry("clientName", "Nombre del cliente", &client.name),
    ];
    texts.extend(
        [
            optional("clientAddress", "Dirección del cliente", &client.address),
            optional("clientPhone", "Teléfono del cliente", &client.phone),
            optional("clientEmail", "Email del cliente", &client.email),
            optional("clientTaxId", "NIF del cliente", &client.tax_id),
        ]
        .into_iter()
        .flatten(),
    );
    texts.push(entry("tradeName", "Nombre comercial", &business.trade_name));
    texts.extend(optional("legalName", "Razón social", &business.legal_name));
    texts.push(entry("taxId", "NIF / CIF del negocio", &business.tax_id));
    texts.push(entry("phone", "Teléfono del negocio", &business.phone));
    texts.push(entry("address", "Dirección del negocio", &business.address));
    texts.extend(optional("email", "Email del negocio", &business.email));
    texts.extend(optional("website", "Web del negocio", &business.website));

    for line in lines {
        texts.push(entry(
            &format!("lines.{}.concept", line.id),
            &format!("Concepto «{}»", line.concept),
            &line.concept,
        ));
        if let Some(unit) = &line.unit_label {
            texts.push(entry(
                &format!("lines.{}.unitLabel", line.id),
                &format!("Unidad de «{}»", line.concept),
                unit,
            ));
        }
    }
    texts
}

fn assemble(
    conn: &Connection,
    data_dir: &Path,
    today: Date,
    allow_missing_logo: bool,
) -> Result<Assembled, AppError> {
    let draft = draft_repo::load(conn)?.ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
    let details = quote_repo::get_details(conn)?.unwrap_or_default();
    let profile = profile_repo::get(conn)?;

    let lines = stored_lines(&draft);
    let totals = compute_totals(&lines, draft.vat_rate_bp).map_err(amount_error)?;
    let hash = draft_content_hash(&lines, draft.vat_rate_bp);
    let number = match &details.quote_number {
        Some(manual) => manual.clone(),
        None => automatic_number(conn, &details, &hash, today.year())?,
    };

    let logo_bytes = read_logo(data_dir, profile.as_ref());
    let logo = logo_bytes
        .as_deref()
        .and_then(|bytes| normalize(bytes).ok());
    let logo_configured = profile
        .as_ref()
        .is_some_and(|profile| profile.logo.is_some());
    let logo_unreadable = logo_configured && logo.is_none();

    let document = QuoteDocument {
        number: number.clone(),
        issue_date: format_date(today),
        valid_until: format_date(valid_until(today, details.validity_days)),
        business: document_business(profile.as_ref()),
        client: document_client(&details),
        lines: document_lines(&lines)?,
        totals: DocumentTotals {
            base: format_eur(totals.base_cents),
            vat_label: format!("IVA ({} %)", format_quantity(draft.vat_rate_bp)),
            vat: format_eur(totals.vat_cents),
            total: format_eur(totals.total_cents),
        },
        logo: logo.as_ref().map(|logo| LogoSize {
            width_px: logo.width_px,
            height_px: logo.height_px,
        }),
    };

    let fonts = PdfFonts::load().ok_or_else(AppError::pdf_render)?;
    let missing = missing_profile_fields(profile.as_ref());
    let printed = printed_texts(&document, &lines);
    let texts: Vec<CheckedText<'_>> = printed
        .iter()
        .map(|(field, label, text)| CheckedText { field, label, text })
        .collect();
    let blockers = blockers::evaluate(
        &BlockerInput {
            lines: &lines,
            status: draft.status,
            missing_profile_fields: &missing,
            client_name: &details.client_name,
            quote_number: &number,
            texts: &texts,
            logo_unreadable: logo_unreadable && !allow_missing_logo,
        },
        &|character| fonts.supports(character),
    );
    let warnings = blockers::warnings(&WarningInput {
        has_logo: logo.is_some(),
        accepted_unreadable_logo: logo_unreadable && allow_missing_logo,
        applied_vat_bp: draft.vat_rate_bp,
        current_vat_bp: current_vat_rate_bp(conn)?,
        unparsed_fragments: draft.unparsed_fragments.len(),
    });
    let token = confirmation_token(&document, logo.as_ref().and(logo_bytes.as_deref()));

    Ok(Assembled {
        details,
        number,
        content_hash: hash,
        document,
        logo,
        blockers,
        warnings,
        token,
    })
}

fn preparation_view(assembled: Assembled) -> QuotePreparationView {
    let details = assembled.details;
    let document = assembled.document;
    let confirmation_token = assembled.blockers.is_empty().then_some(assembled.token);
    QuotePreparationView {
        details: QuoteDetailsInput {
            client_name: details.client_name,
            client_address: details.client_address,
            client_phone: details.client_phone,
            client_email: details.client_email,
            client_tax_id: details.client_tax_id,
            quote_number: assembled.number,
            validity_days: details.validity_days,
        },
        issue_date_display: document.issue_date,
        valid_until_display: document.valid_until,
        line_count: document.lines.len(),
        total_display: document.totals.total,
        blockers: assembled.blockers,
        warnings: assembled.warnings,
        confirmation_token,
    }
}

/// Calcula lo que se va a emitir. No crea archivos ni gasta números.
pub fn prepare_on(
    state: &AppState,
    today: Date,
    allow_missing_logo: bool,
) -> Result<QuotePreparationView, AppError> {
    let conn = state.conn()?;
    assemble(&conn, &state.data_dir, today, allow_missing_logo).map(preparation_view)
}

pub fn prepare(
    state: &AppState,
    allow_missing_logo: bool,
) -> Result<QuotePreparationView, AppError> {
    prepare_on(state, today(), allow_missing_logo)
}

/// Documento que se emitiría ahora mismo.
pub fn document_on(
    state: &AppState,
    today: Date,
    allow_missing_logo: bool,
) -> Result<QuoteDocument, AppError> {
    let conn = state.conn()?;
    assemble(&conn, &state.data_dir, today, allow_missing_logo).map(|assembled| assembled.document)
}

/// Guarda los datos del presupuesto. No cambia el estado de revisión del borrador (FR-012b).
pub fn save_details_on(
    state: &AppState,
    input: QuoteDetailsInput,
    today: Date,
    allow_missing_logo: bool,
) -> Result<QuotePreparationView, AppError> {
    let clean = validate(&input).map_err(AppError::validation)?;

    let mut conn = state.conn()?;
    let tx = conn.transaction()?;
    let draft = draft_repo::load(&tx)?.ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
    let previous = quote_repo::get_details(&tx)?.unwrap_or_default();

    let hash = draft_content_hash(&stored_lines(&draft), draft.vat_rate_bp);
    let automatic = automatic_number(&tx, &previous, &hash, today.year())?;
    // Escribir el número propuesto lo deja en automático (data-model.md).
    let quote_number = (clean.quote_number != automatic).then_some(clean.quote_number);

    quote_repo::save_details(
        &tx,
        &QuoteDetails {
            client_name: clean.client_name,
            client_address: clean.client_address,
            client_phone: clean.client_phone,
            client_email: clean.client_email,
            client_tax_id: clean.client_tax_id,
            quote_number,
            issued_number: previous.issued_number,
            issued_content_hash: previous.issued_content_hash,
            validity_days: clean.validity_days,
        },
    )?;
    let view = preparation_view(assemble(&tx, &state.data_dir, today, allow_missing_logo)?);
    tx.commit()?;
    Ok(view)
}

pub fn save_details(
    state: &AppState,
    input: QuoteDetailsInput,
    allow_missing_logo: bool,
) -> Result<QuotePreparationView, AppError> {
    save_details_on(state, input, today(), allow_missing_logo)
}

/// Solo puede haber una generación activa a la vez (FR-026).
pub fn begin_generation(state: &AppState) -> Result<GenerationGuard<'_>, AppError> {
    state
        .quote
        .try_begin()
        .ok_or_else(|| AppError::invalid_state(BUSY))
}

fn ensure_confirmed(assembled: &Assembled, token: &str) -> Result<(), AppError> {
    if !assembled.blockers.is_empty() {
        return Err(AppError::quote_blocked(assembled.blockers.clone()));
    }
    if assembled.token != token {
        return Err(AppError::quote_changed());
    }
    Ok(())
}

/// Comprueba que no haya bloqueos y que lo confirmado siga vigente.
pub fn check_confirmation_on(
    state: &AppState,
    token: &str,
    allow_missing_logo: bool,
    today: Date,
) -> Result<(), AppError> {
    let conn = state.conn()?;
    let assembled = assemble(&conn, &state.data_dir, today, allow_missing_logo)?;
    ensure_confirmed(&assembled, token)
}

/// Sustituye por un guion lo que no se admite en un nombre de archivo de Windows (FR-020).
fn safe_file_part(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() || r#"<>:"/\|?*"#.contains(character) {
                '-'
            } else {
                character
            }
        })
        .collect()
}

/// Nombre que se propone al elegir la ubicación: `Presupuesto 2026-001 2026-11-18.pdf`.
pub fn proposed_file_name_on(
    state: &AppState,
    today: Date,
    allow_missing_logo: bool,
) -> Result<String, AppError> {
    let conn = state.conn()?;
    let assembled = assemble(&conn, &state.data_dir, today, allow_missing_logo)?;
    Ok(format!(
        "Presupuesto {} {}.pdf",
        safe_file_part(&assembled.number),
        format_iso(today)
    ))
}

/// Asegura que el archivo termine en `.pdf`, sin duplicar la extensión (FR-020).
pub fn with_pdf_extension(path: &Path) -> PathBuf {
    let is_pdf = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));
    if is_pdf {
        return path.to_path_buf();
    }
    let mut named = path.as_os_str().to_owned();
    named.push(".pdf");
    PathBuf::from(named)
}

/// Compone el PDF, gasta el número y guarda el archivo en una sola operación (R9). Si la escritura
/// falla, la transacción se deshace: el número no se gasta (FR-013b).
pub fn save_to_path_on(
    state: &AppState,
    token: &str,
    allow_missing_logo: bool,
    path: &Path,
    today: Date,
) -> Result<SavedPdf, AppError> {
    let path = with_pdf_extension(path);
    let mut conn = state.conn()?;
    let tx = conn.transaction()?;
    let assembled = assemble(&tx, &state.data_dir, today, allow_missing_logo)?;
    ensure_confirmed(&assembled, token)?;

    let rendered = render_document(&assembled.document, today, assembled.logo.as_ref())?;

    if let Some((year, number)) = parse_series_number(&assembled.number) {
        quote_repo::record_number(&tx, year, number)?;
    }
    quote_repo::save_details(
        &tx,
        &QuoteDetails {
            quote_number: None,
            issued_number: Some(assembled.number),
            issued_content_hash: Some(assembled.content_hash),
            ..assembled.details
        },
    )?;

    pdf_store::write_atomic(&path, &rendered.bytes)?;
    tx.commit()?;

    Ok(SavedPdf {
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        full_path: path,
        page_count: rendered.page_count,
    })
}

/// Genera el PDF en una ruta ya elegida.
pub fn generate_to_path_on(
    state: &AppState,
    token: &str,
    allow_missing_logo: bool,
    path: &Path,
    today: Date,
) -> Result<SavedPdf, AppError> {
    let _generation = begin_generation(state)?;
    save_to_path_on(state, token, allow_missing_logo, path, today)
}
