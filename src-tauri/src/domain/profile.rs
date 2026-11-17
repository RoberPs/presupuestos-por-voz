//! Perfil del negocio: datos, límites, estado completo/incompleto (R7) y vistas del contrato.

use serde::{Deserialize, Serialize};

use crate::domain::money;
use crate::domain::normalize::collapse_spaces;
use crate::domain::validation;
use crate::error::{ErrorCode, FieldError};

pub const DEFAULT_VAT_BP: i64 = 2100;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub province: String,
}

/// Entrada de `profile_save` / `profile_validate`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessProfileInput {
    pub trade_name: String,
    #[serde(default)]
    pub legal_name: Option<String>,
    pub tax_id: String,
    pub phone: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    pub address: Address,
    pub vat_rate_percent: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoMeta {
    /// Ruta relativa a `app_data_dir`, p. ej. `branding/logo-1a2b3c4d.png`.
    pub file: String,
    pub mime: String,
    pub width_px: u32,
    pub height_px: u32,
    pub bytes: u64,
    pub sha256: String,
}

impl LogoMeta {
    pub fn low_resolution(&self) -> bool {
        self.width_px.min(self.height_px) < LOW_RES_MIN_SIDE_PX
    }
}

pub const LOW_RES_MIN_SIDE_PX: u32 = 200;

/// Perfil tal como se guarda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessProfile {
    pub trade_name: String,
    pub legal_name: Option<String>,
    pub tax_id: String,
    pub phone: String,
    pub email: Option<String>,
    pub website: Option<String>,
    pub address: Address,
    pub vat_rate_bp: i64,
    pub logo: Option<LogoMeta>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileStatus {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoView {
    pub data_url: String,
    pub mime: String,
    pub width_px: u32,
    pub height_px: u32,
    pub low_resolution: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoSetResult {
    pub logo: LogoView,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessProfileView {
    pub trade_name: String,
    pub legal_name: Option<String>,
    pub tax_id: String,
    pub phone: String,
    pub email: Option<String>,
    pub website: Option<String>,
    pub address: Address,
    pub vat_rate_percent: String,
    pub currency: &'static str,
    pub logo: Option<LogoView>,
    pub status: ProfileStatus,
    pub missing_fields: Vec<FieldError>,
}

/// Límites de longitud en caracteres (data-model.md).
struct Limit {
    field: &'static str,
    max: usize,
}

const LIMITS: &[Limit] = &[
    Limit {
        field: "tradeName",
        max: 120,
    },
    Limit {
        field: "legalName",
        max: 160,
    },
    Limit {
        field: "taxId",
        max: 20,
    },
    Limit {
        field: "phone",
        max: 30,
    },
    Limit {
        field: "email",
        max: 254,
    },
    Limit {
        field: "website",
        max: 200,
    },
    Limit {
        field: "address.street",
        max: 160,
    },
    Limit {
        field: "address.postalCode",
        max: 10,
    },
    Limit {
        field: "address.city",
        max: 80,
    },
    Limit {
        field: "address.province",
        max: 80,
    },
];

fn input_value<'a>(input: &'a BusinessProfileInput, field: &str) -> &'a str {
    match field {
        "tradeName" => &input.trade_name,
        "legalName" => input.legal_name.as_deref().unwrap_or(""),
        "taxId" => &input.tax_id,
        "phone" => &input.phone,
        "email" => input.email.as_deref().unwrap_or(""),
        "website" => input.website.as_deref().unwrap_or(""),
        "address.street" => &input.address.street,
        "address.postalCode" => &input.address.postal_code,
        "address.city" => &input.address.city,
        "address.province" => &input.address.province,
        _ => "",
    }
}

/// Errores que impiden guardar: textos demasiado largos e IVA fuera de rango.
pub fn hard_errors(input: &BusinessProfileInput) -> Vec<FieldError> {
    let mut errors: Vec<FieldError> = LIMITS
        .iter()
        .filter(|l| collapse_spaces(input_value(input, l.field)).chars().count() > l.max)
        .map(|l| FieldError::new(l.field, ErrorCode::TooLong))
        .collect();
    if let Err(code) = money::parse_percent_bp(&input.vat_rate_percent) {
        errors.push(FieldError::new("vatRatePercent", code));
    }
    errors
}

fn clean(s: &str) -> String {
    collapse_spaces(s)
}

fn clean_opt(s: &Option<String>) -> Option<String> {
    s.as_deref().map(clean).filter(|v| !v.is_empty())
}

/// Convierte la entrada en el perfil que se guarda (trim y NIF normalizado). El IVA inválido
/// se sustituye por el valor por defecto; `hard_errors` se encarga de rechazarlo antes.
pub fn from_input(input: &BusinessProfileInput) -> BusinessProfile {
    BusinessProfile {
        trade_name: clean(&input.trade_name),
        legal_name: clean_opt(&input.legal_name),
        tax_id: validation::normalize_tax_id(&input.tax_id),
        phone: clean(&input.phone),
        email: clean_opt(&input.email),
        website: clean_opt(&input.website),
        address: Address {
            street: clean(&input.address.street),
            postal_code: clean(&input.address.postal_code),
            city: clean(&input.address.city),
            province: clean(&input.address.province),
        },
        vat_rate_bp: money::parse_percent_bp(&input.vat_rate_percent).unwrap_or(DEFAULT_VAT_BP),
        logo: None,
    }
}

/// Campos obligatorios vacíos o con formato inválido (incluido un email opcional mal escrito).
pub fn missing_fields(p: &BusinessProfile) -> Vec<FieldError> {
    type Check = fn(&str) -> Result<String, ErrorCode>;
    let required: [(&str, &str, Option<Check>); 7] = [
        ("tradeName", &p.trade_name, None),
        ("taxId", &p.tax_id, Some(validation::validate_tax_id)),
        ("phone", &p.phone, Some(validation::validate_phone)),
        ("address.street", &p.address.street, None),
        (
            "address.postalCode",
            &p.address.postal_code,
            Some(validation::validate_postal_code),
        ),
        ("address.city", &p.address.city, None),
        ("address.province", &p.address.province, None),
    ];

    let mut out = Vec::new();
    for (field, value, check) in required {
        if value.trim().is_empty() {
            out.push(FieldError::new(field, ErrorCode::Required));
        } else if let Some(Err(code)) = check.map(|f| f(value)) {
            out.push(FieldError::new(field, code));
        }
    }
    if let Some(email) = p.email.as_deref() {
        if let Err(code) = validation::validate_email(email) {
            out.push(FieldError::new("email", code));
        }
    }
    out
}

pub fn status(missing: &[FieldError]) -> ProfileStatus {
    if missing.is_empty() {
        ProfileStatus::Complete
    } else {
        ProfileStatus::Incomplete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> BusinessProfileInput {
        BusinessProfileInput {
            trade_name: "  Pinturas   Ana ".into(),
            legal_name: Some("".into()),
            tax_id: "12.345.678-z".into(),
            phone: "600123456".into(),
            email: None,
            website: None,
            address: Address {
                street: "C/ Sol 3".into(),
                postal_code: "41001".into(),
                city: "Sevilla".into(),
                province: "Sevilla".into(),
            },
            vat_rate_percent: "21".into(),
        }
    }

    #[test]
    fn from_input_cleans_values() {
        let p = from_input(&input());
        assert_eq!(p.trade_name, "Pinturas Ana");
        assert_eq!(p.legal_name, None);
        assert_eq!(p.tax_id, "12345678Z");
        assert_eq!(p.vat_rate_bp, 2100);
        assert!(missing_fields(&p).is_empty());
    }

    #[test]
    fn hard_errors_detect_long_text_and_vat() {
        let mut i = input();
        i.address.city = "x".repeat(81);
        i.vat_rate_percent = "100,01".into();
        let codes: Vec<_> = hard_errors(&i)
            .into_iter()
            .map(|e| (e.field, e.code))
            .collect();
        assert_eq!(
            codes,
            vec![
                ("address.city".to_string(), ErrorCode::TooLong),
                ("vatRatePercent".to_string(), ErrorCode::VatOutOfRange)
            ]
        );
    }
}
