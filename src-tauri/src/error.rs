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
    QuantityRequired,
    QuantityInvalidFormat,
    QuantityTooManyDecimals,
    QuantityNotPositive,
    AmountTooLarge,
    TextRequired,
    ValidityInvalid,
}

impl ErrorCode {
    /// Mensaje por defecto, en español sencillo, para mostrar junto al campo.
    pub fn message(self) -> &'static str {
        match self {
            ErrorCode::Required => "Este dato es obligatorio.",
            ErrorCode::TooLong => "El texto es demasiado largo.",
            ErrorCode::TaxIdInvalid => {
                "El NIF, NIE o CIF no es válido. Revisa los números y la letra."
            }
            ErrorCode::PostalCodeInvalid => {
                "El código postal debe tener 5 cifras (por ejemplo, 28001)."
            }
            ErrorCode::PhoneInvalid => {
                "El teléfono no es válido. Escribe 9 cifras (por ejemplo, 600 123 456)."
            }
            ErrorCode::EmailInvalid => "El email no es válido (por ejemplo, nombre@correo.es).",
            ErrorCode::VatOutOfRange => {
                "El IVA debe ser un número entre 0 y 100, con dos decimales como máximo."
            }
            ErrorCode::PriceRequired => "Escribe el precio.",
            ErrorCode::PriceInvalidFormat => {
                "Escribe el precio solo con números, por ejemplo 45,50."
            }
            ErrorCode::PriceTooManyDecimals => "El precio admite como máximo dos decimales.",
            ErrorCode::PriceNotPositive => "El precio debe ser mayor que cero.",
            ErrorCode::UnitRequired => "Elige o escribe una unidad.",
            ErrorCode::ConceptRequired => "Escribe el concepto.",
            ErrorCode::QuantityRequired => "Escribe la cantidad.",
            ErrorCode::QuantityInvalidFormat => {
                "Escribe la cantidad solo con números, por ejemplo 1,5."
            }
            ErrorCode::QuantityTooManyDecimals => "La cantidad admite como máximo dos decimales.",
            ErrorCode::QuantityNotPositive => "La cantidad debe ser mayor que cero.",
            ErrorCode::AmountTooLarge => {
                "El importe es demasiado grande. Revisa la cantidad y el precio."
            }
            ErrorCode::TextRequired => "Escribe primero lo que quieres presupuestar.",
            ErrorCode::ValidityInvalid => "La validez debe ser un número de días entre 1 y 365.",
        }
    }
}

/// Problemas con el micrófono (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MicrophoneCode {
    MicNotFound,
    MicAccessDenied,
    MicUnavailable,
}

impl MicrophoneCode {
    pub fn message(self) -> &'static str {
        match self {
            MicrophoneCode::MicNotFound => {
                "No se ha encontrado ningún micrófono. Conecta uno, importa un audio o escribe la visita a mano."
            }
            MicrophoneCode::MicAccessDenied => {
                "Windows no deja usar el micrófono. Actívalo en Configuración > Privacidad > Micrófono, importa un audio o escribe la visita a mano."
            }
            MicrophoneCode::MicUnavailable => {
                "El micrófono no está disponible ahora. Cierra otros programas que lo usen, importa un audio o escribe la visita a mano."
            }
        }
    }
}

/// Motivo de rechazo de un audio importado (FR-002c).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AudioRejectCode {
    AudioUnsupportedFormat,
    AudioCorrupt,
    AudioTooLong,
}

impl AudioRejectCode {
    pub fn message(self) -> &'static str {
        match self {
            AudioRejectCode::AudioUnsupportedFormat => {
                "Ese archivo no es un audio admitido. Usa un audio M4A, MP3 o WAV."
            }
            AudioRejectCode::AudioCorrupt => {
                "No se ha podido abrir el audio: el archivo está dañado."
            }
            AudioRejectCode::AudioTooLong => {
                "El audio dura más de 10 minutos. Elige uno más corto."
            }
        }
    }
}

/// Fallos de la transcripción local (FR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TranscriptionCode {
    ModelUnavailable,
    TranscriptionFailed,
}

impl TranscriptionCode {
    pub fn message(self) -> &'static str {
        match self {
            TranscriptionCode::ModelUnavailable => {
                "No se encuentra el reconocimiento de voz de la aplicación. Tu audio está guardado: vuelve a instalar la aplicación y reinténtalo, o escribe la visita a mano."
            }
            TranscriptionCode::TranscriptionFailed => {
                "No se ha podido pasar el audio a texto. Tu audio está guardado: inténtalo de nuevo o escribe la visita a mano."
            }
        }
    }

    /// Valor guardado en `dictations.error_code`.
    pub fn as_db(self) -> &'static str {
        match self {
            TranscriptionCode::ModelUnavailable => "MODEL_UNAVAILABLE",
            TranscriptionCode::TranscriptionFailed => "TRANSCRIPTION_FAILED",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "MODEL_UNAVAILABLE" => Some(TranscriptionCode::ModelUnavailable),
            "TRANSCRIPTION_FAILED" => Some(TranscriptionCode::TranscriptionFailed),
            _ => None,
        }
    }
}

/// Acciones que exigen la confirmación del profesional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfirmationReason {
    ReplaceDraft,
    ReplacePendingAudio,
    DiscardManualChanges,
}

impl ConfirmationReason {
    pub fn message(self) -> &'static str {
        match self {
            ConfirmationReason::ReplaceDraft => {
                "Ya tienes un presupuesto en curso. Si sigues, se sustituirá por el nuevo."
            }
            ConfirmationReason::ReplacePendingAudio => {
                "Tienes un audio sin procesar. Si sigues, se perderá."
            }
            ConfirmationReason::DiscardManualChanges => {
                "Has hecho cambios a mano en este presupuesto. Si sigues, se perderán."
            }
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
        Self {
            field: field.into(),
            code,
            message: code.message().to_string(),
        }
    }
}

/// Motivo por el que no se ha podido guardar el PDF (FR-023).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PdfSaveCode {
    PdfAccessDenied,
    PdfDiskFull,
    PdfLocationUnavailable,
    PdfFileInUse,
    PdfWriteFailed,
}

impl PdfSaveCode {
    pub fn message(self) -> &'static str {
        match self {
            PdfSaveCode::PdfAccessDenied => {
                "No tienes permiso para guardar en esa carpeta. Elige otra."
            }
            PdfSaveCode::PdfDiskFull => {
                "No queda espacio suficiente en ese disco. Libera espacio o elige otra carpeta."
            }
            PdfSaveCode::PdfLocationUnavailable => {
                "Esa carpeta ya no está disponible. Comprueba que el disco o la memoria siguen conectados, o elige otra."
            }
            PdfSaveCode::PdfFileInUse => {
                "El archivo está abierto en otro programa. Ciérralo y vuelve a intentarlo, o elige otro nombre."
            }
            PdfSaveCode::PdfWriteFailed => {
                "No se ha podido guardar el PDF. Inténtalo de nuevo o elige otra carpeta."
            }
        }
    }
}

/// Línea pendiente de completar, identificada por su concepto (FR-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingLineRef {
    pub id: i64,
    pub concept: String,
}

/// Dato con caracteres que el PDF no puede mostrar (FR-018b).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsupportedField {
    pub field: String,
    pub label: String,
    pub characters: Vec<String>,
}

/// Motivo por el que todavía no se puede generar el PDF (data-model.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuoteBlocker {
    NoLines,
    PendingLines { lines: Vec<PendingLineRef> },
    NotReviewed,
    ProfileIncomplete { missing_fields: Vec<FieldError> },
    ClientNameRequired,
    QuoteNumberRequired,
    UnsupportedCharacters { fields: Vec<UnsupportedField> },
    LogoUnreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AppError {
    #[error("datos no válidos")]
    Validation { errors: Vec<FieldError> },
    #[error("{message}")]
    DuplicateRate {
        existing_rate_id: i64,
        message: String,
    },
    #[error("{message}")]
    NotFound { message: String },
    #[error("{message}")]
    LogoRejected {
        code: LogoRejectCode,
        message: String,
    },
    #[error("{message}")]
    Storage { message: String },
    #[error("{message}")]
    Microphone {
        code: MicrophoneCode,
        message: String,
    },
    #[error("{message}")]
    AudioRejected {
        code: AudioRejectCode,
        message: String,
    },
    #[error("{message}")]
    Transcription {
        code: TranscriptionCode,
        message: String,
    },
    #[error("{message}")]
    ConfirmationRequired {
        reason: ConfirmationReason,
        message: String,
    },
    #[error("{message}")]
    InvalidState { message: String },
    #[error("{message}")]
    DraftHasPending {
        pending_line_ids: Vec<i64>,
        message: String,
    },
    #[error("{message}")]
    QuoteBlocked {
        blockers: Vec<QuoteBlocker>,
        message: String,
    },
    #[error("{message}")]
    QuoteChanged { message: String },
    #[error("{message}")]
    PdfSave { code: PdfSaveCode, message: String },
    #[error("{message}")]
    PdfRender { message: String },
}

pub const STORAGE_MESSAGE: &str = "No se ha podido guardar. Inténtalo de nuevo.";

impl AppError {
    pub fn storage() -> Self {
        AppError::Storage {
            message: STORAGE_MESSAGE.to_string(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        AppError::NotFound {
            message: message.into(),
        }
    }

    pub fn logo_rejected(code: LogoRejectCode) -> Self {
        AppError::LogoRejected {
            code,
            message: code.message().to_string(),
        }
    }

    pub fn validation(errors: Vec<FieldError>) -> Self {
        AppError::Validation { errors }
    }

    /// Error de validación de un solo campo.
    pub fn field(field: impl Into<String>, code: ErrorCode) -> Self {
        AppError::validation(vec![FieldError::new(field, code)])
    }

    pub fn microphone(code: MicrophoneCode) -> Self {
        AppError::Microphone {
            code,
            message: code.message().to_string(),
        }
    }

    pub fn audio_rejected(code: AudioRejectCode) -> Self {
        AppError::AudioRejected {
            code,
            message: code.message().to_string(),
        }
    }

    pub fn transcription(code: TranscriptionCode) -> Self {
        AppError::Transcription {
            code,
            message: code.message().to_string(),
        }
    }

    pub fn confirmation_required(reason: ConfirmationReason) -> Self {
        AppError::ConfirmationRequired {
            reason,
            message: reason.message().to_string(),
        }
    }

    pub fn invalid_state(message: impl Into<String>) -> Self {
        AppError::InvalidState {
            message: message.into(),
        }
    }

    pub fn draft_has_pending(pending_line_ids: Vec<i64>) -> Self {
        AppError::DraftHasPending {
            pending_line_ids,
            message: "Faltan líneas por completar antes de dar el presupuesto por bueno.".into(),
        }
    }

    pub fn quote_blocked(blockers: Vec<QuoteBlocker>) -> Self {
        AppError::QuoteBlocked {
            blockers,
            message: "Todavía no se puede generar el PDF. Revisa lo que falta.".into(),
        }
    }

    pub fn quote_changed() -> Self {
        AppError::QuoteChanged {
            message: "El presupuesto ha cambiado. Revísalo y vuelve a confirmar.".into(),
        }
    }

    pub fn pdf_save(code: PdfSaveCode) -> Self {
        AppError::PdfSave {
            code,
            message: code.message().to_string(),
        }
    }

    pub fn pdf_render() -> Self {
        AppError::PdfRender {
            message: "No se ha podido preparar el PDF. Inténtalo de nuevo.".into(),
        }
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
        let e = AppError::DuplicateRate {
            existing_rate_id: 7,
            message: "x".into(),
        };
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

    #[test]
    fn voice_draft_errors_serialize_like_the_contract() {
        let json =
            serde_json::to_value(AppError::microphone(MicrophoneCode::MicAccessDenied)).unwrap();
        assert_eq!(json["kind"], "microphone");
        assert_eq!(json["code"], "MIC_ACCESS_DENIED");

        let json =
            serde_json::to_value(AppError::audio_rejected(AudioRejectCode::AudioTooLong)).unwrap();
        assert_eq!(json["kind"], "audioRejected");
        assert_eq!(json["code"], "AUDIO_TOO_LONG");

        let json =
            serde_json::to_value(AppError::transcription(TranscriptionCode::ModelUnavailable))
                .unwrap();
        assert_eq!(json["kind"], "transcription");
        assert_eq!(json["code"], "MODEL_UNAVAILABLE");

        let json = serde_json::to_value(AppError::confirmation_required(
            ConfirmationReason::ReplacePendingAudio,
        ))
        .unwrap();
        assert_eq!(json["kind"], "confirmationRequired");
        assert_eq!(json["reason"], "replacePendingAudio");

        let json = serde_json::to_value(AppError::invalid_state("x")).unwrap();
        assert_eq!(json["kind"], "invalidState");

        let json = serde_json::to_value(AppError::draft_has_pending(vec![3, 5])).unwrap();
        assert_eq!(json["kind"], "draftHasPending");
        assert_eq!(json["pendingLineIds"], serde_json::json!([3, 5]));

        let json = serde_json::to_value(AppError::field("text", ErrorCode::TextRequired)).unwrap();
        assert_eq!(json["errors"][0]["code"], "TEXT_REQUIRED");
        let json = serde_json::to_value(AppError::field(
            "quantity",
            ErrorCode::QuantityTooManyDecimals,
        ))
        .unwrap();
        assert_eq!(json["errors"][0]["code"], "QUANTITY_TOO_MANY_DECIMALS");
    }

    #[test]
    fn quote_errors_serialize_like_the_contract() {
        let json = serde_json::to_value(AppError::quote_blocked(vec![
            QuoteBlocker::NoLines,
            QuoteBlocker::PendingLines {
                lines: vec![PendingLineRef {
                    id: 4,
                    concept: "Bajante".into(),
                }],
            },
            QuoteBlocker::NotReviewed,
            QuoteBlocker::ProfileIncomplete {
                missing_fields: vec![FieldError::new("taxId", ErrorCode::Required)],
            },
            QuoteBlocker::ClientNameRequired,
            QuoteBlocker::QuoteNumberRequired,
            QuoteBlocker::UnsupportedCharacters {
                fields: vec![UnsupportedField {
                    field: "clientName".into(),
                    label: "Nombre del cliente".into(),
                    characters: vec!["😀".into()],
                }],
            },
            QuoteBlocker::LogoUnreadable,
        ]))
        .unwrap();
        assert_eq!(json["kind"], "quoteBlocked");
        assert!(json["message"].is_string());
        let blockers = &json["blockers"];
        assert_eq!(blockers[0]["kind"], "noLines");
        assert_eq!(blockers[1]["kind"], "pendingLines");
        assert_eq!(blockers[1]["lines"][0]["id"], 4);
        assert_eq!(blockers[1]["lines"][0]["concept"], "Bajante");
        assert_eq!(blockers[2]["kind"], "notReviewed");
        assert_eq!(blockers[3]["kind"], "profileIncomplete");
        assert_eq!(blockers[3]["missingFields"][0]["field"], "taxId");
        assert_eq!(blockers[4]["kind"], "clientNameRequired");
        assert_eq!(blockers[5]["kind"], "quoteNumberRequired");
        assert_eq!(blockers[6]["kind"], "unsupportedCharacters");
        assert_eq!(blockers[6]["fields"][0]["field"], "clientName");
        assert_eq!(blockers[6]["fields"][0]["label"], "Nombre del cliente");
        assert_eq!(blockers[6]["fields"][0]["characters"][0], "😀");
        assert_eq!(blockers[7]["kind"], "logoUnreadable");

        let json = serde_json::to_value(AppError::quote_changed()).unwrap();
        assert_eq!(json["kind"], "quoteChanged");
        assert!(json["message"].is_string());

        let json = serde_json::to_value(AppError::pdf_render()).unwrap();
        assert_eq!(json["kind"], "pdfRender");
        assert!(json["message"].is_string());
    }

    #[test]
    fn every_pdf_save_code_serializes_like_the_contract() {
        for (code, expected) in [
            (PdfSaveCode::PdfAccessDenied, "PDF_ACCESS_DENIED"),
            (PdfSaveCode::PdfDiskFull, "PDF_DISK_FULL"),
            (
                PdfSaveCode::PdfLocationUnavailable,
                "PDF_LOCATION_UNAVAILABLE",
            ),
            (PdfSaveCode::PdfFileInUse, "PDF_FILE_IN_USE"),
            (PdfSaveCode::PdfWriteFailed, "PDF_WRITE_FAILED"),
        ] {
            let json = serde_json::to_value(AppError::pdf_save(code)).unwrap();
            assert_eq!(json["kind"], "pdfSave");
            assert_eq!(json["code"], expected);
            assert_eq!(json["message"], code.message());
        }
    }

    #[test]
    fn validity_invalid_serializes_like_the_contract() {
        let json =
            serde_json::to_value(AppError::field("validityDays", ErrorCode::ValidityInvalid))
                .unwrap();
        assert_eq!(json["errors"][0]["field"], "validityDays");
        assert_eq!(json["errors"][0]["code"], "VALIDITY_INVALID");
    }
}
