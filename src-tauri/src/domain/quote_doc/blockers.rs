//! Condiciones para generar el PDF y avisos de la confirmación (data-model.md).

use serde::Serialize;

use crate::domain::draft::{DraftLine, DraftStatus};
use crate::domain::money::format_quantity;
use crate::error::{FieldError, PendingLineRef, QuoteBlocker, UnsupportedField};

/// Texto que se va a imprimir en el documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedText<'a> {
    pub field: &'a str,
    pub label: &'a str,
    pub text: &'a str,
}

/// Todo lo que decide si se puede generar el PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockerInput<'a> {
    pub lines: &'a [DraftLine],
    pub status: DraftStatus,
    pub missing_profile_fields: &'a [FieldError],
    pub client_name: &'a str,
    pub quote_number: &'a str,
    pub texts: &'a [CheckedText<'a>],
    pub logo_unreadable: bool,
}

fn unsupported_characters(text: &str, supports: &dyn Fn(char) -> bool) -> Vec<String> {
    let mut characters: Vec<String> = Vec::new();
    for character in text.chars() {
        if character.is_whitespace() || supports(character) {
            continue;
        }
        let character = character.to_string();
        if !characters.contains(&character) {
            characters.push(character);
        }
    }
    characters
}

/// Motivos por los que todavía no se puede generar el PDF, todos a la vez. `supports` indica si la
/// tipografía del documento puede mostrar un carácter (FR-018b).
pub fn evaluate(input: &BlockerInput<'_>, supports: &dyn Fn(char) -> bool) -> Vec<QuoteBlocker> {
    let mut blockers = Vec::new();

    if input.lines.is_empty() {
        blockers.push(QuoteBlocker::NoLines);
    }
    let pending: Vec<PendingLineRef> = input
        .lines
        .iter()
        .filter(|line| line.is_pending())
        .map(|line| PendingLineRef {
            id: line.id,
            concept: line.concept.clone(),
        })
        .collect();
    if !pending.is_empty() {
        blockers.push(QuoteBlocker::PendingLines { lines: pending });
    }
    if input.status != DraftStatus::Reviewed {
        blockers.push(QuoteBlocker::NotReviewed);
    }
    if !input.missing_profile_fields.is_empty() {
        blockers.push(QuoteBlocker::ProfileIncomplete {
            missing_fields: input.missing_profile_fields.to_vec(),
        });
    }
    if input.client_name.trim().is_empty() {
        blockers.push(QuoteBlocker::ClientNameRequired);
    }
    if input.quote_number.trim().is_empty() {
        blockers.push(QuoteBlocker::QuoteNumberRequired);
    }

    let fields: Vec<UnsupportedField> = input
        .texts
        .iter()
        .filter_map(|text| {
            let characters = unsupported_characters(text.text, supports);
            (!characters.is_empty()).then(|| UnsupportedField {
                field: text.field.to_string(),
                label: text.label.to_string(),
                characters,
            })
        })
        .collect();
    if !fields.is_empty() {
        blockers.push(QuoteBlocker::UnsupportedCharacters { fields });
    }

    if input.logo_unreadable {
        blockers.push(QuoteBlocker::LogoUnreadable);
    }
    blockers
}

/// Aviso de la confirmación que no impide generar (FR-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuoteWarning {
    NoLogo,
    LogoUnreadable,
    VatChanged {
        applied_rate_percent: String,
        current_rate_percent: String,
    },
    UnparsedFragments {
        count: usize,
    },
}

/// Datos de los que dependen los avisos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarningInput {
    pub has_logo: bool,
    pub accepted_unreadable_logo: bool,
    pub applied_vat_bp: i64,
    pub current_vat_bp: i64,
    pub unparsed_fragments: usize,
}

pub fn warnings(input: &WarningInput) -> Vec<QuoteWarning> {
    let mut warnings = Vec::new();
    if input.accepted_unreadable_logo {
        warnings.push(QuoteWarning::LogoUnreadable);
    } else if !input.has_logo {
        warnings.push(QuoteWarning::NoLogo);
    }
    if input.applied_vat_bp != input.current_vat_bp {
        warnings.push(QuoteWarning::VatChanged {
            applied_rate_percent: format_quantity(input.applied_vat_bp),
            current_rate_percent: format_quantity(input.current_vat_bp),
        });
    }
    if input.unparsed_fragments > 0 {
        warnings.push(QuoteWarning::UnparsedFragments {
            count: input.unparsed_fragments,
        });
    }
    warnings
}
