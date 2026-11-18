//! Genera el borrador completo a partir del texto. Es una función pura y determinista (FR-014).

use crate::domain::draft::matcher::{RateIndex, RateMatch};
use crate::domain::draft::parser::{parse_text, ParsedJob};
use crate::domain::draft::units::UnitCatalog;
use crate::domain::draft::{BuiltDraft, BuiltLine, DraftLine, PriceSource};
use crate::domain::rate::{RateView, UnitView, MAX_CONCEPT_CHARS, MAX_UNIT_CHARS};

fn truncate_chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn build_line(
    position: usize,
    job: ParsedJob,
    rates: &[RateView],
    index: &RateIndex,
    units: &UnitCatalog,
) -> BuiltLine {
    let dictated_unit = job
        .unit
        .as_deref()
        .map(|unit| truncate_chars(&units.label_of(unit), MAX_UNIT_CHARS));
    let found = index.match_job(&job.concept, job.unit.as_deref());

    let mut line = DraftLine {
        id: 0,
        position: position as i64,
        concept: truncate_chars(&job.concept, MAX_CONCEPT_CHARS),
        quantity_centi: job.quantity_centi,
        quantity_issue: job.quantity_issue,
        unit_label: dictated_unit,
        price_cents: None,
        price_source: PriceSource::Pending,
        rate_id: None,
        rate_concept: None,
        pending_reason: None,
        source_fragment: Some(job.fragment),
    };

    let suggestion_rate_ids = match found {
        RateMatch::Applied(rate_id) => {
            // El precio sale siempre de la tarifa aplicada (Principio IV).
            if let Some(rate) = rates.iter().find(|rate| rate.id == rate_id) {
                line.concept = rate.concept.clone();
                line.unit_label = Some(rate.unit.label.clone());
                line.price_cents = Some(rate.price_cents);
                line.price_source = PriceSource::Rate;
                line.rate_id = Some(rate.id);
                line.rate_concept = Some(rate.concept.clone());
            }
            Vec::new()
        }
        RateMatch::Pending {
            reason,
            suggestions,
        } => {
            line.pending_reason = Some(reason);
            suggestions
        }
    };

    BuiltLine {
        line,
        suggestion_rate_ids,
    }
}

/// Interpreta `text` y lo convierte en líneas con las tarifas y unidades indicadas.
pub fn build_draft(
    text: &str,
    rates: &[RateView],
    units: &[UnitView],
    vat_rate_bp: i64,
) -> BuiltDraft {
    let catalog = UnitCatalog::new(units);
    let parsed = parse_text(text, &catalog);
    let index = RateIndex::new(rates, &catalog);

    let lines = parsed
        .jobs
        .into_iter()
        .enumerate()
        .map(|(position, job)| build_line(position, job, rates, &index, &catalog))
        .collect();

    BuiltDraft {
        lines,
        unparsed_fragments: parsed.unparsed,
        vat_rate_bp,
    }
}
