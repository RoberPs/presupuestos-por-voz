use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::audio::{RecordingEvent, RecordingSink};
use crate::error::AppError;
use crate::services::dictation_service::{self, ProcessResult, SessionView};
use crate::state::{AppState, ProcessingSink};
use crate::transcription::ProcessingEvent;

fn recording_sink(channel: Channel<RecordingEvent>) -> RecordingSink {
    Arc::new(move |event| {
        if let Err(e) = channel.send(event) {
            log::warn!("No se ha podido enviar el evento de grabación: {e}");
        }
    })
}

fn processing_sink(channel: Channel<ProcessingEvent>) -> ProcessingSink {
    Arc::new(move |event| {
        if let Err(e) = channel.send(event) {
            log::warn!("No se ha podido enviar el evento del proceso: {e}");
        }
    })
}

fn task_error(e: tauri::Error) -> AppError {
    log::error!("Fallo en la tarea del dictado: {e}");
    AppError::storage()
}

#[tauri::command]
pub fn dictation_start(
    state: State<'_, AppState>,
    confirm_replace: bool,
    on_event: Channel<RecordingEvent>,
) -> Result<(), AppError> {
    dictation_service::start(&state, confirm_replace, Some(recording_sink(on_event)))
}

#[tauri::command]
pub async fn dictation_stop(app: AppHandle) -> Result<SessionView, AppError> {
    tauri::async_runtime::spawn_blocking(move || dictation_service::stop(&app.state::<AppState>()))
        .await
        .map_err(task_error)?
}

/// Abre el diálogo nativo (M4A, MP3 y WAV). `None` si el usuario cancela.
#[tauri::command]
pub async fn dictation_import(
    app: AppHandle,
    confirm_replace: bool,
) -> Result<Option<SessionView>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        dictation_service::check_can_replace(&state, confirm_replace)?;

        let picked = app
            .dialog()
            .file()
            .set_title("Importar un audio")
            .add_filter("Audio", &["m4a", "mp3", "wav"])
            .blocking_pick_file();
        let Some(file) = picked else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|e| {
            log::error!("Ruta de archivo no válida: {e}");
            AppError::storage()
        })?;
        dictation_service::import_file(&state, &path, true).map(Some)
    })
    .await
    .map_err(task_error)?
}

#[tauri::command]
pub async fn dictation_process(
    app: AppHandle,
    on_progress: Channel<ProcessingEvent>,
) -> Result<ProcessResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        dictation_service::process(&app.state::<AppState>(), Some(processing_sink(on_progress)))
    })
    .await
    .map_err(task_error)?
}

#[tauri::command]
pub fn dictation_cancel_processing(state: State<'_, AppState>) {
    dictation_service::cancel(&state);
}

#[tauri::command]
pub fn dictation_subscribe(
    state: State<'_, AppState>,
    on_recording: Channel<RecordingEvent>,
    on_progress: Channel<ProcessingEvent>,
) {
    dictation_service::subscribe(
        &state,
        recording_sink(on_recording),
        processing_sink(on_progress),
    );
}

#[tauri::command]
pub fn dictation_recover(state: State<'_, AppState>) -> Result<SessionView, AppError> {
    dictation_service::recover(&state)
}

#[tauri::command]
pub fn dictation_discard(state: State<'_, AppState>) -> Result<SessionView, AppError> {
    dictation_service::discard(&state)
}
