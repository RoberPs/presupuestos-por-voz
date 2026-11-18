//! Transcripción local detrás de un trait, para probar los caminos de fallo sin el motor real.

#[cfg(feature = "local-transcription")]
pub mod whisper;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::Serialize;

use crate::error::TranscriptionCode;

/// Opciones de una transcripción.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranscribeOptions {
    /// Conceptos de las tarifas, como contexto para reconocer términos del oficio (R11).
    pub vocabulary: String,
}

/// Texto reconocido.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Transcript {
    pub text: String,
    /// La confianza del motor es baja: conviene revisar el texto (FR-007).
    pub low_quality: bool,
}

/// Por qué no se ha obtenido un texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscribeFailure {
    Cancelled,
    Failed(TranscriptionCode),
}

/// Motor de transcripción. Recibe audio de 16 kHz y mono en el rango [-1, 1].
pub trait Transcriber: Send + Sync {
    fn transcribe(
        &self,
        samples: &[f32],
        options: &TranscribeOptions,
        progress: &(dyn Fn(u8) + Send + Sync),
        cancel: &Arc<AtomicBool>,
    ) -> Result<Transcript, TranscribeFailure>;
}

/// Sustituye al motor real al compilar sin la feature `local-transcription`.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnavailableTranscriber;

impl Transcriber for UnavailableTranscriber {
    fn transcribe(
        &self,
        _samples: &[f32],
        _options: &TranscribeOptions,
        _progress: &(dyn Fn(u8) + Send + Sync),
        _cancel: &Arc<AtomicBool>,
    ) -> Result<Transcript, TranscribeFailure> {
        Err(TranscribeFailure::Failed(
            TranscriptionCode::ModelUnavailable,
        ))
    }
}

/// Etapas del proceso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessingStage {
    Preparing,
    Transcribing,
    BuildingDraft,
}

/// Resultado final del proceso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessingOutcome {
    DraftReady,
    NoSpeech,
    Cancelled,
    Failed,
}

/// Eventos de `dictation_process` (contrato IPC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProcessingEvent {
    Stage { stage: ProcessingStage },
    Progress { percent: u8 },
    Finished { outcome: ProcessingOutcome },
}
