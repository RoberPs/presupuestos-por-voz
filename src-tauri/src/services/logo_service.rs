//! Validación y sustitución del logo (R3, FR-002, FR-003, FR-003b, FR-007).

use std::fs;
use std::path::Path;

use image::ImageFormat;
use sha2::{Digest, Sha256};

use crate::domain::profile::{LogoMeta, LogoSetResult, LogoView, LOW_RES_MIN_SIDE_PX};
use crate::error::{AppError, LogoRejectCode};
use crate::state::AppState;
use crate::storage::{logo_store, profile_repo};

pub const MAX_LOGO_BYTES: u64 = 5 * 1024 * 1024;

pub const LOW_RESOLUTION_WARNING: &str =
    "El logo es pequeño y puede verse borroso en los presupuestos.";

#[derive(Debug)]
pub struct ValidatedLogo {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub ext: &'static str,
    pub width_px: u32,
    pub height_px: u32,
    pub sha256: String,
}

impl ValidatedLogo {
    pub fn low_resolution(&self) -> bool {
        self.width_px.min(self.height_px) < LOW_RES_MIN_SIDE_PX
    }
}

/// Valida, en orden: tamaño, lectura, formato por *magic bytes*, decodificación completa.
pub fn validate_logo_file(path: &Path) -> Result<ValidatedLogo, AppError> {
    let meta = fs::metadata(path).map_err(|e| {
        log::warn!("No se puede leer el logo elegido: {e}");
        AppError::logo_rejected(LogoRejectCode::LogoCorrupt)
    })?;
    if meta.len() > MAX_LOGO_BYTES {
        return Err(AppError::logo_rejected(LogoRejectCode::LogoTooLarge));
    }
    let bytes = fs::read(path).map_err(|e| {
        log::warn!("No se puede leer el logo elegido: {e}");
        AppError::logo_rejected(LogoRejectCode::LogoCorrupt)
    })?;
    if bytes.len() as u64 > MAX_LOGO_BYTES {
        return Err(AppError::logo_rejected(LogoRejectCode::LogoTooLarge));
    }

    let (format, mime, ext) = match image::guess_format(&bytes) {
        Ok(ImageFormat::Png) => (ImageFormat::Png, "image/png", "png"),
        Ok(ImageFormat::Jpeg) => (ImageFormat::Jpeg, "image/jpeg", "jpg"),
        _ => {
            return Err(AppError::logo_rejected(
                LogoRejectCode::LogoUnsupportedFormat,
            ))
        }
    };
    let img = image::load_from_memory_with_format(&bytes, format).map_err(|e| {
        log::warn!("Logo dañado: {e}");
        AppError::logo_rejected(LogoRejectCode::LogoCorrupt)
    })?;
    let sha256 = hex::encode(Sha256::digest(&bytes));

    Ok(ValidatedLogo {
        width_px: img.width(),
        height_px: img.height(),
        bytes,
        mime,
        ext,
        sha256,
    })
}

pub fn logo_view(data_dir: &Path, meta: &LogoMeta) -> Result<LogoView, AppError> {
    Ok(LogoView {
        data_url: logo_store::read_data_url(data_dir, &meta.file, &meta.mime)?,
        mime: meta.mime.clone(),
        width_px: meta.width_px,
        height_px: meta.height_px,
        low_resolution: meta.low_resolution(),
    })
}

/// Sustituye el logo. Si algo falla, el logo anterior queda intacto.
pub fn set_logo(state: &AppState, path: &Path) -> Result<LogoSetResult, AppError> {
    let logo = validate_logo_file(path)?;
    let rel = logo_store::write_atomic(&state.data_dir, &logo.bytes, &logo.sha256, logo.ext)?;
    let meta = LogoMeta {
        file: rel.clone(),
        mime: logo.mime.to_string(),
        width_px: logo.width_px,
        height_px: logo.height_px,
        bytes: logo.bytes.len() as u64,
        sha256: logo.sha256.clone(),
    };

    let previous = {
        let mut conn = state.conn()?;
        let outcome = (|| -> rusqlite::Result<Option<String>> {
            let tx = conn.transaction()?;
            profile_repo::ensure_row(&tx)?;
            let previous = profile_repo::get(&tx)?.and_then(|p| p.logo).map(|l| l.file);
            profile_repo::set_logo(&tx, Some(&meta))?;
            tx.commit()?;
            Ok(previous)
        })();
        match outcome {
            Ok(previous) => previous,
            Err(e) => {
                // Quitar el archivo nuevo salvo que sea el mismo que ya estaba en uso
                let in_use = profile_repo::get(&conn).ok().flatten().and_then(|p| p.logo);
                if in_use.map(|l| l.file) != Some(rel.clone()) {
                    let _ = logo_store::delete(&state.data_dir, &rel);
                }
                return Err(e.into());
            }
        }
    };

    if let Some(old) = previous.filter(|old| *old != rel) {
        if let Err(e) = logo_store::delete(&state.data_dir, &old) {
            log::warn!("No se ha podido borrar el logo anterior ({old}): {e}");
        }
    }

    let warnings = if logo.low_resolution() {
        vec![LOW_RESOLUTION_WARNING.to_string()]
    } else {
        vec![]
    };
    Ok(LogoSetResult {
        logo: logo_view(&state.data_dir, &meta)?,
        warnings,
    })
}

/// Quita el logo del perfil y, tras confirmar la transacción, borra su archivo.
pub fn remove_logo(state: &AppState) -> Result<(), AppError> {
    let previous = {
        let mut conn = state.conn()?;
        let tx = conn.transaction()?;
        profile_repo::ensure_row(&tx)?;
        let previous = profile_repo::get(&tx)?.and_then(|p| p.logo).map(|l| l.file);
        profile_repo::set_logo(&tx, None)?;
        tx.commit()?;
        previous
    };
    if let Some(old) = previous {
        if let Err(e) = logo_store::delete(&state.data_dir, &old) {
            log::warn!("No se ha podido borrar el logo ({old}): {e}");
        }
    }
    Ok(())
}

/// Borra de `branding/` los logos y temporales que no son el logo vigente (p. ej. tras un
/// cierre inesperado). Devuelve cuántos archivos ha borrado.
pub fn cleanup_orphans(data_dir: &Path, conn: &rusqlite::Connection) -> Result<usize, AppError> {
    let dir = data_dir.join(logo_store::BRANDING_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.into()),
    };
    let current = profile_repo::get(conn)?
        .and_then(|p| p.logo)
        .and_then(|l| l.file.rsplit('/').next().map(str::to_string));

    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let ours = name.starts_with("logo-") || name.starts_with(".tmp-");
        if ours && Some(&name) != current.as_ref() && entry.path().is_file() {
            match fs::remove_file(entry.path()) {
                Ok(()) => removed += 1,
                Err(e) => log::warn!("No se ha podido borrar {name}: {e}"),
            }
        }
    }
    Ok(removed)
}
