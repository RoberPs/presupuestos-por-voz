//! Perfil del negocio: lectura, guardado de borradores y validación (FR-001…FR-008).

use std::path::Path;

use crate::domain::money;
use crate::domain::profile::{self, BusinessProfile, BusinessProfileInput, BusinessProfileView};
use crate::error::{AppError, FieldError};
use crate::services::logo_service;
use crate::state::AppState;
use crate::storage::profile_repo;

pub fn build_view(data_dir: &Path, p: BusinessProfile) -> BusinessProfileView {
    let missing = profile::missing_fields(&p);
    let logo = p
        .logo
        .as_ref()
        .and_then(|meta| match logo_service::logo_view(data_dir, meta) {
            Ok(view) => Some(view),
            Err(_) => {
                log::error!("No se encuentra el archivo del logo: {}", meta.file);
                None
            }
        });
    BusinessProfileView {
        trade_name: p.trade_name,
        legal_name: p.legal_name,
        tax_id: p.tax_id,
        phone: p.phone,
        email: p.email,
        website: p.website,
        address: p.address,
        vat_rate_percent: money::format_percent_bp(p.vat_rate_bp),
        currency: "EUR",
        logo,
        status: profile::status(&missing),
        missing_fields: missing,
    }
}

/// `None` en el primer arranque (no hay fila): activa el modo guiado.
pub fn get_profile(state: &AppState) -> Result<Option<BusinessProfileView>, AppError> {
    let stored = profile_repo::get(&*state.conn()?)?;
    Ok(stored.map(|p| build_view(&state.data_dir, p)))
}

/// Guarda aunque falten datos (FR-008). Solo rechaza textos demasiado largos e IVA fuera de rango.
pub fn save_profile(
    state: &AppState,
    input: BusinessProfileInput,
) -> Result<BusinessProfileView, AppError> {
    let hard = profile::hard_errors(&input);
    if !hard.is_empty() {
        return Err(AppError::validation(hard));
    }
    let data = profile::from_input(&input);
    let saved = {
        let mut conn = state.conn()?;
        let tx = conn.transaction()?;
        profile_repo::upsert(&tx, &data)?;
        let saved = profile_repo::get(&tx)?.ok_or_else(AppError::storage)?;
        tx.commit()?;
        saved
    };
    Ok(build_view(&state.data_dir, saved))
}

/// Errores sin guardar: los que bloquean y los que dejan el perfil incompleto (uno por campo).
pub fn validate_profile(input: &BusinessProfileInput) -> Vec<FieldError> {
    let mut errors = profile::hard_errors(input);
    for e in profile::missing_fields(&profile::from_input(input)) {
        if !errors.iter().any(|x| x.field == e.field) {
            errors.push(e);
        }
    }
    errors
}
