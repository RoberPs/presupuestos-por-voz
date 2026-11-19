//! Núcleo de Presupuestos por Voz.
//!
//! Sin red ni servidor (constitución, Principios I y II): solo SQLite y archivos en `app_data_dir`.

pub mod audio;
pub mod commands;
pub mod domain;
pub mod error;
pub mod pdf;
pub mod services;
pub mod state;
pub mod storage;
pub mod transcription;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

use crate::audio::capture::MicRecorder;
use crate::state::AppState;
use crate::transcription::Transcriber;

/// Variable de entorno para usar otra carpeta de datos. Solo se respeta en builds de depuración
/// (pruebas e2e y semillas de desarrollo).
pub const DATA_DIR_ENV: &str = "PRESUPUESTOS_DATA_DIR";

fn resolve_data_dir(app: &tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        if let Ok(dir) = std::env::var(DATA_DIR_ENV) {
            if !dir.trim().is_empty() {
                return Ok(PathBuf::from(dir));
            }
        }
    }
    Ok(app.path().app_data_dir()?)
}

/// Motor de transcripción local con el modelo incluido en el instalador.
#[cfg(feature = "local-transcription")]
fn local_transcriber(app: &tauri::App) -> Arc<dyn Transcriber> {
    use tauri::path::BaseDirectory;

    match app.path().resolve(
        "resources/models/ggml-small-q5_1.bin",
        BaseDirectory::Resource,
    ) {
        Ok(path) => Arc::new(transcription::whisper::WhisperTranscriber::new(path)),
        Err(e) => {
            log::error!("No se ha podido localizar el modelo de voz: {e}");
            Arc::new(transcription::UnavailableTranscriber)
        }
    }
}

#[cfg(not(feature = "local-transcription"))]
fn local_transcriber(_app: &tauri::App) -> Arc<dyn Transcriber> {
    Arc::new(transcription::UnavailableTranscriber)
}

/// Al arrancar: un proceso interrumpido vuelve a `recorded` y se borran los audios sin usar.
fn recover_dictations(conn: &rusqlite::Connection, data_dir: &std::path::Path) {
    if let Err(e) = storage::dictation_repo::reset_processing(conn) {
        log::warn!("No se ha podido recuperar el dictado interrumpido: {e}");
    }
    let cleaned = storage::dictation_repo::audio_files(conn)
        .map_err(error::AppError::from)
        .and_then(|files| storage::recording_store::cleanup_orphans(data_dir, &files));
    match cleaned {
        Ok(0) => {}
        Ok(n) => log::info!("Limpieza: {n} audio(s) sin usar borrados"),
        Err(e) => log::warn!("No se ha podido limpiar la carpeta de audios: {e}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: None,
                    }),
                ])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = resolve_data_dir(app)?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = storage::db::open(&data_dir)?;
            match services::logo_service::cleanup_orphans(&data_dir, &conn) {
                Ok(0) => {}
                Ok(n) => log::info!("Limpieza: {n} archivo(s) de logo sin usar borrados"),
                Err(e) => log::warn!("No se ha podido limpiar la carpeta del logo: {e}"),
            }
            recover_dictations(&conn, &data_dir);
            let transcriber = local_transcriber(app);
            app.manage(AppState::with_engines(
                conn,
                data_dir,
                transcriber,
                Arc::new(MicRecorder),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::profile::profile_get,
            commands::profile::profile_save,
            commands::profile::profile_validate,
            commands::logo::logo_pick_and_set,
            commands::logo::logo_remove,
            commands::units::units_list,
            commands::rates::rates_list,
            commands::rates::rates_create,
            commands::rates::rates_update,
            commands::rates::rates_delete,
            commands::session::session_get,
            commands::dictation::dictation_start,
            commands::dictation::dictation_stop,
            commands::dictation::dictation_import,
            commands::dictation::dictation_process,
            commands::dictation::dictation_cancel_processing,
            commands::dictation::dictation_subscribe,
            commands::dictation::dictation_recover,
            commands::dictation::dictation_discard,
            commands::draft::draft_generate_from_text,
            commands::draft::draft_text_save,
            commands::draft::draft_line_add,
            commands::draft::draft_line_update,
            commands::draft::draft_line_assign_rate,
            commands::draft::draft_line_delete,
            commands::draft::draft_apply_current_vat,
            commands::draft::draft_mark_reviewed,
            commands::draft::draft_discard,
            commands::quote::quote_prepare,
            commands::quote::quote_details_save,
            commands::quote::quote_generate,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación");
}
