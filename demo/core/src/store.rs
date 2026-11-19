//! Casos de uso de la demo sobre datos en memoria.
//!
//! Reproduce el comportamiento de `src-tauri/src/services` (mismas reglas, mensajes y orden de
//! validación), pero sustituye SQLite por estructuras en memoria. Todo el cálculo lo hacen los
//! módulos compartidos con la app: `domain` y `pdf`.

use std::collections::BTreeMap;

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::Date;

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
use crate::domain::normalize::{collapse_spaces, normalize};
use crate::domain::profile::{
    self, Address, BusinessProfile, BusinessProfileInput, BusinessProfileView, LogoMeta,
    LogoSetResult, LogoView, DEFAULT_VAT_BP,
};
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
use crate::domain::rate::{self, RateInput, RateView, UnitView, MAX_CONCEPT_CHARS, MAX_UNIT_CHARS};
use crate::error::{
    AppError, ConfirmationReason, ErrorCode, FieldError, LogoRejectCode, QuoteBlocker,
};
use crate::pdf::fonts::PdfFonts;
use crate::pdf::logo::{normalize as normalize_logo, NormalizedLogo};
use crate::pdf::renderer::{render_document, RenderedPdf};

// Mismos textos que los servicios de la app.
const LINE_NOT_FOUND: &str = "Esa línea ya no existe.";
const RATE_NOT_FOUND: &str = "Esa tarifa ya no existe. Puede que se haya borrado.";
const NO_DRAFT: &str = "Todavía no hay ningún presupuesto en curso.";
const NO_LINES: &str = "Añade al menos una línea antes de dar el presupuesto por bueno.";
const MAX_TEXT_CHARS: usize = 20_000;
const MAX_LOGO_BYTES: usize = 5 * 1024 * 1024;
const LOW_RESOLUTION_WARNING: &str =
    "El logo es pequeño y puede verse borroso en los presupuestos.";

/// Texto de ejemplo que la demo deja escrito en «Escribir la visita».
pub const EXAMPLE_TEXT: &str = "cambiar bajante, 3 metros, grifo monomando 1 unidad, \
mano de obra 2 horas, revisar caldera";

// --- Vistas: espejo de src/shared/ipc/types.ts -------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerateMode {
    New,
    Regenerate,
}

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

/// En la demo no hay dictado: la actividad siempre es `idle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActivityView {
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub activity: ActivityView,
    pub dictation: Option<()>,
    pub draft: Option<DraftView>,
    pub pending_text: Option<String>,
    pub recovery: Option<()>,
}

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

/// PDF listo para descargar.
pub struct GeneratedPdf {
    pub file_name: String,
    pub rendered: RenderedPdf,
}

// --- Almacenamiento en memoria -----------------------------------------------------------------

struct StoredUnit {
    view: UnitView,
    norm: String,
}

struct StoredRate {
    id: i64,
    concept: String,
    concept_norm: String,
    unit_id: i64,
    price_cents: i64,
}

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

pub struct Store {
    profile: Option<BusinessProfile>,
    logo_bytes: Option<Vec<u8>>,
    units: Vec<StoredUnit>,
    rates: Vec<StoredRate>,
    next_unit_id: i64,
    next_rate_id: i64,
    next_line_id: i64,
    draft: Option<StoredDraft>,
    pending_text: Option<String>,
    details: Option<QuoteDetails>,
    sequences: BTreeMap<i32, i64>,
}

fn amount_error(code: ErrorCode) -> AppError {
    AppError::field("price", code)
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    /// Datos vacíos, con las unidades predefinidas de la migración 001.
    pub fn new() -> Self {
        let units = ["unidad", "hora", "metro", "m²", "metro lineal"]
            .iter()
            .enumerate()
            .map(|(index, label)| StoredUnit {
                view: UnitView {
                    id: index as i64 + 1,
                    label: (*label).to_string(),
                    is_predefined: true,
                },
                norm: (*label).to_string(),
            })
            .collect();
        Store {
            profile: None,
            logo_bytes: None,
            units,
            rates: Vec::new(),
            next_unit_id: 6,
            next_rate_id: 1,
            next_line_id: 1,
            draft: None,
            pending_text: None,
            details: None,
            sequences: BTreeMap::new(),
        }
    }

    /// Negocio ficticio, tarifas de ejemplo y un texto de visita listo para probar.
    pub fn demo() -> Self {
        let mut store = Store::new();
        store
            .profile_save(BusinessProfileInput {
                trade_name: "Fontanería Ejemplo".into(),
                legal_name: Some("Negocio de demostración".into()),
                tax_id: "12345678Z".into(),
                phone: "600 123 456".into(),
                email: Some("hola@ejemplo.es".into()),
                website: None,
                address: Address {
                    street: "Calle Mayor 1".into(),
                    postal_code: "28001".into(),
                    city: "Madrid".into(),
                    province: "Madrid".into(),
                },
                vat_rate_percent: "21".into(),
            })
            .expect("perfil de la demo");
        for (concept, unit, price) in [
            ("Bajante", "metro", "25"),
            ("Mano de obra", "hora", "30"),
            ("Grifo monomando", "unidad", "45"),
            ("Codo de PVC", "unidad", "3,50"),
            ("Pintura plástica", "m²", "8,50"),
            ("Desplazamiento", "unidad", "20"),
        ] {
            store
                .rates_create(RateInput {
                    concept: concept.into(),
                    unit_label: unit.into(),
                    price: price.into(),
                })
                .expect("tarifa de la demo");
        }
        store.pending_text = Some(EXAMPLE_TEXT.to_string());
        store
    }

    // --- Perfil y logo ---

    fn logo_view(&self, meta: &LogoMeta) -> Option<LogoView> {
        let bytes = self.logo_bytes.as_ref()?;
        Some(LogoView {
            data_url: format!(
                "data:{};base64,{}",
                meta.mime,
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ),
            mime: meta.mime.clone(),
            width_px: meta.width_px,
            height_px: meta.height_px,
            low_resolution: meta.low_resolution(),
        })
    }

    fn profile_view(&self, p: &BusinessProfile) -> BusinessProfileView {
        let missing = profile::missing_fields(p);
        BusinessProfileView {
            trade_name: p.trade_name.clone(),
            legal_name: p.legal_name.clone(),
            tax_id: p.tax_id.clone(),
            phone: p.phone.clone(),
            email: p.email.clone(),
            website: p.website.clone(),
            address: p.address.clone(),
            vat_rate_percent: format_percent_bp(p.vat_rate_bp),
            currency: "EUR",
            logo: p.logo.as_ref().and_then(|meta| self.logo_view(meta)),
            status: profile::status(&missing),
            missing_fields: missing,
        }
    }

    pub fn profile_get(&self) -> Option<BusinessProfileView> {
        self.profile.as_ref().map(|p| self.profile_view(p))
    }

    /// Guarda aunque falten datos. Conserva el logo, como `profile_repo::upsert`.
    pub fn profile_save(
        &mut self,
        input: BusinessProfileInput,
    ) -> Result<BusinessProfileView, AppError> {
        let hard = profile::hard_errors(&input);
        if !hard.is_empty() {
            return Err(AppError::validation(hard));
        }
        let mut data = profile::from_input(&input);
        data.logo = self.profile.as_ref().and_then(|p| p.logo.clone());
        self.profile = Some(data);
        Ok(self.profile_get().expect("perfil guardado"))
    }

    pub fn profile_validate(input: &BusinessProfileInput) -> Vec<FieldError> {
        let mut errors = profile::hard_errors(input);
        for e in profile::missing_fields(&profile::from_input(input)) {
            if !errors.iter().any(|x| x.field == e.field) {
                errors.push(e);
            }
        }
        errors
    }

    /// Fila vacía del perfil, como `profile_repo::ensure_row`.
    fn ensure_profile(&mut self) -> &mut BusinessProfile {
        self.profile.get_or_insert_with(|| BusinessProfile {
            trade_name: String::new(),
            legal_name: None,
            tax_id: String::new(),
            phone: String::new(),
            email: None,
            website: None,
            address: Address::default(),
            vat_rate_bp: DEFAULT_VAT_BP,
            logo: None,
        })
    }

    /// Valida el logo como `logo_service::validate_logo_file` y lo sustituye.
    pub fn logo_set(&mut self, bytes: Vec<u8>) -> Result<LogoSetResult, AppError> {
        if bytes.len() > MAX_LOGO_BYTES {
            return Err(AppError::logo_rejected(LogoRejectCode::LogoTooLarge));
        }
        let (format, mime) = match image::guess_format(&bytes) {
            Ok(image::ImageFormat::Png) => (image::ImageFormat::Png, "image/png"),
            Ok(image::ImageFormat::Jpeg) => (image::ImageFormat::Jpeg, "image/jpeg"),
            _ => {
                return Err(AppError::logo_rejected(
                    LogoRejectCode::LogoUnsupportedFormat,
                ))
            }
        };
        let img = image::load_from_memory_with_format(&bytes, format)
            .map_err(|_| AppError::logo_rejected(LogoRejectCode::LogoCorrupt))?;
        let meta = LogoMeta {
            file: "demo-logo".into(),
            mime: mime.into(),
            width_px: img.width(),
            height_px: img.height(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
        };
        let warnings = if meta.low_resolution() {
            vec![LOW_RESOLUTION_WARNING.to_string()]
        } else {
            vec![]
        };
        self.logo_bytes = Some(bytes);
        self.ensure_profile().logo = Some(meta.clone());
        Ok(LogoSetResult {
            logo: self.logo_view(&meta).expect("logo guardado"),
            warnings,
        })
    }

    pub fn logo_remove(&mut self) -> BusinessProfileView {
        self.ensure_profile().logo = None;
        self.logo_bytes = None;
        self.profile_get().expect("perfil")
    }

    // --- Unidades y tarifas ---

    pub fn units_list(&self) -> Vec<UnitView> {
        let mut predefined: Vec<&StoredUnit> =
            self.units.iter().filter(|u| u.view.is_predefined).collect();
        predefined.sort_by_key(|u| u.view.id);
        let mut custom: Vec<&StoredUnit> =
            self.units.iter().filter(|u| !u.view.is_predefined).collect();
        custom.sort_by(|a, b| (&a.norm, &a.view.label).cmp(&(&b.norm, &b.view.label)));
        predefined
            .into_iter()
            .chain(custom)
            .map(|u| u.view.clone())
            .collect()
    }

    fn rate_view(&self, rate: &StoredRate) -> RateView {
        let unit = self
            .units
            .iter()
            .find(|u| u.view.id == rate.unit_id)
            .map(|u| u.view.clone())
            .expect("unidad de la tarifa");
        RateView::new(rate.id, rate.concept.clone(), unit, rate.price_cents)
    }

    fn rate_get(&self, id: i64) -> Option<RateView> {
        self.rates
            .iter()
            .find(|r| r.id == id)
            .map(|r| self.rate_view(r))
    }

    fn rates_all(&self) -> Vec<RateView> {
        let mut rates: Vec<&StoredRate> = self.rates.iter().collect();
        rates.sort_by(|a, b| (&a.concept_norm, &a.concept).cmp(&(&b.concept_norm, &b.concept)));
        rates.into_iter().map(|r| self.rate_view(r)).collect()
    }

    pub fn rates_list(&self, query: Option<String>) -> Vec<RateView> {
        let q = query.as_deref().map(normalize).unwrap_or_default();
        self.rates_all()
            .into_iter()
            .filter(|r| q.is_empty() || normalize(&r.concept).contains(&q))
            .collect()
    }

    fn rate_write(&mut self, id: Option<i64>, input: RateInput) -> Result<RateView, AppError> {
        let v = rate::validate(&input).map_err(AppError::validation)?;
        if let Some(id) = id {
            if self.rate_get(id).is_none() {
                return Err(AppError::not_found(RATE_NOT_FOUND));
            }
        }
        let unit_id = match self.units.iter().find(|u| u.norm == v.unit_norm) {
            Some(unit) => unit.view.id,
            None => {
                let unit_id = self.next_unit_id;
                self.next_unit_id += 1;
                self.units.push(StoredUnit {
                    view: UnitView {
                        id: unit_id,
                        label: v.unit_label.clone(),
                        is_predefined: false,
                    },
                    norm: v.unit_norm.clone(),
                });
                unit_id
            }
        };
        let duplicate = self.rates.iter().find(|r| {
            r.concept_norm == v.concept_norm && r.unit_id == unit_id && Some(r.id) != id
        });
        if let Some(existing) = duplicate {
            let view = self.rate_view(existing);
            return Err(AppError::DuplicateRate {
                existing_rate_id: existing.id,
                message: format!(
                    "Ya existe la tarifa «{}» por {}. Puedes editarla en lugar de crear otra.",
                    view.concept, view.unit.label
                ),
            });
        }

        let rate_id = match id {
            Some(id) => {
                let rate = self.rates.iter_mut().find(|r| r.id == id).expect("tarifa");
                rate.concept = v.concept;
                rate.concept_norm = v.concept_norm;
                rate.unit_id = unit_id;
                rate.price_cents = v.price_cents;
                id
            }
            None => {
                let id = self.next_rate_id;
                self.next_rate_id += 1;
                self.rates.push(StoredRate {
                    id,
                    concept: v.concept,
                    concept_norm: v.concept_norm,
                    unit_id,
                    price_cents: v.price_cents,
                });
                id
            }
        };
        Ok(self.rate_get(rate_id).expect("tarifa guardada"))
    }

    pub fn rates_create(&mut self, input: RateInput) -> Result<RateView, AppError> {
        self.rate_write(None, input)
    }

    pub fn rates_update(&mut self, id: i64, input: RateInput) -> Result<RateView, AppError> {
        self.rate_write(Some(id), input)
    }

    /// Borra la tarifa. Como en SQLite: `rate_id` pasa a nulo y desaparece de las sugerencias.
    pub fn rates_delete(&mut self, id: i64) -> Result<(), AppError> {
        let before = self.rates.len();
        self.rates.retain(|r| r.id != id);
        if self.rates.len() == before {
            return Err(AppError::not_found(RATE_NOT_FOUND));
        }
        if let Some(draft) = &mut self.draft {
            for built in &mut draft.lines {
                if built.line.rate_id == Some(id) {
                    built.line.rate_id = None;
                }
                built.suggestion_rate_ids.retain(|rate_id| *rate_id != id);
            }
        }
        Ok(())
    }

    // --- Borrador ---

    fn current_vat_bp(&self) -> i64 {
        self.profile.as_ref().map_or(DEFAULT_VAT_BP, |p| p.vat_rate_bp)
    }

    fn line_view(&self, built: &BuiltLine) -> Result<DraftLineView, AppError> {
        let line = &built.line;
        let amount_cents = line.amount_cents().map_err(amount_error)?;
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
            suggestions: built
                .suggestion_rate_ids
                .iter()
                .filter_map(|id| self.rate_get(*id))
                .collect(),
            source_fragment: line.source_fragment.clone(),
        })
    }

    fn build_view(&self, draft: &StoredDraft) -> Result<DraftView, AppError> {
        let lines: Vec<DraftLine> = draft.lines.iter().map(|l| l.line.clone()).collect();
        let totals = compute_totals(&lines, draft.vat_rate_bp).map_err(amount_error)?;
        let current_vat = self.current_vat_bp();
        Ok(DraftView {
            status: draft.status,
            source_text: draft.source_text.clone(),
            text_origin: draft.text_origin,
            text_corrected: draft.text_corrected,
            has_manual_changes: draft.has_manual_changes,
            lines: draft
                .lines
                .iter()
                .map(|line| self.line_view(line))
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
            vat_change: (current_vat != draft.vat_rate_bp).then(|| VatChangeView {
                current_rate_percent: format_percent_bp(current_vat),
            }),
        })
    }

    fn draft_view(&self) -> Result<Option<DraftView>, AppError> {
        self.draft.as_ref().map(|d| self.build_view(d)).transpose()
    }

    pub fn session(&self) -> Result<SessionView, AppError> {
        Ok(SessionView {
            activity: ActivityView::Idle,
            dictation: None,
            draft: self.draft_view()?,
            pending_text: self.pending_text.clone(),
            recovery: None,
        })
    }

    fn build_from_text(&self, text: &str) -> BuiltDraft {
        let units: Vec<UnitView> = self.units_list();
        build_draft(text, &self.rates_all(), &units, self.current_vat_bp())
    }

    /// Sustituye el borrador y vacía el texto pendiente, como `draft_repo::replace`.
    fn replace_draft(&mut self, built: BuiltDraft, text: &str, origin: TextOrigin) {
        let lines = built
            .lines
            .into_iter()
            .map(|mut built_line| {
                built_line.line.id = self.next_line_id;
                self.next_line_id += 1;
                built_line
            })
            .collect();
        self.draft = Some(StoredDraft {
            status: DraftStatus::Draft,
            source_text: text.to_string(),
            text_origin: origin,
            text_corrected: false,
            vat_rate_bp: built.vat_rate_bp,
            has_manual_changes: false,
            lines,
            unparsed_fragments: built.unparsed_fragments,
        });
        self.pending_text = None;
    }

    pub fn generate_from_text(
        &mut self,
        text: &str,
        mode: GenerateMode,
        confirmed: bool,
    ) -> Result<DraftView, AppError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(AppError::field("text", ErrorCode::TextRequired));
        }
        if text.chars().count() > MAX_TEXT_CHARS {
            return Err(AppError::field("text", ErrorCode::TooLong));
        }
        match mode {
            GenerateMode::Regenerate => {
                let previous = self
                    .draft
                    .clone()
                    .ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
                if previous.has_manual_changes && !confirmed {
                    return Err(AppError::confirmation_required(
                        ConfirmationReason::DiscardManualChanges,
                    ));
                }
                let built = self.build_from_text(text);
                // Regenerar conserva los datos del presupuesto (R10).
                self.replace_draft(built, text, previous.text_origin);
                let draft = self.draft.as_mut().expect("borrador");
                draft.text_corrected = previous.text_corrected || previous.source_text != text;
            }
            GenerateMode::New => {
                if !confirmed && self.draft.is_some() {
                    return Err(AppError::confirmation_required(
                        ConfirmationReason::ReplaceDraft,
                    ));
                }
                let built = self.build_from_text(text);
                // Un borrador nuevo borra en cascada los datos del presupuesto anterior.
                self.details = None;
                self.replace_draft(built, text, TextOrigin::Typed);
            }
        }
        Ok(self.draft_view()?.expect("borrador"))
    }

    pub fn save_text(&mut self, text: &str) -> Result<(), AppError> {
        if text.chars().count() > MAX_TEXT_CHARS {
            return Err(AppError::field("text", ErrorCode::TooLong));
        }
        match &mut self.draft {
            Some(draft) if draft.source_text != text => {
                draft.source_text = text.to_string();
                draft.text_corrected = true;
            }
            Some(_) => {}
            None if text.trim().is_empty() => self.pending_text = None,
            None => self.pending_text = Some(text.to_string()),
        }
        Ok(())
    }

    /// Aplica una edición sobre una copia: si falla, el borrador no cambia (como la transacción).
    fn edit<F>(&mut self, apply: F) -> Result<DraftView, AppError>
    where
        F: FnOnce(&Store, &mut StoredDraft) -> Result<(), AppError>,
    {
        let mut draft = self
            .draft
            .clone()
            .ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
        apply(self, &mut draft)?;
        draft.has_manual_changes = true;
        draft.status = DraftStatus::Draft;
        let view = self.build_view(&draft)?;
        self.draft = Some(draft);
        Ok(view)
    }

    pub fn line_update(&mut self, line_id: i64, patch: DraftLinePatch) -> Result<DraftView, AppError> {
        self.edit(|store, draft| {
            let built = draft
                .lines
                .iter_mut()
                .find(|b| b.line.id == line_id)
                .ok_or_else(|| AppError::not_found(LINE_NOT_FOUND))?;

            let mut errors = Vec::new();
            let concept = patch.concept.as_deref().and_then(|v| valid_concept(v, &mut errors));
            let quantity = patch.quantity.as_deref().and_then(|v| valid_quantity(v, &mut errors));
            let unit = patch.unit_label.as_deref().and_then(|v| valid_unit(v, &mut errors));
            let price = patch.price.as_deref().and_then(|v| valid_price(v, &mut errors));

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
                apply_unit(&store.units_list(), built, unit);
            }
            if let Some(price) = price {
                apply_manual_price(built, price);
            }
            Ok(())
        })
    }

    pub fn line_assign_rate(&mut self, line_id: i64, rate_id: i64) -> Result<DraftView, AppError> {
        self.edit(|store, draft| {
            let built = draft
                .lines
                .iter_mut()
                .find(|b| b.line.id == line_id)
                .ok_or_else(|| AppError::not_found(LINE_NOT_FOUND))?;
            let rate = store
                .rate_get(rate_id)
                .ok_or_else(|| AppError::not_found(RATE_NOT_FOUND))?;
            apply_rate(&mut built.line, &rate);
            built.suggestion_rate_ids.clear();
            Ok(())
        })
    }

    pub fn line_add(&mut self, input: DraftLineInput) -> Result<DraftView, AppError> {
        let id = self.next_line_id;
        let view = self.edit(|store, draft| {
            let position = draft
                .lines
                .iter()
                .map(|b| b.line.position + 1)
                .max()
                .unwrap_or(0);
            let line = match input {
                DraftLineInput::Rate { rate_id, quantity } => {
                    let quantity = parse_quantity_centi(&quantity)
                        .map_err(|code| AppError::field("quantity", code))?;
                    let rate = store
                        .rate_get(rate_id)
                        .ok_or_else(|| AppError::not_found(RATE_NOT_FOUND))?;
                    let mut line = empty_line(id, position, rate.concept.clone(), quantity);
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
                    let mut line = empty_line(id, position, concept, quantity);
                    line.unit_label = Some(unit);
                    line.price_cents = Some(price);
                    line.price_source = PriceSource::Manual;
                    line
                }
            };
            draft.lines.push(BuiltLine {
                line,
                suggestion_rate_ids: Vec::new(),
            });
            Ok(())
        })?;
        self.next_line_id += 1;
        Ok(view)
    }

    pub fn line_delete(&mut self, line_id: i64) -> Result<DraftView, AppError> {
        self.edit(|_, draft| {
            let before = draft.lines.len();
            draft.lines.retain(|b| b.line.id != line_id);
            if draft.lines.len() == before {
                return Err(AppError::not_found(LINE_NOT_FOUND));
            }
            Ok(())
        })
    }

    pub fn apply_current_vat(&mut self) -> Result<DraftView, AppError> {
        let vat = self.current_vat_bp();
        let draft = self
            .draft
            .as_mut()
            .ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
        draft.vat_rate_bp = vat;
        draft.status = DraftStatus::Draft;
        Ok(self.draft_view()?.expect("borrador"))
    }

    pub fn mark_reviewed(&mut self) -> Result<DraftView, AppError> {
        let draft = self
            .draft
            .as_mut()
            .ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
        if draft.lines.is_empty() {
            return Err(AppError::invalid_state(NO_LINES));
        }
        let pending: Vec<i64> = draft
            .lines
            .iter()
            .filter(|b| b.line.is_pending())
            .map(|b| b.line.id)
            .collect();
        if !pending.is_empty() {
            return Err(AppError::draft_has_pending(pending));
        }
        draft.status = DraftStatus::Reviewed;
        Ok(self.draft_view()?.expect("borrador"))
    }

    pub fn discard(&mut self) -> Result<SessionView, AppError> {
        self.draft = None;
        self.pending_text = None;
        self.details = None;
        self.session()
    }

    // --- PDF del presupuesto ---

    fn automatic_number(&self, details: &QuoteDetails, current_hash: &str, year: i32) -> String {
        propose(&Proposal {
            manual: None,
            issued_number: details.issued_number.as_deref(),
            issued_hash: details.issued_content_hash.as_deref(),
            current_hash,
            year,
            last_number: self.sequences.get(&year).copied().unwrap_or(0),
        })
    }

    fn assemble(&self, today: Date, allow_missing_logo: bool) -> Result<Assembled, AppError> {
        let draft = self
            .draft
            .as_ref()
            .ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
        let details = self.details.clone().unwrap_or_default();
        let profile = self.profile.as_ref();

        let lines: Vec<DraftLine> = draft.lines.iter().map(|b| b.line.clone()).collect();
        let totals = compute_totals(&lines, draft.vat_rate_bp).map_err(amount_error)?;
        let hash = draft_content_hash(&lines, draft.vat_rate_bp);
        let number = match &details.quote_number {
            Some(manual) => manual.clone(),
            None => self.automatic_number(&details, &hash, today.year()),
        };

        let logo_bytes = profile
            .and_then(|p| p.logo.as_ref())
            .and(self.logo_bytes.as_deref());
        let logo = logo_bytes.and_then(|bytes| normalize_logo(bytes).ok());
        let logo_configured = profile.is_some_and(|p| p.logo.is_some());
        let logo_unreadable = logo_configured && logo.is_none();

        let document = QuoteDocument {
            number: number.clone(),
            issue_date: format_date(today),
            valid_until: format_date(valid_until(today, details.validity_days)),
            business: document_business(profile),
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
        let missing = missing_profile_fields(profile);
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
            current_vat_bp: self.current_vat_bp(),
            unparsed_fragments: draft.unparsed_fragments.len(),
        });
        let token = confirmation_token(&document, logo.as_ref().and(logo_bytes));

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

    pub fn quote_prepare(
        &self,
        today: Date,
        allow_missing_logo: bool,
    ) -> Result<QuotePreparationView, AppError> {
        self.assemble(today, allow_missing_logo).map(preparation_view)
    }

    pub fn quote_details_save(
        &mut self,
        input: QuoteDetailsInput,
        today: Date,
        allow_missing_logo: bool,
    ) -> Result<QuotePreparationView, AppError> {
        let clean = validate(&input).map_err(AppError::validation)?;
        let draft = self
            .draft
            .as_ref()
            .ok_or_else(|| AppError::invalid_state(NO_DRAFT))?;
        let previous = self.details.clone().unwrap_or_default();
        let lines: Vec<DraftLine> = draft.lines.iter().map(|b| b.line.clone()).collect();
        let hash = draft_content_hash(&lines, draft.vat_rate_bp);
        let automatic = self.automatic_number(&previous, &hash, today.year());
        let quote_number = (clean.quote_number != automatic).then_some(clean.quote_number);

        let saved = QuoteDetails {
            client_name: clean.client_name,
            client_address: clean.client_address,
            client_phone: clean.client_phone,
            client_email: clean.client_email,
            client_tax_id: clean.client_tax_id,
            quote_number,
            issued_number: previous.issued_number,
            issued_content_hash: previous.issued_content_hash,
            validity_days: clean.validity_days,
        };
        let backup = self.details.replace(saved);
        match self.assemble(today, allow_missing_logo) {
            Ok(assembled) => Ok(preparation_view(assembled)),
            Err(e) => {
                self.details = backup;
                Err(e)
            }
        }
    }

    /// Compone el PDF y gasta el número. En la demo el archivo se descarga en el navegador.
    pub fn quote_generate(
        &mut self,
        token: &str,
        today: Date,
        allow_missing_logo: bool,
    ) -> Result<GeneratedPdf, AppError> {
        let assembled = self.assemble(today, allow_missing_logo)?;
        if !assembled.blockers.is_empty() {
            return Err(AppError::quote_blocked(assembled.blockers.clone()));
        }
        if assembled.token != token {
            return Err(AppError::quote_changed());
        }
        let rendered = render_document(&assembled.document, today, assembled.logo.as_ref())?;

        if let Some((year, number)) = parse_series_number(&assembled.number) {
            let last = self.sequences.entry(year).or_insert(0);
            *last = (*last).max(number);
        }
        let file_name = format!(
            "Presupuesto {} {}.pdf",
            safe_file_part(&assembled.number),
            format_iso(today)
        );
        self.details = Some(QuoteDetails {
            quote_number: None,
            issued_number: Some(assembled.number),
            issued_content_hash: Some(assembled.content_hash),
            ..assembled.details
        });
        Ok(GeneratedPdf {
            file_name,
            rendered,
        })
    }
}

// --- Funciones auxiliares, iguales a las de los servicios de la app -----------------------------

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

fn apply_unit(units: &[UnitView], built: &mut BuiltLine, unit: String) {
    let line = &mut built.line;
    let catalog = UnitCatalog::new(units);
    let same_unit = line.unit_label.as_deref().is_some_and(|current| {
        catalog.canonical_of_label(current) == catalog.canonical_of_label(&unit)
    });
    if same_unit {
        return;
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
}

fn apply_manual_price(built: &mut BuiltLine, price_cents: i64) {
    built.suggestion_rate_ids.clear();
    let line = &mut built.line;
    line.price_cents = Some(price_cents);
    line.price_source = PriceSource::Manual;
    line.rate_id = None;
    line.rate_concept = None;
    line.pending_reason = None;
}

fn apply_rate(line: &mut DraftLine, rate: &RateView) {
    line.concept = rate.concept.clone();
    line.unit_label = Some(rate.unit.label.clone());
    line.price_cents = Some(rate.price_cents);
    line.price_source = PriceSource::Rate;
    line.rate_id = Some(rate.id);
    line.rate_concept = Some(rate.concept.clone());
    line.pending_reason = None;
}

fn empty_line(id: i64, position: i64, concept: String, quantity_centi: i64) -> DraftLine {
    DraftLine {
        id,
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
