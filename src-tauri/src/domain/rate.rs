//! Tarifas: validación de la entrada y vistas del contrato (FR-010…FR-019).

use serde::{Deserialize, Serialize};

use crate::domain::money;
use crate::domain::normalize::{collapse_spaces, normalize};
use crate::error::{ErrorCode, FieldError};

pub const MAX_CONCEPT_CHARS: usize = 160;
pub const MAX_UNIT_CHARS: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateInput {
    pub concept: String,
    pub unit_label: String,
    pub price: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidRate {
    pub concept: String,
    pub concept_norm: String,
    pub unit_label: String,
    pub unit_norm: String,
    pub price_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitView {
    pub id: i64,
    pub label: String,
    pub is_predefined: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateView {
    pub id: i64,
    pub concept: String,
    pub unit: UnitView,
    pub price_cents: i64,
    pub price_display: String,
}

impl RateView {
    pub fn new(id: i64, concept: String, unit: UnitView, price_cents: i64) -> Self {
        Self {
            id,
            concept,
            unit,
            price_cents,
            price_display: money::format_eur(price_cents),
        }
    }
}

pub fn validate(input: &RateInput) -> Result<ValidRate, Vec<FieldError>> {
    let mut errors = Vec::new();

    let concept = collapse_spaces(&input.concept);
    if concept.is_empty() {
        errors.push(FieldError::new("concept", ErrorCode::ConceptRequired));
    } else if concept.chars().count() > MAX_CONCEPT_CHARS {
        errors.push(FieldError::new("concept", ErrorCode::TooLong));
    }

    let unit_label = collapse_spaces(&input.unit_label);
    if unit_label.is_empty() {
        errors.push(FieldError::new("unitLabel", ErrorCode::UnitRequired));
    } else if unit_label.chars().count() > MAX_UNIT_CHARS {
        errors.push(FieldError::new("unitLabel", ErrorCode::TooLong));
    }

    let price = money::parse_price_cents(&input.price);
    if let Err(code) = price {
        errors.push(FieldError::new("price", code));
    }

    match price {
        Ok(price_cents) if errors.is_empty() => Ok(ValidRate {
            concept_norm: normalize(&concept),
            unit_norm: normalize(&unit_label),
            concept,
            unit_label,
            price_cents,
        }),
        _ => Err(errors),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_normalizes() {
        let v = validate(&RateInput {
            concept: " Instalación  de grifo ".into(),
            unit_label: " Metro  lineal ".into(),
            price: "1.234,56".into(),
        })
        .unwrap();
        assert_eq!(v.concept, "Instalación de grifo");
        assert_eq!(v.concept_norm, "instalacion de grifo");
        assert_eq!(v.unit_label, "Metro lineal");
        assert_eq!(v.unit_norm, "metro lineal");
        assert_eq!(v.price_cents, 123456);
    }
}
