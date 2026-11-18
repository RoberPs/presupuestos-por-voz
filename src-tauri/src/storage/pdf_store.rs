//! Escritura atómica del PDF en la ubicación elegida (research.md, R8).

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::error::{AppError, PdfSaveCode};

/// Temporal de nombre corto y fijo en la misma carpeta: no alarga la ruta y la siguiente generación
/// sustituye el que deje un cierre inesperado.
pub const TEMP_FILE_NAME: &str = ".presupuesto.tmp";

/// Paso de la escritura en el que se ha producido un fallo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStage {
    CreateTemp,
    WriteTemp,
    Rename,
}

/// Traduce un error de Windows al código del contrato. Un error 5 al renombrar indica que el destino
/// está bloqueado, porque crear el temporal ya demostró que hay permiso en la carpeta.
pub fn classify(stage: SaveStage, os_code: Option<i32>) -> PdfSaveCode {
    match (stage, os_code) {
        (SaveStage::CreateTemp, Some(5)) => PdfSaveCode::PdfAccessDenied,
        (SaveStage::Rename, Some(5 | 32 | 33)) => PdfSaveCode::PdfFileInUse,
        (_, Some(39 | 112)) => PdfSaveCode::PdfDiskFull,
        (_, Some(3 | 15 | 21 | 53 | 67 | 1167)) => PdfSaveCode::PdfLocationUnavailable,
        _ => PdfSaveCode::PdfWriteFailed,
    }
}

fn write_and_rename(
    temp: &Path,
    destination: &Path,
    bytes: &[u8],
) -> Result<(), (SaveStage, io::Error)> {
    let mut file = fs::File::create(temp).map_err(|error| (SaveStage::CreateTemp, error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| (SaveStage::WriteTemp, error))?;
    drop(file);
    fs::rename(temp, destination).map_err(|error| (SaveStage::Rename, error))
}

/// Escribe `bytes` en un temporal, lo vuelca a disco y lo renombra a `destination`. Ante cualquier
/// fallo borra el temporal y deja intacto el archivo anterior (FR-024). El registro técnico solo
/// recibe el código del sistema, nunca la ruta (FR-028b).
pub fn write_atomic(destination: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let folder = destination.parent().unwrap_or_else(|| Path::new("."));
    let temp = folder.join(TEMP_FILE_NAME);
    write_and_rename(&temp, destination, bytes).map_err(|(stage, error)| {
        let _ = fs::remove_file(&temp);
        let os_code = error.raw_os_error();
        log::error!("No se ha podido guardar el PDF ({stage:?}, código del sistema: {os_code:?})");
        AppError::pdf_save(classify(stage, os_code))
    })
}
