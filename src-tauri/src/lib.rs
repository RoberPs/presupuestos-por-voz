//! Núcleo de Presupuestos por Voz.
//!
//! Sin red ni servidor (constitución, Principios I y II): solo SQLite y archivos en `app_data_dir`.

pub mod commands;
pub mod domain;
pub mod error;
pub mod services;
pub mod state;
pub mod storage;

use std::path::PathBuf;

use tauri::Manager;

use crate::state::AppState;

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
            app.manage(AppState::new(conn, data_dir));
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
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación");
}
