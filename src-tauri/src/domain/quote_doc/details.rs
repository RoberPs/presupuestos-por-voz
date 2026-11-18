//! Validación de los datos del cliente, el número y la validez (data-model.md).
//!
//! Los datos se guardan tal como se escriben, sin comprobar su formato: pueden ser de particulares,
//! empresas o clientes extranjeros.

use serde::{Deserialize, Serialize};

use crate::domain::normalize::collapse_spaces;
use crate::error::{ErrorCode, FieldError};

const MAX_CLIENT_NAME: usize = 120;
const MAX_CLIENT_ADDRESS: usize = 200;
const MAX_CLIENT_PHONE: usize = 30;
const MAX_CLIENT_EMAIL: usize = 254;
const MAX_CLIENT_TAX_ID: usize = 20;
const MAX_QUOTE_NUMBER: usize = 30;
const VALIDITY_DAYS: std::ops::RangeInclusive<i64> = 1..=365;

/// Datos que escribe el profesional en la confirmación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteDetailsInput {
    pub client_name: String,
    pub client_address: Option<String>,
    pub client_phone: Option<String>,
    pub client_email: Option<String>,
    pub client_tax_id: Option<String>,
    pub quote_number: String,
    pub validity_days: i64,
}

/// Datos ya limpios y dentro de los límites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanDetails {
    pub client_name: String,
    pub client_address: Option<String>,
    pub client_phone: Option<String>,
    pub client_email: Option<String>,
    pub client_tax_id: Option<String>,
    pub quote_number: String,
    pub validity_days: i64,
}

fn required(value: &str, field: &str, max: usize, errors: &mut Vec<FieldError>) -> String {
    let clean = collapse_spaces(value);
    if clean.chars().count() > max {
        errors.push(FieldError::new(field, ErrorCode::TooLong));
    }
    clean
}

fn optional(
    value: Option<&str>,
    field: &str,
    max: usize,
    errors: &mut Vec<FieldError>,
) -> Option<String> {
    let clean = required(value.unwrap_or(""), field, max, errors);
    (!clean.is_empty()).then_some(clean)
}

/// Limpia los datos y comprueba sus límites. Un nombre o un número vacíos se admiten aquí y
/// aparecen después como bloqueo.
pub fn validate(input: &QuoteDetailsInput) -> Result<CleanDetails, Vec<FieldError>> {
    let mut errors = Vec::new();
    let clean = CleanDetails {
        client_name: required(
            &input.client_name,
            "clientName",
            MAX_CLIENT_NAME,
            &mut errors,
        ),
        client_address: optional(
            input.client_address.as_deref(),
            "clientAddress",
            MAX_CLIENT_ADDRESS,
            &mut errors,
        ),
        client_phone: optional(
            input.client_phone.as_deref(),
            "clientPhone",
            MAX_CLIENT_PHONE,
            &mut errors,
        ),
        client_email: optional(
            input.client_email.as_deref(),
            "clientEmail",
            MAX_CLIENT_EMAIL,
            &mut errors,
        ),
        client_tax_id: optional(
            input.client_tax_id.as_deref(),
            "clientTaxId",
            MAX_CLIENT_TAX_ID,
            &mut errors,
        ),
        quote_number: required(
            &input.quote_number,
            "quoteNumber",
            MAX_QUOTE_NUMBER,
            &mut errors,
        ),
        validity_days: input.validity_days,
    };
    if !VALIDITY_DAYS.contains(&input.validity_days) {
        errors.push(FieldError::new("validityDays", ErrorCode::ValidityInvalid));
    }
    if errors.is_empty() {
        Ok(clean)
    } else {
        Err(errors)
    }
}
