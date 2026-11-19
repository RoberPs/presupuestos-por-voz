//! Casos de uso del dictado: grabar, importar, procesar, cancelar, recuperar y descartar.
//!
//! El borrador anterior no se sustituye hasta que el nuevo está listo, en una única transacción
//! (SC-008). La conexión SQLite nunca queda bloqueada mientras se transcribe.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use rusqlite::Connection;
use serde::Serialize;

use crate::audio::decode::decode_to_wav;
use crate::audio::quality::has_speech;
use crate::audio::{wav, RecordingSink, MAX_DURATION_MS};
use crate::domain::draft::TextOrigin;
use crate::error::{AppError, ConfirmationReason, TranscriptionCode};
use crate::services::draft_service::{self, DraftView};
use crate::state::{Activity, AppState, ProcessingSink};
use crate::storage::dictation_repo::{self, Dictation, DictationOrigin, DictationStatus, Slot};
use crate::storage::{draft_repo, rate_repo, recording_store};
use crate::transcription::{
    ProcessingEvent, ProcessingOutcome, ProcessingStage, TranscribeFailure, TranscribeOptions,
    Transcript,
};

const BUSY_MESSAGE: &str = "Espera a que termine el dictado en curso.";
const NO_AUDIO_MESSAGE: &str = "No hay ningún audio pendiente de procesar.";
const NOT_RECORDING_MESSAGE: &str = "No hay ninguna grabación en curso.";
const NO_RECOVERY_MESSAGE: &str = "No hay ninguna grabación que recuperar.";
/// Longitud máxima del vocabulario que se pasa al motor (R11).
const MAX_VOCABULARY_CHARS: usize = 600;
const MAX_TEXT_CHARS: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ActivityView {
    Idle,
    Recording { elapsed_ms: i64 },
    Processing { percent: Option<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationView {
    pub origin: DictationOrigin,
    pub original_file_name: Option<String>,
    pub duration_ms: i64,
    pub status: DictationStatus,
    pub low_quality: bool,
    pub error_code: Option<TranscriptionCode>,
    pub transcribed_locally: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryView {
    pub duration_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub activity: ActivityView,
    pub dictation: Option<DictationView>,
    pub draft: Option<DraftView>,
    pub pending_text: Option<String>,
    pub recovery: Option<RecoveryView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessOutcome {
    DraftReady,
    NoSpeech,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessResult {
    pub outcome: ProcessOutcome,
    pub session: SessionView,
}

/// Devuelve la actividad a `Idle` al terminar, también si hay un error.
struct IdleGuard<'a>(&'a AppState);

impl Drop for IdleGuard<'_> {
    fn drop(&mut self) {
        self.0.dictation.set_activity(Activity::Idle);
    }
}

fn busy() -> AppError {
    AppError::invalid_state(BUSY_MESSAGE)
}

fn dictation_view(dictation: Dictation) -> DictationView {
    DictationView {
        transcribed_locally: dictation.status == DictationStatus::Transcribed,
        origin: dictation.origin,
        original_file_name: dictation.original_file_name,
        duration_ms: dictation.duration_ms,
        status: dictation.status,
        low_quality: dictation.low_quality,
        error_code: dictation.error_code,
    }
}

/// Grabación interrumpida con audio que se puede recuperar.
fn recovery(state: &AppState) -> Option<RecoveryView> {
    if matches!(state.dictation.activity(), Activity::Recording { .. }) {
        return None;
    }
    let part = recording_store::part_path(&state.data_dir);
    match wav::duration_from_size(&part) {
        Ok(duration_ms) if duration_ms > 0 => Some(RecoveryView { duration_ms }),
        _ => None,
    }
}

/// Estado completo de la pantalla.
pub fn session(state: &AppState) -> Result<SessionView, AppError> {
    let activity = match state.dictation.activity() {
        Activity::Idle => ActivityView::Idle,
        Activity::Recording { started } => ActivityView::Recording {
            elapsed_ms: i64::try_from(started.elapsed().as_millis()).unwrap_or(MAX_DURATION_MS),
        },
        Activity::Processing { percent } => ActivityView::Processing { percent },
    };

    let conn = state.conn()?;
    let dictation = match dictation_repo::get(&conn, Slot::Incoming)? {
        Some(incoming) => Some(incoming),
        None => dictation_repo::get(&conn, Slot::Current)?,
    };
    Ok(SessionView {
        activity,
        dictation: dictation.map(dictation_view),
        draft: draft_service::load_view(&conn)?,
        pending_text: draft_repo::pending_text(&conn)?,
        recovery: recovery(state),
    })
}

/// Comprueba que se puede empezar un audio nuevo sin perder nada sin avisar.
pub fn check_can_replace(state: &AppState, confirmed: bool) -> Result<(), AppError> {
    if state.dictation.activity() != Activity::Idle {
        return Err(busy());
    }
    if confirmed {
        return Ok(());
    }
    let conn = state.conn()?;
    if dictation_repo::get(&conn, Slot::Incoming)?.is_some() || recovery(state).is_some() {
        return Err(AppError::confirmation_required(
            ConfirmationReason::ReplacePendingAudio,
        ));
    }
    if draft_repo::exists(&conn)? {
        return Err(AppError::confirmation_required(
            ConfirmationReason::ReplaceDraft,
        ));
    }
    Ok(())
}

/// Borra la ranura `incoming` y su audio, si existen.
fn clear_incoming(conn: &Connection, state: &AppState) -> Result<(), AppError> {
    if let Some(incoming) = dictation_repo::get(conn, Slot::Incoming)? {
        dictation_repo::delete(conn, Slot::Incoming)?;
        recording_store::remove_audio(&state.data_dir, &incoming.audio_file)?;
    }
    Ok(())
}

fn new_incoming(
    audio_file: String,
    duration_ms: i64,
    origin: DictationOrigin,
    original_file_name: Option<String>,
) -> Dictation {
    Dictation {
        audio_file,
        duration_ms: duration_ms.min(MAX_DURATION_MS),
        origin,
        original_file_name,
        status: DictationStatus::Recorded,
        low_quality: false,
        error_code: None,
    }
}

/// Empieza a grabar. El borrador actual no se toca hasta generar el nuevo.
pub fn start(
    state: &AppState,
    confirm_replace: bool,
    sink: Option<RecordingSink>,
) -> Result<(), AppError> {
    check_can_replace(state, confirm_replace)?;
    let started = Activity::Recording {
        started: Instant::now(),
    };
    if !state.dictation.try_begin(started) {
        return Err(busy());
    }

    let begin = || -> Result<(), AppError> {
        recording_store::ensure_dir(&state.data_dir)?;
        if sink.is_some() {
            state.dictation.events.set_recording(sink);
        }
        let events = state.dictation.events.clone();
        let forward: RecordingSink = Arc::new(move |event| events.emit_recording(event));
        let part = recording_store::part_path(&state.data_dir);
        let recording = state.recorder.start(&part, forward)?;
        state.dictation.set_recording(recording);
        clear_incoming(&*state.conn()?, state)
    };
    begin().inspect_err(|_| state.dictation.set_activity(Activity::Idle))
}

/// Cierra la grabación y crea la ranura `incoming`. Sin audio, no crea nada.
pub fn stop(state: &AppState) -> Result<SessionView, AppError> {
    if !matches!(state.dictation.activity(), Activity::Recording { .. }) {
        return Err(AppError::invalid_state(NOT_RECORDING_MESSAGE));
    }
    let guard = IdleGuard(state);
    let recording = state
        .dictation
        .take_recording()
        .ok_or_else(|| AppError::invalid_state(NOT_RECORDING_MESSAGE))?;
    let duration_ms = recording.stop()?;

    if duration_ms <= 0 {
        recording_store::remove_part(&state.data_dir)?;
    } else {
        store_recording(state, duration_ms)?;
    }
    drop(guard);
    session(state)
}

/// Convierte la grabación temporal en la ranura `incoming`.
fn store_recording(state: &AppState, duration_ms: i64) -> Result<(), AppError> {
    let audio_file = recording_store::new_audio_file(&state.data_dir);
    recording_store::promote_part(&state.data_dir, &audio_file)?;
    let conn = state.conn()?;
    clear_incoming(&conn, state)?;
    dictation_repo::put(
        &conn,
        Slot::Incoming,
        &new_incoming(audio_file, duration_ms, DictationOrigin::Recorded, None),
    )?;
    Ok(())
}

/// Valida y normaliza un audio. Si se rechaza, todo queda como estaba (FR-002c).
pub fn import_file(
    state: &AppState,
    src: &Path,
    confirm_replace: bool,
) -> Result<SessionView, AppError> {
    check_can_replace(state, confirm_replace)?;
    if !state
        .dictation
        .try_begin(Activity::Processing { percent: None })
    {
        return Err(busy());
    }
    let guard = IdleGuard(state);

    recording_store::ensure_dir(&state.data_dir)?;
    let audio_file = recording_store::new_audio_file(&state.data_dir);
    let dest = recording_store::absolute(&state.data_dir, &audio_file);
    let duration_ms = decode_to_wav(src, &dest)?;

    {
        let conn = state.conn()?;
        clear_incoming(&conn, state)?;
        recording_store::remove_part(&state.data_dir)?;
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned());
        dictation_repo::put(
            &conn,
            Slot::Incoming,
            &new_incoming(audio_file, duration_ms, DictationOrigin::Imported, name),
        )?;
    }
    drop(guard);
    session(state)
}

/// Conceptos de las tarifas, como contexto para el motor. No influye en los precios.
fn vocabulary(conn: &Connection) -> Result<String, AppError> {
    let mut text = String::new();
    for rate in rate_repo::list_all(conn)? {
        if text.chars().count() + rate.concept.chars().count() + 2 > MAX_VOCABULARY_CHARS {
            break;
        }
        if !text.is_empty() {
            text.push_str(", ");
        }
        text.push_str(&rate.concept);
    }
    Ok(text)
}

/// Sustituye el borrador en una única transacción y conserva el audio del nuevo.
fn apply_transcript(state: &AppState, transcript: &Transcript) -> Result<(), AppError> {
    let source_text: String = transcript.text.chars().take(MAX_TEXT_CHARS).collect();
    let previous_audio = {
        let mut conn = state.conn()?;
        let tx = conn.transaction()?;
        let built = draft_service::build_from_text(&tx, &source_text)?;
        let previous = dictation_repo::get(&tx, Slot::Current)?;
        draft_repo::replace(&tx, &built, &source_text, TextOrigin::Transcribed)?;
        dictation_repo::promote_incoming(&tx)?;
        dictation_repo::set_status(&tx, Slot::Current, DictationStatus::Transcribed, None)?;
        dictation_repo::set_low_quality(&tx, Slot::Current, transcript.low_quality)?;
        tx.commit()?;
        previous
    };
    if let Some(previous) = previous_audio {
        recording_store::remove_audio(&state.data_dir, &previous.audio_file)?;
    }
    Ok(())
}

fn set_incoming_status(
    state: &AppState,
    status: DictationStatus,
    error_code: Option<TranscriptionCode>,
) -> Result<(), AppError> {
    dictation_repo::set_status(&*state.conn()?, Slot::Incoming, status, error_code)?;
    Ok(())
}

/// Registra el fallo conservando el audio, para poder reintentar (FR-004).
fn fail(state: &AppState, code: TranscriptionCode) -> AppError {
    if let Err(e) = set_incoming_status(state, DictationStatus::Failed, Some(code)) {
        log::error!("No se ha podido registrar el fallo del dictado: {e}");
    }
    AppError::transcription(code)
}

fn run_process(state: &AppState, incoming: &Dictation) -> Result<ProcessOutcome, AppError> {
    let events = &state.dictation.events;
    set_incoming_status(state, DictationStatus::Processing, None)?;
    events.emit_processing(ProcessingEvent::Stage {
        stage: ProcessingStage::Preparing,
    });

    let path = recording_store::absolute(&state.data_dir, &incoming.audio_file);
    let samples = wav::read_samples(&path).map_err(|e| {
        log::error!("No se ha podido leer el audio guardado: {e}");
        fail(state, TranscriptionCode::TranscriptionFailed)
    })?;
    if !has_speech(&samples) {
        set_incoming_status(state, DictationStatus::NoSpeech, None)?;
        return Ok(ProcessOutcome::NoSpeech);
    }

    let options = TranscribeOptions {
        vocabulary: vocabulary(&*state.conn()?)?,
    };
    events.emit_processing(ProcessingEvent::Stage {
        stage: ProcessingStage::Transcribing,
    });
    let progress = {
        let events = events.clone();
        move |percent: u8| events.emit_processing(ProcessingEvent::Progress { percent })
    };
    let progress = |percent: u8| {
        state.dictation.set_activity(Activity::Processing {
            percent: Some(percent),
        });
        progress(percent);
    };

    let transcript =
        match state
            .transcriber
            .transcribe(&samples, &options, &progress, &state.dictation.cancel)
        {
            Ok(transcript) => transcript,
            Err(TranscribeFailure::Cancelled) => {
                set_incoming_status(state, DictationStatus::Recorded, None)?;
                return Ok(ProcessOutcome::Cancelled);
            }
            Err(TranscribeFailure::Failed(code)) => return Err(fail(state, code)),
        };

    if transcript.text.trim().is_empty() {
        set_incoming_status(state, DictationStatus::NoSpeech, None)?;
        return Ok(ProcessOutcome::NoSpeech);
    }

    events.emit_processing(ProcessingEvent::Stage {
        stage: ProcessingStage::BuildingDraft,
    });
    apply_transcript(state, &transcript).map_err(|e| {
        log::error!("No se ha podido generar el borrador: {e}");
        fail(state, TranscriptionCode::TranscriptionFailed)
    })?;
    Ok(ProcessOutcome::DraftReady)
}

/// Transcribe la ranura `incoming` y, si hay texto, genera el borrador.
pub fn process(state: &AppState, sink: Option<ProcessingSink>) -> Result<ProcessResult, AppError> {
    if sink.is_some() {
        state.dictation.events.set_processing(sink);
    }
    if !state
        .dictation
        .try_begin(Activity::Processing { percent: None })
    {
        return Err(busy());
    }
    let guard = IdleGuard(state);
    state.dictation.cancel.store(false, Ordering::SeqCst);

    let incoming = dictation_repo::get(&*state.conn()?, Slot::Incoming)?
        .ok_or_else(|| AppError::invalid_state(NO_AUDIO_MESSAGE))?;

    let result = run_process(state, &incoming);
    drop(guard);

    let finished = match &result {
        Ok(ProcessOutcome::DraftReady) => ProcessingOutcome::DraftReady,
        Ok(ProcessOutcome::NoSpeech) => ProcessingOutcome::NoSpeech,
        Ok(ProcessOutcome::Cancelled) => ProcessingOutcome::Cancelled,
        Err(_) => ProcessingOutcome::Failed,
    };
    state
        .dictation
        .events
        .emit_processing(ProcessingEvent::Finished { outcome: finished });

    Ok(ProcessResult {
        outcome: result?,
        session: session(state)?,
    })
}

/// Pide cancelar el proceso en curso. El audio se conserva (FR-009).
pub fn cancel(state: &AppState) {
    if matches!(state.dictation.activity(), Activity::Processing { .. }) {
        state.dictation.cancel.store(true, Ordering::SeqCst);
    }
}

/// Sustituye los canales de eventos al volver a la pantalla.
pub fn subscribe(state: &AppState, recording: RecordingSink, processing: ProcessingSink) {
    state.dictation.events.set_recording(Some(recording));
    state.dictation.events.set_processing(Some(processing));
}

/// Convierte la grabación interrumpida en una ranura `incoming`.
pub fn recover(state: &AppState) -> Result<SessionView, AppError> {
    if state.dictation.activity() != Activity::Idle {
        return Err(busy());
    }
    let part = recording_store::part_path(&state.data_dir);
    if !part.exists() {
        return Err(AppError::invalid_state(NO_RECOVERY_MESSAGE));
    }
    let duration_ms = wav::repair(&part)?;
    if duration_ms <= 0 {
        recording_store::remove_part(&state.data_dir)?;
    } else {
        store_recording(state, duration_ms)?;
    }
    session(state)
}

/// Descarta la grabación recuperable o la ranura `incoming`. El borrador vigente no cambia.
pub fn discard(state: &AppState) -> Result<SessionView, AppError> {
    if state.dictation.activity() != Activity::Idle {
        return Err(busy());
    }
    recording_store::remove_part(&state.data_dir)?;
    clear_incoming(&*state.conn()?, state)?;
    session(state)
}
