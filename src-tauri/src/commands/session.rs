use tauri::State;

use crate::error::AppError;
use crate::services::dictation_service::{self, SessionView};
use crate::state::AppState;

/// Estado completo al abrir la pantalla, incluida una grabación recuperable.
#[tauri::command]
pub fn session_get(state: State<'_, AppState>) -> Result<SessionView, AppError> {
    dictation_service::session(&state)
}
