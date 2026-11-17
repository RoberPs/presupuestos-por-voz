use tauri::State;

use crate::domain::rate::{RateInput, RateView};
use crate::error::AppError;
use crate::services::rate_service;
use crate::state::AppState;

#[tauri::command]
pub fn rates_list(
    state: State<'_, AppState>,
    query: Option<String>,
) -> Result<Vec<RateView>, AppError> {
    rate_service::list(&state, query)
}

#[tauri::command]
pub fn rates_create(state: State<'_, AppState>, input: RateInput) -> Result<RateView, AppError> {
    rate_service::create(&state, input)
}

#[tauri::command]
pub fn rates_update(
    state: State<'_, AppState>,
    id: i64,
    input: RateInput,
) -> Result<RateView, AppError> {
    rate_service::update(&state, id, input)
}

#[tauri::command]
pub fn rates_delete(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    rate_service::delete(&state, id)
}
