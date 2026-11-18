//! Casos de uso del borrador: generación, edición y vistas con los importes ya calculados por el
//! núcleo. La interfaz nunca hace cálculos de dinero.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::audio::wav;
use crate::domain::draft::builder::build_draft;
use crate::domain::draft::totals::compute_totals;
use crate::domain::draft::units::UnitCatalog;
use crate::domain::draft::{
    BuiltDraft, BuiltLine, DraftLine, DraftStatus, PendingReason, PriceSource, QuantityIssue,
    StoredDraft, TextOrigin,
};
use crate::domain::money::{
    format_eur, format_percent_bp, format_quantity, parse_price_cents, parse_quantity_centi,
};
use crate::domain::normalize::collapse_spaces;
use crate::domain::rate::{RateView, MAX_CONCEPT_CHARS, MAX_UNIT_CHARS};
use crate::error::{AppError, ConfirmationReason, ErrorCode, FieldError};
use crate::state::{Activity, AppState};
use crate::storage::dictation_repo::{self, Slot};
use crate::storage::{draft_repo, profile_repo, quote_repo, rate_repo, recording_store, unit_repo};

/// IVA general cuando el negocio aún no ha guardado su perfil.
const DEFAULT_VAT_RATE_BP: i64 = 2_100;
const LINE_NOT_FOUND: &str = "Esa línea ya no existe.";
const RATE_NOT_FOUND: &str = "Esa tarifa ya no existe. Puede que se haya borrado.";
const NO_DRAFT: &str = "Todavía no hay ningún presupuesto en curso.";
const NO_LINES: &str = "Añade al menos una línea antes de dar el presupuesto por bueno.";
const BUSY: &str = "Espera a que termine el dictado en curso.";

/// Longitud máxima del texto de la visita.
const MAX_TEXT_CHARS: usize = 20_000;

/// Cómo se genera el borrador a partir de un texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerateMode {
    New,
    Regenerate,
}

/// Línea nueva, desde una tarifa o escrita a mano (FR-025).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DraftLineInput {
    Rate {
        rate_id: i64,
        quantity: String,
    },
    Manual {
        concept: String,
        quantity: String,
        unit_label: String,
        price: String,
    },
}

/// Campos que el profesional ha cambiado en una línea.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DraftLinePatch {
    pub concept: Option<String>,
    pub quantity: Option<String>,
    pub unit_label: Option<String>,
    pub price: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftTotalsView {
    pub base_cents: i64,
    pub base_display: String,
    pub vat_rate_percent: String,
    pub vat_cents: i64,
    pub vat_display: String,
    pub total_cents: i64,
    pub total_display: String,
    pub pending_count: i64,
    pub is_provisional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftLineView {
    pub id: i64,
    pub concept: String,
    pub quantity: Option<String>,
    pub quantity_issue: Option<QuantityIssue>,
    pub unit_label: Option<String>,
    pub price_cents: Option<i64>,
    pub price_display: Option<String>,
    pub amount_cents: Option<i64>,
    pub amount_display: Option<String>,
    pub price_source: PriceSource,
    pub rate_id: Option<i64>,
    pub rate_concept: Option<String>,
    pub pending_reason: Option<PendingReason>,
    pub suggestions: Vec<RateView>,
    pub source_fragment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VatChangeView {
    pub current_rate_percent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftView {
    pub status: DraftStatus,
    pub source_text: String,
    pub text_origin: TextOrigin,
    pub text_corrected: bool,
    pub has_manual_changes: bool,
    pub lines: Vec<DraftLineView>,
    pub unparsed_fragments: Vec<String>,
    pub totals: DraftTotalsView,
    pub vat_change: Option<VatChangeView>,
}

fn amount_error(code: ErrorCode) -> AppError {
    AppError::field("price", code)
}

/// IVA general vigente del negocio.
pub fn current_vat_rate_bp(conn: &Connection) -> Result<i64, AppError> {
    Ok(profile_repo::get(conn)?.map_or(DEFAULT_VAT_RATE_BP, |profile| profile.vat_rate_bp))
}

fn line_view(conn: &Connection, built: &BuiltLine) -> Result<DraftLineView, AppError> {
    let line: &DraftLine = &built.line;
    let amount_cents = line.amount_cents().map_err(amount_error)?;

    let mut suggestions = Vec::with_capacity(built.suggestion_rate_ids.len());
    for rate_id in &built.suggestion_rate_ids {
        // El precio de la sugerencia se lee de la tarifa vigente al mostrarla.
        if let Some(rate) = rate_repo::get(conn, *rate_id)? {
            suggestions.push(rate);
        }
    }

    Ok(DraftLineView {
        id: line.id,
        concept: line.concept.clone(),
        quantity: line.quantity_centi.map(format_quantity),
        quantity_issue: line.quantity_issue,
        unit_label: line.unit_label.clone(),
        price_cents: line.price_cents,
        price_display: line.price_cents.map(format_eur),
        amount_cents,
        amount_display: amount_cents.map(format_eur),
        price_source: line.price_source,
        rate_id: line.rate_id,
        rate_concept: line.rate_concept.clone(),
        pending_reason: line.pending_reason,
        suggestions,
        source_fragment: line.source_fragment.clone(),
    })
}

/// Construye la vista completa de un borrador guardado.
pub fn build_view(conn: &Connection, draft: &StoredDraft) -> Result<DraftView, AppError> {
    let lines: Vec<DraftLine> = draft.lines.iter().map(|l| l.line.clone()).collect();
    let totals = compute_totals(&lines, draft.vat_rate_bp).map_err(amount_error)?;

    let current_vat = current_vat_rate_bp(conn)?;
    let vat_change = (current_vat != draft.vat_rate_bp).then(|| VatChangeView {
        current_rate_percent: format_percent_bp(current_vat),
    });

    Ok(DraftView {
        status: draft.status,
        source_text: draft.source_text.clone(),
        text_origin: draft.text_origin,
        text_corrected: draft.text_corrected,
        has_manual_changes: draft.has_manual_changes,
        lines: draft
            .lines
            .iter()
            .map(|line| line_view(conn, line))
            .collect::<Result<_, _>>()?,
        unparsed_fragments: draft.unparsed_fragments.clone(),
        totals: DraftTotalsView {
            base_cents: totals.base_cents,
            base_display: format_eur(totals.base_cents),
            vat_rate_percent: format_percent_bp(draft.vat_rate_bp),
            vat_cents: totals.vat_cents,
            vat_display: format_eur(totals.vat_cents),
            total_cents: totals.total_cents,
            total_display: format_eur(totals.total_cents),
            pending_count: totals.pending_count,
            is_provisional: totals.is_provisional,
        },
        vat_change,
    })
}

/// Vista del borrador en curso, si existe.
pub fn load_view(conn: &Connection) -> Result<Option<DraftView>, AppError> {
    draft_repo::load(conn)?
        .map(|draft| build_view(conn, &draft))
        .transpose()
}

/// Interpreta un texto con las tarifas, las unidades y el IVA vigentes.
pub fn build_from_text(conn: &Connection, text: &str) -> Result<BuiltDraft, AppError> {
    let rates = rate_repo::list_all(conn)?;
    let units = unit_repo::list(conn)?;
    Ok(build_draft(
        text,
        &rates,
        &units,
        current_vat_rate_bp(conn)?,
    ))
}

fn valid_text(text: &str) -> Result<(), AppError> {
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(AppError::field("text", ErrorCode::TooLong));
    }
    Ok(())
}

/// Rehace el borrador con el texto corregido, con las tarifas y el IVA vigentes (FR-011 y FR-029).
fn regenerate(conn: &Connection, text: &str, confirmed: bool) -> Result<DraftView, AppError> {
    let previous = draft_repo::load(conn)?.ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
    if previous.has_manual_changes && !confirmed {
        return Err(AppError::confirmation_required(
            ConfirmationReason::DiscardManualChanges,
        ));
    }
    let built = build_from_text(conn, text)?;
    // `replace` borra la fila del borrador y, en cascada, los datos del presupuesto (R10).
    let quote_details = quote_repo::get_details(conn)?;
    draft_repo::replace(conn, &built, text, previous.text_origin)?;
    if let Some(details) = quote_details {
        quote_repo::save_details(conn, &details)?;
    }
    let corrected = previous.text_corrected || previous.source_text != text;
    draft_repo::set_source_text(conn, text, corrected)?;
    let draft = draft_repo::load(conn)?.ok_or_else(AppError::storage)?;
    build_view(conn, &draft)
}

/// Crea un borrador desde un texto escrito. Sustituye al anterior y a su audio (FR-013).
fn generate_new(
    state: &AppState,
    conn: &Connection,
    text: &str,
    confirmed: bool,
) -> Result<(DraftView, Vec<String>), AppError> {
    if !confirmed {
        let part = recording_store::part_path(&state.data_dir);
        let has_recording = wav::duration_from_size(&part).is_ok_and(|duration| duration > 0);
        if dictation_repo::get(conn, Slot::Incoming)?.is_some() || has_recording {
            return Err(AppError::confirmation_required(
                ConfirmationReason::ReplacePendingAudio,
            ));
        }
        if draft_repo::exists(conn)? {
            return Err(AppError::confirmation_required(
                ConfirmationReason::ReplaceDraft,
            ));
        }
    }

    let audio_files = dictation_repo::audio_files(conn)?;
    let built = build_from_text(conn, text)?;
    draft_repo::replace(conn, &built, text, TextOrigin::Typed)?;
    dictation_repo::delete(conn, Slot::Current)?;
    dictation_repo::delete(conn, Slot::Incoming)?;
    let draft = draft_repo::load(conn)?.ok_or_else(AppError::storage)?;
    Ok((build_view(conn, &draft)?, audio_files))
}

/// Genera el borrador a partir de un texto. Depende solo del texto, las tarifas y las unidades.
pub fn generate_from_text(
    state: &AppState,
    text: &str,
    mode: GenerateMode,
    confirmed: bool,
) -> Result<DraftView, AppError> {
    if state.dictation.activity() != Activity::Idle {
        return Err(AppError::invalid_state(BUSY));
    }
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::field("text", ErrorCode::TextRequired));
    }
    valid_text(text)?;

    let (view, obsolete_audio) = {
        let mut conn = state.conn()?;
        let tx = conn.transaction()?;
        let result = match mode {
            GenerateMode::Regenerate => (regenerate(&tx, text, confirmed)?, Vec::new()),
            GenerateMode::New => generate_new(state, &tx, text, confirmed)?,
        };
        tx.commit()?;
        result
    };

    if mode == GenerateMode::New {
        for audio_file in obsolete_audio {
            recording_store::remove_audio(&state.data_dir, &audio_file)?;
        }
        recording_store::remove_part(&state.data_dir)?;
    }
    Ok(view)
}

/// Guarda el texto mientras se edita, sin regenerar las líneas (FR-035).
pub fn save_text(state: &AppState, text: &str) -> Result<(), AppError> {
    valid_text(text)?;
    let conn = state.conn()?;
    match draft_repo::load(&conn)? {
        Some(draft) if draft.source_text != text => {
            draft_repo::set_source_text(&conn, text, true)?;
        }
        Some(_) => {}
        None if text.trim().is_empty() => draft_repo::clear_pending_text(&conn)?,
        None => draft_repo::save_pending_text(&conn, text)?,
    }
    Ok(())
}

/// Aplica una edición en una transacción. Si el resultado no es válido, no cambia nada.
fn edit<F>(state: &AppState, apply: F) -> Result<DraftView, AppError>
where
    F: FnOnce(&Connection) -> Result<(), AppError>,
{
    let mut conn = state.conn()?;
    let tx = conn.transaction()?;
    if !draft_repo::exists(&tx)? {
        return Err(AppError::invalid_state(NO_DRAFT));
    }
    apply(&tx)?;
    draft_repo::mark_edited(&tx)?;
    let draft = draft_repo::load(&tx)?.ok_or_else(AppError::storage)?;
    let view = build_view(&tx, &draft)?;
    tx.commit()?;
    Ok(view)
}

fn valid_concept(value: &str, errors: &mut Vec<FieldError>) -> Option<String> {
    let concept = collapse_spaces(value);
    if concept.is_empty() {
        errors.push(FieldError::new("concept", ErrorCode::ConceptRequired));
    } else if concept.chars().count() > MAX_CONCEPT_CHARS {
        errors.push(FieldError::new("concept", ErrorCode::TooLong));
    } else {
        return Some(concept);
    }
    None
}

fn valid_quantity(value: &str, errors: &mut Vec<FieldError>) -> Option<i64> {
    parse_quantity_centi(value)
        .map_err(|code| errors.push(FieldError::new("quantity", code)))
        .ok()
}

fn valid_unit(value: &str, errors: &mut Vec<FieldError>) -> Option<String> {
    let unit = collapse_spaces(value);
    if unit.is_empty() {
        errors.push(FieldError::new("unitLabel", ErrorCode::UnitRequired));
    } else if unit.chars().count() > MAX_UNIT_CHARS {
        errors.push(FieldError::new("unitLabel", ErrorCode::TooLong));
    } else {
        return Some(unit);
    }
    None
}

fn valid_price(value: &str, errors: &mut Vec<FieldError>) -> Option<i64> {
    parse_price_cents(value)
        .map_err(|code| errors.push(FieldError::new("price", code)))
        .ok()
}

/// Cambia la unidad. Una línea con precio de tarifa deja de valer: nunca se convierten unidades
/// y la tarifa queda como sugerencia (FR-021 y FR-024b).
fn apply_unit(conn: &Connection, built: &mut BuiltLine, unit: String) -> Result<(), AppError> {
    let line = &mut built.line;
    let catalog = UnitCatalog::new(&unit_repo::list(conn)?);
    let same_unit = line.unit_label.as_deref().is_some_and(|current| {
        catalog.canonical_of_label(current) == catalog.canonical_of_label(&unit)
    });
    if same_unit {
        return Ok(());
    }

    if line.price_source == PriceSource::Rate {
        built.suggestion_rate_ids = line.rate_id.into_iter().collect();
        line.price_cents = None;
        line.price_source = PriceSource::Pending;
        line.rate_id = None;
        line.rate_concept = None;
        line.pending_reason = Some(PendingReason::UnitMismatch);
    }
    line.unit_label = Some(unit);
    Ok(())
}

/// Un precio escrito a mano sustituye al de la tarifa (FR-032).
fn apply_manual_price(built: &mut BuiltLine, price_cents: i64) {
    built.suggestion_rate_ids.clear();
    let line = &mut built.line;
    line.price_cents = Some(price_cents);
    line.price_source = PriceSource::Manual;
    line.rate_id = None;
    line.rate_concept = None;
    line.pending_reason = None;
}

/// Edita una línea según las reglas de data-model.md.
pub fn line_update(
    state: &AppState,
    line_id: i64,
    patch: DraftLinePatch,
) -> Result<DraftView, AppError> {
    edit(state, |conn| {
        let mut built = draft_repo::get_line(conn, line_id)?
            .ok_or_else(|| AppError::not_found(LINE_NOT_FOUND))?;

        let mut errors = Vec::new();
        let concept = patch
            .concept
            .as_deref()
            .and_then(|v| valid_concept(v, &mut errors));
        let quantity = patch
            .quantity
            .as_deref()
            .and_then(|v| valid_quantity(v, &mut errors));
        let unit = patch
            .unit_label
            .as_deref()
            .and_then(|v| valid_unit(v, &mut errors));
        let price = patch
            .price
            .as_deref()
            .and_then(|v| valid_price(v, &mut errors));

        // El precio necesita una unidad a la que referirse (FR-026).
        let has_unit = unit.is_some() || built.line.unit_label.is_some();
        if price.is_some() && !has_unit && patch.unit_label.is_none() {
            errors.push(FieldError::new("unitLabel", ErrorCode::UnitRequired));
        }
        if !errors.is_empty() {
            return Err(AppError::validation(errors));
        }

        if let Some(concept) = concept {
            built.line.concept = concept;
        }
        if let Some(quantity) = quantity {
            built.line.quantity_centi = Some(quantity);
            built.line.quantity_issue = None;
        }
        if let Some(unit) = unit {
            apply_unit(conn, &mut built, unit)?;
        }
        if let Some(price) = price {
            apply_manual_price(&mut built, price);
        }

        draft_repo::update_line(conn, &built.line)?;
        draft_repo::replace_suggestions(conn, line_id, &built.suggestion_rate_ids)?;
        Ok(())
    })
}

/// Copia en la línea el precio, la unidad y el concepto de una tarifa.
fn apply_rate(line: &mut DraftLine, rate: &RateView) {
    line.concept = rate.concept.clone();
    line.unit_label = Some(rate.unit.label.clone());
    line.price_cents = Some(rate.price_cents);
    line.price_source = PriceSource::Rate;
    line.rate_id = Some(rate.id);
    line.rate_concept = Some(rate.concept.clone());
    line.pending_reason = None;
}

/// Completa una línea con una tarifa (FR-026).
pub fn line_assign_rate(
    state: &AppState,
    line_id: i64,
    rate_id: i64,
) -> Result<DraftView, AppError> {
    edit(state, |conn| {
        let mut built = draft_repo::get_line(conn, line_id)?
            .ok_or_else(|| AppError::not_found(LINE_NOT_FOUND))?;
        let rate =
            rate_repo::get(conn, rate_id)?.ok_or_else(|| AppError::not_found(RATE_NOT_FOUND))?;
        apply_rate(&mut built.line, &rate);
        draft_repo::update_line(conn, &built.line)?;
        draft_repo::replace_suggestions(conn, line_id, &[])?;
        Ok(())
    })
}

fn empty_line(position: i64, concept: String, quantity_centi: i64) -> DraftLine {
    DraftLine {
        id: 0,
        position,
        concept,
        quantity_centi: Some(quantity_centi),
        quantity_issue: None,
        unit_label: None,
        price_cents: None,
        price_source: PriceSource::Pending,
        rate_id: None,
        rate_concept: None,
        pending_reason: None,
        source_fragment: None,
    }
}

/// Añade una línea al final del borrador (FR-025).
pub fn line_add(state: &AppState, input: DraftLineInput) -> Result<DraftView, AppError> {
    edit(state, |conn| {
        let position = draft_repo::next_position(conn)?;
        let line = match input {
            DraftLineInput::Rate { rate_id, quantity } => {
                let quantity = parse_quantity_centi(&quantity)
                    .map_err(|code| AppError::field("quantity", code))?;
                let rate = rate_repo::get(conn, rate_id)?
                    .ok_or_else(|| AppError::not_found(RATE_NOT_FOUND))?;
                let mut line = empty_line(position, rate.concept.clone(), quantity);
                apply_rate(&mut line, &rate);
                line
            }
            DraftLineInput::Manual {
                concept,
                quantity,
                unit_label,
                price,
            } => {
                let mut errors = Vec::new();
                let concept = valid_concept(&concept, &mut errors);
                let quantity = valid_quantity(&quantity, &mut errors);
                let unit = valid_unit(&unit_label, &mut errors);
                let price = valid_price(&price, &mut errors);
                let (Some(concept), Some(quantity), Some(unit), Some(price)) =
                    (concept, quantity, unit, price)
                else {
                    return Err(AppError::validation(errors));
                };
                let mut line = empty_line(position, concept, quantity);
                line.unit_label = Some(unit);
                line.price_cents = Some(price);
                line.price_source = PriceSource::Manual;
                line
            }
        };
        draft_repo::insert_line(conn, &line, &[])?;
        Ok(())
    })
}

/// Borra una línea. La confirmación la hace la interfaz.
pub fn line_delete(state: &AppState, line_id: i64) -> Result<DraftView, AppError> {
    edit(state, |conn| {
        match draft_repo::delete_line(conn, line_id)? {
            0 => Err(AppError::not_found(LINE_NOT_FOUND)),
            _ => Ok(()),
        }
    })
}

/// Aplica el IVA actual del negocio, a petición del profesional (FR-029b).
pub fn apply_current_vat(state: &AppState) -> Result<DraftView, AppError> {
    let mut conn = state.conn()?;
    let tx = conn.transaction()?;
    if !draft_repo::exists(&tx)? {
        return Err(AppError::invalid_state(NO_DRAFT));
    }
    draft_repo::set_vat_rate(&tx, current_vat_rate_bp(&tx)?)?;
    draft_repo::set_status(&tx, DraftStatus::Draft)?;
    let draft = draft_repo::load(&tx)?.ok_or_else(AppError::storage)?;
    let view = build_view(&tx, &draft)?;
    tx.commit()?;
    Ok(view)
}

/// Da el borrador por bueno: exige al menos una línea y ninguna pendiente (FR-034).
pub fn mark_reviewed(state: &AppState) -> Result<DraftView, AppError> {
    let mut conn = state.conn()?;
    let tx = conn.transaction()?;
    let draft = draft_repo::load(&tx)?.ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
    if draft.lines.is_empty() {
        return Err(AppError::invalid_state(NO_LINES));
    }
    let pending: Vec<i64> = draft
        .lines
        .iter()
        .filter(|built| built.line.is_pending())
        .map(|built| built.line.id)
        .collect();
    if !pending.is_empty() {
        return Err(AppError::draft_has_pending(pending));
    }

    draft_repo::set_status(&tx, DraftStatus::Reviewed)?;
    let draft = draft_repo::load(&tx)?.ok_or_else(AppError::storage)?;
    let view = build_view(&tx, &draft)?;
    tx.commit()?;
    Ok(view)
}

/// Borra el borrador, su texto y su audio. La confirmación la hace la interfaz (FR-036).
pub fn discard(state: &AppState) -> Result<(), AppError> {
    if state.dictation.activity() != Activity::Idle {
        return Err(AppError::invalid_state(BUSY));
    }
    let audio_files = {
        let mut conn = state.conn()?;
        let tx = conn.transaction()?;
        let files = dictation_repo::audio_files(&tx)?;
        draft_repo::delete(&tx)?;
        draft_repo::clear_pending_text(&tx)?;
        dictation_repo::delete(&tx, Slot::Current)?;
        dictation_repo::delete(&tx, Slot::Incoming)?;
        tx.commit()?;
        files
    };
    for audio_file in audio_files {
        recording_store::remove_audio(&state.data_dir, &audio_file)?;
    }
    recording_store::remove_part(&state.data_dir)
}
