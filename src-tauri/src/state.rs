//! Estado compartido de la aplicación.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use rusqlite::Connection;

use crate::audio::{ActiveRecording, Recorder, RecordingEvent, RecordingSink, UnavailableRecorder};
use crate::error::AppError;
use crate::transcription::{ProcessingEvent, Transcriber, UnavailableTranscriber};

pub type ProcessingSink = Arc<dyn Fn(ProcessingEvent) + Send + Sync>;

/// Bloquea un mutex; uno envenenado se recupera, porque el estado sigue siendo utilizable.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Qué está haciendo el dictado ahora mismo. Solo puede haber una actividad a la vez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Activity {
    #[default]
    Idle,
    Recording {
        started: Instant,
    },
    Processing {
        percent: Option<u8>,
    },
}

/// Canales de eventos actuales. Se pueden sustituir al volver a la pantalla (`dictation_subscribe`).
#[derive(Default)]
pub struct EventHub {
    recording: Mutex<Option<RecordingSink>>,
    processing: Mutex<Option<ProcessingSink>>,
}

impl EventHub {
    pub fn set_recording(&self, sink: Option<RecordingSink>) {
        *lock(&self.recording) = sink;
    }

    pub fn set_processing(&self, sink: Option<ProcessingSink>) {
        *lock(&self.processing) = sink;
    }

    pub fn emit_recording(&self, event: RecordingEvent) {
        let sink = lock(&self.recording).clone();
        if let Some(sink) = sink {
            sink(event);
        }
    }

    pub fn emit_processing(&self, event: ProcessingEvent) {
        let sink = lock(&self.processing).clone();
        if let Some(sink) = sink {
            sink(event);
        }
    }
}

/// Grabación y proceso activos.
#[derive(Default)]
pub struct DictationRuntime {
    activity: Mutex<Activity>,
    recording: Mutex<Option<Box<dyn ActiveRecording>>>,
    pub cancel: Arc<AtomicBool>,
    pub events: Arc<EventHub>,
}

impl DictationRuntime {
    pub fn activity(&self) -> Activity {
        *lock(&self.activity)
    }

    pub fn set_activity(&self, activity: Activity) {
        *lock(&self.activity) = activity;
    }

    /// Pasa a `next` solo si no hay ninguna actividad en curso. Devuelve si lo ha conseguido.
    pub fn try_begin(&self, next: Activity) -> bool {
        let mut activity = lock(&self.activity);
        if *activity == Activity::Idle {
            *activity = next;
            true
        } else {
            false
        }
    }

    pub fn set_recording(&self, recording: Box<dyn ActiveRecording>) {
        *lock(&self.recording) = Some(recording);
    }

    pub fn take_recording(&self) -> Option<Box<dyn ActiveRecording>> {
        lock(&self.recording).take()
    }
}

/// Generación del PDF en curso y último destino elegido. Solo se guarda en memoria.
#[derive(Default)]
pub struct QuoteRuntime {
    generating: AtomicBool,
    last_destination: Mutex<Option<PathBuf>>,
}

/// Mientras exista, ninguna otra generación puede empezar (FR-026).
pub struct GenerationGuard<'a> {
    runtime: &'a QuoteRuntime,
}

impl Drop for GenerationGuard<'_> {
    fn drop(&mut self) {
        self.runtime.generating.store(false, Ordering::SeqCst);
    }
}

impl QuoteRuntime {
    /// Empieza una generación. `None` si ya hay otra activa.
    pub fn try_begin(&self) -> Option<GenerationGuard<'_>> {
        self.generating
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| GenerationGuard { runtime: self })
    }

    pub fn last_destination(&self) -> Option<PathBuf> {
        lock(&self.last_destination).clone()
    }

    pub fn set_last_destination(&self, destination: Option<PathBuf>) {
        *lock(&self.last_destination) = destination;
    }
}

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub data_dir: PathBuf,
    pub dictation: DictationRuntime,
    pub quote: QuoteRuntime,
    pub transcriber: Arc<dyn Transcriber>,
    pub recorder: Arc<dyn Recorder>,
}

impl AppState {
    /// Estado sin motor de voz ni micrófono: la escritura a mano sigue funcionando.
    pub fn new(conn: Connection, data_dir: PathBuf) -> Self {
        Self::with_engines(
            conn,
            data_dir,
            Arc::new(UnavailableTranscriber),
            Arc::new(UnavailableRecorder),
        )
    }

    pub fn with_engines(
        conn: Connection,
        data_dir: PathBuf,
        transcriber: Arc<dyn Transcriber>,
        recorder: Arc<dyn Recorder>,
    ) -> Self {
        Self {
            conn: Mutex::new(conn),
            data_dir,
            dictation: DictationRuntime::default(),
            quote: QuoteRuntime::default(),
            transcriber,
            recorder,
        }
    }

    /// Bloquea la conexión. Un mutex envenenado se recupera: SQLite garantiza la consistencia.
    pub fn conn(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        Ok(lock(&self.conn))
    }
}
