//! Escritura segura del PDF y clasificación de los fallos (FR-023 y FR-024).

use std::path::Path;

use app_lib::error::{AppError, PdfSaveCode};
use app_lib::storage::pdf_store::{classify, write_atomic, SaveStage, TEMP_FILE_NAME};

fn save_code(result: Result<(), AppError>) -> PdfSaveCode {
    match result {
        Err(AppError::PdfSave { code, .. }) => code,
        other => panic!("resultado inesperado: {other:?}"),
    }
}

fn files_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn windows_errors_map_to_the_contract_codes() {
    let cases = [
        (SaveStage::CreateTemp, 5, PdfSaveCode::PdfAccessDenied),
        (SaveStage::Rename, 5, PdfSaveCode::PdfFileInUse),
        (SaveStage::Rename, 32, PdfSaveCode::PdfFileInUse),
        (SaveStage::Rename, 33, PdfSaveCode::PdfFileInUse),
        (SaveStage::WriteTemp, 39, PdfSaveCode::PdfDiskFull),
        (SaveStage::WriteTemp, 112, PdfSaveCode::PdfDiskFull),
        (SaveStage::CreateTemp, 112, PdfSaveCode::PdfDiskFull),
        (
            SaveStage::CreateTemp,
            3,
            PdfSaveCode::PdfLocationUnavailable,
        ),
        (
            SaveStage::CreateTemp,
            15,
            PdfSaveCode::PdfLocationUnavailable,
        ),
        (
            SaveStage::CreateTemp,
            21,
            PdfSaveCode::PdfLocationUnavailable,
        ),
        (
            SaveStage::WriteTemp,
            53,
            PdfSaveCode::PdfLocationUnavailable,
        ),
        (SaveStage::Rename, 67, PdfSaveCode::PdfLocationUnavailable),
        (
            SaveStage::WriteTemp,
            1167,
            PdfSaveCode::PdfLocationUnavailable,
        ),
        (SaveStage::CreateTemp, 32, PdfSaveCode::PdfWriteFailed),
        (SaveStage::WriteTemp, 5, PdfSaveCode::PdfWriteFailed),
        (SaveStage::WriteTemp, 87, PdfSaveCode::PdfWriteFailed),
    ];
    for (stage, os_code, expected) in cases {
        assert_eq!(
            classify(stage, Some(os_code)),
            expected,
            "{stage:?} {os_code}"
        );
    }
}

#[test]
fn an_error_without_a_system_code_is_a_generic_failure() {
    assert_eq!(
        classify(SaveStage::WriteTemp, None),
        PdfSaveCode::PdfWriteFailed
    );
}

#[test]
fn the_file_is_written_without_leaving_a_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Presupuesto.pdf");
    write_atomic(&path, b"contenido").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"contenido");
    assert_eq!(files_in(dir.path()), ["Presupuesto.pdf"]);
}

#[test]
fn an_existing_file_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Presupuesto.pdf");
    std::fs::write(&path, b"anterior").unwrap();
    write_atomic(&path, b"nuevo").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"nuevo");
}

#[test]
fn a_temporary_file_left_by_a_crash_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(TEMP_FILE_NAME), b"resto").unwrap();
    let path = dir.path().join("Presupuesto.pdf");
    write_atomic(&path, b"nuevo").unwrap();
    assert_eq!(files_in(dir.path()), ["Presupuesto.pdf"]);
}

#[test]
fn a_missing_folder_is_reported_as_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-existe").join("Presupuesto.pdf");
    assert_eq!(
        save_code(write_atomic(&path, b"contenido")),
        PdfSaveCode::PdfLocationUnavailable
    );
}

#[cfg(windows)]
#[test]
fn a_locked_file_is_reported_as_in_use_and_stays_intact() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Presupuesto.pdf");
    std::fs::write(&path, b"anterior").unwrap();

    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    assert_eq!(
        save_code(write_atomic(&path, b"nuevo")),
        PdfSaveCode::PdfFileInUse
    );
    drop(lock);

    assert_eq!(std::fs::read(&path).unwrap(), b"anterior");
    assert_eq!(files_in(dir.path()), ["Presupuesto.pdf"], "sin temporales");
}
