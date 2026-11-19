//! Comandos IPC del PDF del presupuesto (contracts/ipc-commands.md).

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::domain::quote_doc::details::QuoteDetailsInput;
use crate::error::AppError;
use crate::services::quote_service::{self, QuoteGenerateResult, QuotePreparationView};
use crate::state::AppState;

#[tauri::command]
pub fn quote_prepare(
    state: State<'_, AppState>,
    allow_missing_logo: bool,
) -> Result<QuotePreparationView, AppError> {
    quote_service::prepare(&state, allow_missing_logo)
}

#[tauri::command]
pub fn quote_details_save(
    state: State<'_, AppState>,
    details: QuoteDetailsInput,
    allow_missing_logo: bool,
) -> Result<QuotePreparationView, AppError> {
    quote_service::save_details(&state, details, allow_missing_logo)
}

/// Es la confirmación explícita del profesional (FR-004). El diálogo se abre desde Rust, así que no
/// hace falta ningún permiso nuevo (R7).
#[tauri::command]
pub async fn quote_generate(
    app: AppHandle,
    confirmation_token: String,
    allow_missing_logo: bool,
) -> Result<QuoteGenerateResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _generation = quote_service::begin_generation(&state)?;
        quote_service::check_confirmation_on(
            &state,
            &confirmation_token,
            allow_missing_logo,
            quote_service::today(),
        )?;

        let mut dialog = app
            .dialog()
            .file()
            .set_title("Guardar el presupuesto")
            .add_filter("PDF", &["pdf"]);
        if let Some(window) = app.get_webview_window("main") {
            dialog = dialog.set_parent(&window);
        }
        // Tras un fallo de guardado se reabre con la última carpeta y el último nombre (FR-023).
        match state.quote.last_destination() {
            Some(previous) => {
                if let Some(folder) = previous.parent() {
                    dialog = dialog.set_directory(folder);
                }
                if let Some(name) = previous.file_name() {
                    dialog = dialog.set_file_name(name.to_string_lossy());
                }
            }
            None => {
                dialog = dialog.set_file_name(quote_service::proposed_file_name_on(
                    &state,
                    quote_service::today(),
                    allow_missing_logo,
                )?);
            }
        }

        let Some(file) = dialog.blocking_save_file() else {
            return Ok(QuoteGenerateResult::Cancelled);
        };
        let path = file.into_path().map_err(|_| {
            log::error!("La ubicación elegida para el PDF no es válida");
            AppError::storage()
        })?;

        let saved = quote_service::save_to_path_on(
            &state,
            &confirmation_token,
            allow_missing_logo,
            &path,
            quote_service::today(),
        );
        match &saved {
            Ok(_) => state.quote.set_last_destination(None),
            Err(AppError::PdfSave { .. }) => state.quote.set_last_destination(Some(path)),
            Err(_) => {}
        }
        saved.map(QuoteGenerateResult::from)
    })
    .await
    .map_err(|_| {
        log::error!("Fallo en la tarea de generación del PDF");
        AppError::pdf_render()
    })?
}
