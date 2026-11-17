use tauri::State;

use crate::domain::rate::UnitView;
use crate::error::AppError;
use crate::services::rate_service;
use crate::state::AppState;

#[tauri::command]
pub fn units_list(state: State<'_, AppState>) -> Result<Vec<UnitView>, AppError> {
    rate_service::units(&state)
}
