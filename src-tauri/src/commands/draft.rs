use tauri::State;

use crate::error::AppError;
use crate::services::dictation_service::{self, SessionView};
use crate::services::draft_service::{
    self, DraftLineInput, DraftLinePatch, DraftView, GenerateMode,
};
use crate::state::AppState;

#[tauri::command]
pub fn draft_generate_from_text(
    state: State<'_, AppState>,
    text: String,
    mode: GenerateMode,
    confirmed: bool,
) -> Result<DraftView, AppError> {
    draft_service::generate_from_text(&state, &text, mode, confirmed)
}

#[tauri::command]
pub fn draft_text_save(state: State<'_, AppState>, text: String) -> Result<(), AppError> {
    draft_service::save_text(&state, &text)
}

#[tauri::command]
pub fn draft_line_add(
    state: State<'_, AppState>,
    input: DraftLineInput,
) -> Result<DraftView, AppError> {
    draft_service::line_add(&state, input)
}

#[tauri::command]
pub fn draft_line_update(
    state: State<'_, AppState>,
    line_id: i64,
    patch: DraftLinePatch,
) -> Result<DraftView, AppError> {
    draft_service::line_update(&state, line_id, patch)
}

#[tauri::command]
pub fn draft_line_assign_rate(
    state: State<'_, AppState>,
    line_id: i64,
    rate_id: i64,
) -> Result<DraftView, AppError> {
    draft_service::line_assign_rate(&state, line_id, rate_id)
}

#[tauri::command]
pub fn draft_line_delete(state: State<'_, AppState>, line_id: i64) -> Result<DraftView, AppError> {
    draft_service::line_delete(&state, line_id)
}

#[tauri::command]
pub fn draft_apply_current_vat(state: State<'_, AppState>) -> Result<DraftView, AppError> {
    draft_service::apply_current_vat(&state)
}

#[tauri::command]
pub fn draft_mark_reviewed(state: State<'_, AppState>) -> Result<DraftView, AppError> {
    draft_service::mark_reviewed(&state)
}

#[tauri::command]
pub fn draft_discard(state: State<'_, AppState>) -> Result<SessionView, AppError> {
    draft_service::discard(&state)?;
    dictation_service::session(&state)
}
