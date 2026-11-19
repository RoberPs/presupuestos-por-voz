//! Casos de uso de tarifas y unidades (FR-010…FR-022).

use rusqlite::{ErrorCode as SqliteCode, Transaction};

use crate::domain::normalize::{escape_like, normalize};
use crate::domain::rate::{self, RateInput, RateView, UnitView, ValidRate};
use crate::error::AppError;
use crate::state::AppState;
use crate::storage::{rate_repo, unit_repo};

const NOT_FOUND_MESSAGE: &str = "Esa tarifa ya no existe. Puede que se haya borrado.";

pub fn units(state: &AppState) -> Result<Vec<UnitView>, AppError> {
    Ok(unit_repo::list(&*state.conn()?)?)
}

/// Sin consulta (o vacía): todas. Con consulta: las que contienen el texto normalizado.
pub fn list(state: &AppState, query: Option<String>) -> Result<Vec<RateView>, AppError> {
    let conn = state.conn()?;
    let q = query.as_deref().map(normalize).unwrap_or_default();
    if q.is_empty() {
        Ok(rate_repo::list_all(&conn)?)
    } else {
        Ok(rate_repo::search(&conn, &escape_like(&q))?)
    }
}

fn duplicate_error(tx: &Transaction<'_>, existing_id: i64) -> AppError {
    let message = match rate_repo::get(tx, existing_id) {
        Ok(Some(r)) => format!(
            "Ya existe la tarifa «{}» por {}. Puedes editarla en lugar de crear otra.",
            r.concept, r.unit.label
        ),
        _ => "Ya existe una tarifa con ese concepto y esa unidad.".to_string(),
    };
    AppError::DuplicateRate {
        existing_rate_id: existing_id,
        message,
    }
}

fn resolve_unit(tx: &Transaction<'_>, v: &ValidRate) -> Result<i64, AppError> {
    match unit_repo::find_by_norm(tx, &v.unit_norm)? {
        Some(id) => Ok(id),
        None => Ok(unit_repo::insert_custom(tx, &v.unit_label, &v.unit_norm)?),
    }
}

fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(e, rusqlite::Error::SqliteFailure(f, _) if f.code == SqliteCode::ConstraintViolation)
}

/// Escribe la tarifa (`id = None` crea). Valida, resuelve o crea la unidad y comprueba duplicados.
fn write(state: &AppState, id: Option<i64>, input: RateInput) -> Result<RateView, AppError> {
    let v = rate::validate(&input).map_err(AppError::validation)?;
    let mut conn = state.conn()?;
    let tx = conn.transaction()?;

    if let Some(id) = id {
        if rate_repo::get(&tx, id)?.is_none() {
            return Err(AppError::not_found(NOT_FOUND_MESSAGE));
        }
    }
    let unit_id = resolve_unit(&tx, &v)?;
    if let Some(existing) = rate_repo::find_duplicate(&tx, &v.concept_norm, unit_id, id)? {
        return Err(duplicate_error(&tx, existing));
    }

    let written = match id {
        None => rate_repo::insert(&tx, &v.concept, &v.concept_norm, unit_id, v.price_cents),
        Some(id) => rate_repo::update(&tx, id, &v.concept, &v.concept_norm, unit_id, v.price_cents)
            .map(|_| id),
    };
    let rate_id = match written {
        Ok(rate_id) => rate_id,
        // Protección extra: la restricción UNIQUE de la base de datos.
        Err(e) if is_unique_violation(&e) => {
            let existing = rate_repo::find_duplicate(&tx, &v.concept_norm, unit_id, id)?;
            return Err(match existing {
                Some(existing) => duplicate_error(&tx, existing),
                None => e.into(),
            });
        }
        Err(e) => return Err(e.into()),
    };

    let view = rate_repo::get(&tx, rate_id)?.ok_or_else(AppError::storage)?;
    tx.commit()?;
    Ok(view)
}

pub fn create(state: &AppState, input: RateInput) -> Result<RateView, AppError> {
    write(state, None, input)
}

pub fn update(state: &AppState, id: i64, input: RateInput) -> Result<RateView, AppError> {
    write(state, Some(id), input)
}

pub fn delete(state: &AppState, id: i64) -> Result<(), AppError> {
    match rate_repo::delete(&*state.conn()?, id)? {
        0 => Err(AppError::not_found(NOT_FOUND_MESSAGE)),
        _ => Ok(()),
    }
}
