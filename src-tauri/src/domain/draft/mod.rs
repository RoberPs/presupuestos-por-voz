//! Borrador de presupuesto: tipos del dominio, sin dependencias de Tauri ni de audio.

pub mod builder;
pub mod matcher;
pub mod numbers;
pub mod parser;
pub mod totals;
pub mod units;

use serde::Serialize;

use crate::domain::money::line_amount_cents;
use crate::error::ErrorCode;

/// Origen del precio de una línea (FR-032).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PriceSource {
    Rate,
    Manual,
    Pending,
}

impl PriceSource {
    pub fn as_db(self) -> &'static str {
        match self {
            PriceSource::Rate => "rate",
            PriceSource::Manual => "manual",
            PriceSource::Pending => "pending",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "rate" => Some(PriceSource::Rate),
            "manual" => Some(PriceSource::Manual),
            "pending" => Some(PriceSource::Pending),
            _ => None,
        }
    }
}

/// Por qué una línea no tiene precio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PendingReason {
    NoRate,
    AmbiguousRate,
    SimilarRate,
    UnitMismatch,
}

impl PendingReason {
    pub fn as_db(self) -> &'static str {
        match self {
            PendingReason::NoRate => "no_rate",
            PendingReason::AmbiguousRate => "ambiguous_rate",
            PendingReason::SimilarRate => "similar_rate",
            PendingReason::UnitMismatch => "unit_mismatch",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "no_rate" => Some(PendingReason::NoRate),
            "ambiguous_rate" => Some(PendingReason::AmbiguousRate),
            "similar_rate" => Some(PendingReason::SimilarRate),
            "unit_mismatch" => Some(PendingReason::UnitMismatch),
            _ => None,
        }
    }
}

/// Por qué una línea no tiene cantidad (FR-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuantityIssue {
    Missing,
    Ambiguous,
    Rectified,
}

impl QuantityIssue {
    pub fn as_db(self) -> &'static str {
        match self {
            QuantityIssue::Missing => "missing",
            QuantityIssue::Ambiguous => "ambiguous",
            QuantityIssue::Rectified => "rectified",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "missing" => Some(QuantityIssue::Missing),
            "ambiguous" => Some(QuantityIssue::Ambiguous),
            "rectified" => Some(QuantityIssue::Rectified),
            _ => None,
        }
    }
}

/// Estado del borrador (FR-034).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DraftStatus {
    Draft,
    Reviewed,
}

impl DraftStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            DraftStatus::Draft => "draft",
            DraftStatus::Reviewed => "reviewed",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(DraftStatus::Draft),
            "reviewed" => Some(DraftStatus::Reviewed),
            _ => None,
        }
    }
}

/// De dónde procede el texto de la visita.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextOrigin {
    Transcribed,
    Typed,
}

impl TextOrigin {
    pub fn as_db(self) -> &'static str {
        match self {
            TextOrigin::Transcribed => "transcribed",
            TextOrigin::Typed => "typed",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "transcribed" => Some(TextOrigin::Transcribed),
            "typed" => Some(TextOrigin::Typed),
            _ => None,
        }
    }
}

/// Una línea con las tarifas que se le sugieren, en orden estable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltLine {
    pub line: DraftLine,
    pub suggestion_rate_ids: Vec<i64>,
}

/// Resultado de interpretar un texto: líneas y fragmentos sin interpretar (FR-022).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuiltDraft {
    pub lines: Vec<BuiltLine>,
    pub unparsed_fragments: Vec<String>,
    pub vat_rate_bp: i64,
}

/// Borrador guardado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDraft {
    pub status: DraftStatus,
    pub source_text: String,
    pub text_origin: TextOrigin,
    pub text_corrected: bool,
    pub vat_rate_bp: i64,
    pub has_manual_changes: bool,
    pub lines: Vec<BuiltLine>,
    pub unparsed_fragments: Vec<String>,
}

/// Una línea del borrador. El precio solo procede de una tarifa o de una entrada manual
/// (Principio IV de la constitución).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftLine {
    pub id: i64,
    pub position: i64,
    pub concept: String,
    pub quantity_centi: Option<i64>,
    pub quantity_issue: Option<QuantityIssue>,
    pub unit_label: Option<String>,
    pub price_cents: Option<i64>,
    pub price_source: PriceSource,
    pub rate_id: Option<i64>,
    pub rate_concept: Option<String>,
    pub pending_reason: Option<PendingReason>,
    pub source_fragment: Option<String>,
}

impl DraftLine {
    /// Le falta el precio o la cantidad, así que queda fuera del total (FR-031).
    pub fn is_pending(&self) -> bool {
        self.price_cents.is_none() || self.quantity_centi.is_none()
    }

    /// Importe de la línea, si está completa.
    pub fn amount_cents(&self) -> Result<Option<i64>, ErrorCode> {
        match (self.quantity_centi, self.price_cents) {
            (Some(quantity), Some(price)) => line_amount_cents(quantity, price).map(Some),
            _ => Ok(None),
        }
    }
}
