//! Errores de la aplicación serializados según `contracts/ipc-commands.md`.
//!
//! Los `message` van siempre en español sencillo; el detalle técnico solo se registra en el log local.

use serde::Serialize;

/// Códigos de error por campo del contrato IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Required,
    TooLong,
    TaxIdInvalid,
    PostalCodeInvalid,
    PhoneInvalid,
    EmailInvalid,
    VatOutOfRange,
    PriceRequired,
    PriceInvalidFormat,
    PriceTooManyDecimals,
    PriceNotPositive,
    UnitRequired,
    ConceptRequired,
}

impl ErrorCode {
    /// Mensaje por defecto, en español sencillo, para mostrar junto al campo.
    pub fn message(self) -> &'static str {
        match self {
            ErrorCode::Required => "Este dato es obligatorio.",
            ErrorCode::TooLong => "El texto es demasiado largo.",
            ErrorCode::TaxIdInvalid => "El NIF, NIE o CIF no es válido. Revisa los números y la letra.",
            ErrorCode::PostalCodeInvalid => "El código postal debe tener 5 cifras (por ejemplo, 28001).",
            ErrorCode::PhoneInvalid => "El teléfono no es válido. Escribe 9 cifras (por ejemplo, 600 123 456).",
            ErrorCode::EmailInvalid => "El email no es válido (por ejemplo, nombre@correo.es).",
            ErrorCode::VatOutOfRange => {
                "El IVA debe ser un número entre 0 y 100, con dos decimales como máximo."
            }
            ErrorCode::PriceRequired => "Escribe el precio.",
            ErrorCode::PriceInvalidFormat => "Escribe el precio solo con números, por ejemplo 45,50.",
            ErrorCode::PriceTooManyDecimals => "El precio admite como máximo dos decimales.",
            ErrorCode::PriceNotPositive => "El precio debe ser mayor que cero.",
            ErrorCode::UnitRequired => "Elige o escribe una unidad.",
            ErrorCode::ConceptRequired => "Escribe el concepto de la tarifa.",
        }
    }
}

/// Motivo de rechazo de un logo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogoRejectCode {
    LogoTooLarge,
    LogoUnsupportedFormat,
    LogoCorrupt,
}

impl LogoRejectCode {
    pub fn message(self) -> &'static str {
        match self {
            LogoRejectCode::LogoTooLarge => {
                "La imagen pesa más de 5 MB. Elige una imagen más pequeña."
            }
            LogoRejectCode::LogoUnsupportedFormat => {
                "Ese archivo no es una imagen admitida. Usa una imagen PNG o JPG."
            }
            LogoRejectCode::LogoCorrupt => {
                "No se ha podido abrir la imagen: el archivo está dañado. Usa una imagen PNG o JPG."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldError {
    pub field: String,
    pub code: ErrorCode,
    pub message: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, code: ErrorCode) -> Self {
        Self { field: field.into(), code, message: code.message().to_string() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AppError {
    #[error("datos no válidos")]
    Validation { errors: Vec<FieldError> },
    #[error("{message}")]
    DuplicateRate { existing_rate_id: i64, message: String },
    #[error("{message}")]
    NotFound { message: String },
    #[error("{message}")]
    LogoRejected { code: LogoRejectCode, message: String },
    #[error("{message}")]
    Storage { message: String },
}

pub const STORAGE_MESSAGE: &str = "No se ha podido guardar. Inténtalo de nuevo.";

impl AppError {
    pub fn storage() -> Self {
        AppError::Storage { message: STORAGE_MESSAGE.to_string() }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        AppError::NotFound { message: message.into() }
    }

    pub fn logo_rejected(code: LogoRejectCode) -> Self {
        AppError::LogoRejected { code, message: code.message().to_string() }
    }

    pub fn validation(errors: Vec<FieldError>) -> Self {
        AppError::Validation { errors }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        log::error!("Error de base de datos: {e}");
        AppError::storage()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        log::error!("Error de disco: {e}");
        AppError::storage()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_like_the_contract() {
        let e = AppError::DuplicateRate { existing_rate_id: 7, message: "x".into() };
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["kind"], "duplicateRate");
        assert_eq!(json["existingRateId"], 7);

        let v = AppError::validation(vec![FieldError::new("taxId", ErrorCode::TaxIdInvalid)]);
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["kind"], "validation");
        assert_eq!(json["errors"][0]["code"], "TAX_ID_INVALID");

        let l = AppError::logo_rejected(LogoRejectCode::LogoTooLarge);
        let json = serde_json::to_value(&l).unwrap();
        assert_eq!(json["kind"], "logoRejected");
        assert_eq!(json["code"], "LOGO_TOO_LARGE");
    }
}
