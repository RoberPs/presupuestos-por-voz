//! Audio del dictado: captura del micrófono, importación y normalización a 16 kHz y mono.

pub mod capture;
pub mod decode;
pub mod quality;
pub mod resample;
pub mod wav;

use std::path::Path;
use std::sync::Arc;

use serde::Serialize;

use crate::error::{AppError, MicrophoneCode};

/// Frecuencia de los audios guardados y de la transcripción.
pub const SAMPLE_RATE: u32 = 16_000;
/// Duración máxima de un dictado (FR-005).
pub const MAX_DURATION_MS: i64 = 600_000;
/// Aviso un minuto antes del límite (FR-005).
pub const WARNING_AT_MS: i64 = 540_000;

/// Motivo por el que la grabación se ha detenido sola.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    LimitReached,
    DeviceLost,
}

/// Eventos de `dictation_start` (contrato IPC).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RecordingEvent {
    Level {
        elapsed_ms: i64,
        level: f32,
    },
    LimitWarning {
        remaining_ms: i64,
    },
    Stopped {
        reason: StopReason,
        duration_ms: i64,
    },
}

pub type RecordingSink = Arc<dyn Fn(RecordingEvent) + Send + Sync>;

/// Grabación en curso.
pub trait ActiveRecording: Send {
    /// Cierra el archivo y devuelve la duración grabada, en milisegundos.
    fn stop(self: Box<Self>) -> Result<i64, AppError>;
}

/// Micrófono. Escribe la grabación en `part_path` como WAV de 16 kHz, mono y 16 bits.
pub trait Recorder: Send + Sync {
    fn start(
        &self,
        part_path: &Path,
        events: RecordingSink,
    ) -> Result<Box<dyn ActiveRecording>, AppError>;
}

/// Micrófono no disponible: es el valor por defecto fuera de la aplicación real.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnavailableRecorder;

impl Recorder for UnavailableRecorder {
    fn start(
        &self,
        _part_path: &Path,
        _events: RecordingSink,
    ) -> Result<Box<dyn ActiveRecording>, AppError> {
        Err(AppError::microphone(MicrophoneCode::MicUnavailable))
    }
}
