use tauri::State;

use crate::domain::profile::{BusinessProfileInput, BusinessProfileView};
use crate::error::{AppError, FieldError};
use crate::services::profile_service;
use crate::state::AppState;

#[tauri::command]
pub fn profile_get(state: State<'_, AppState>) -> Result<Option<BusinessProfileView>, AppError> {
    profile_service::get_profile(&state)
}

#[tauri::command]
pub fn profile_save(
    state: State<'_, AppState>,
    input: BusinessProfileInput,
) -> Result<BusinessProfileView, AppError> {
    profile_service::save_profile(&state, input)
}

#[tauri::command]
pub fn profile_validate(input: BusinessProfileInput) -> Vec<FieldError> {
    profile_service::validate_profile(&input)
}
