use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::domain::profile::{BusinessProfileView, LogoSetResult};
use crate::error::AppError;
use crate::services::{logo_service, profile_service};
use crate::state::AppState;

/// Abre el diálogo nativo (PNG/JPG) y guarda el logo elegido. `None` si el usuario cancela.
#[tauri::command]
pub async fn logo_pick_and_set(app: AppHandle) -> Result<Option<LogoSetResult>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app
            .dialog()
            .file()
            .set_title("Elegir logo")
            .add_filter("Imagen", &["png", "jpg", "jpeg"])
            .blocking_pick_file();
        let Some(file) = picked else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|e| {
            log::error!("Ruta de archivo no válida: {e}");
            AppError::storage()
        })?;
        let state = app.state::<AppState>();
        logo_service::set_logo(&state, &path).map(Some)
    })
    .await
    .map_err(|e| {
        log::error!("Fallo en la tarea del logo: {e}");
        AppError::storage()
    })?
}

/// Quita el logo. La interfaz pide confirmación antes de llamarlo.
#[tauri::command]
pub fn logo_remove(state: State<'_, AppState>) -> Result<BusinessProfileView, AppError> {
    logo_service::remove_logo(&state)?;
    profile_service::get_profile(&state)?.ok_or_else(AppError::storage)
}
