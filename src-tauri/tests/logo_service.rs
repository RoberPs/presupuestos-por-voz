mod common;

use app_lib::error::{AppError, LogoRejectCode};
use app_lib::services::{logo_service, profile_service};
use common::*;
use sha2::{Digest, Sha256};

fn rejected_code(r: Result<impl std::fmt::Debug, AppError>) -> LogoRejectCode {
    match r {
        Err(AppError::LogoRejected { code, .. }) => code,
        other => panic!("se esperaba LogoRejected, llegó {other:?}"),
    }
}

#[test]
fn rejects_files_over_5_mb() {
    let src = tempfile::tempdir().unwrap();
    let path = src.path().join("grande.png");
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    bytes.resize(6 * 1024 * 1024, 0);
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(rejected_code(logo_service::validate_logo_file(&path)), LogoRejectCode::LogoTooLarge);
}

#[test]
fn rejects_renamed_text_or_pdf() {
    let src = tempfile::tempdir().unwrap();
    let txt = src.path().join("texto.png");
    std::fs::write(&txt, "hola, no soy una imagen").unwrap();
    assert_eq!(
        rejected_code(logo_service::validate_logo_file(&txt)),
        LogoRejectCode::LogoUnsupportedFormat
    );
    let pdf = src.path().join("doc.png");
    std::fs::write(&pdf, b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n1 0 obj\n").unwrap();
    assert_eq!(
        rejected_code(logo_service::validate_logo_file(&pdf)),
        LogoRejectCode::LogoUnsupportedFormat
    );
}

#[test]
fn rejects_truncated_png() {
    let src = tempfile::tempdir().unwrap();
    let full = write_png_rgba(src.path(), "ok.png", 400, 400);
    let bytes = std::fs::read(&full).unwrap();
    let truncated = src.path().join("roto.png");
    std::fs::write(&truncated, &bytes[..bytes.len() / 2]).unwrap();
    assert_eq!(
        rejected_code(logo_service::validate_logo_file(&truncated)),
        LogoRejectCode::LogoCorrupt
    );
}

#[test]
fn accepts_small_png_with_low_resolution_warning() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let png = write_png_rgba(src.path(), "peq.png", 150, 150);

    let result = logo_service::set_logo(&state, &png).unwrap();
    assert!(result.logo.low_resolution);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.logo.mime, "image/png");
    assert_eq!((result.logo.width_px, result.logo.height_px), (150, 150));
    assert!(result.logo.data_url.starts_with("data:image/png;base64,"));
}

#[test]
fn accepts_jpg_without_warning_and_keeps_bytes_intact() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let jpg = write_jpg(src.path(), "logo.jpg", 800, 400);

    let result = logo_service::set_logo(&state, &jpg).unwrap();
    assert!(!result.logo.low_resolution);
    assert!(result.warnings.is_empty());
    assert_eq!(result.logo.mime, "image/jpeg");

    let files = branding_files(data.path());
    assert_eq!(files.len(), 1);
    assert!(files[0].starts_with("logo-") && files[0].ends_with(".jpg"));
    let stored = std::fs::read(data.path().join("branding").join(&files[0])).unwrap();
    let original = std::fs::read(&jpg).unwrap();
    assert_eq!(Sha256::digest(&stored), Sha256::digest(&original));
}

#[test]
fn rejection_keeps_previous_logo() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let good = write_png_rgba(src.path(), "bueno.png", 300, 300);
    logo_service::set_logo(&state, &good).unwrap();
    let before_files = branding_files(data.path());
    let before = profile_service::get_profile(&state).unwrap().unwrap().logo.unwrap();

    let bad = src.path().join("malo.png");
    std::fs::write(&bad, "no").unwrap();
    assert!(logo_service::set_logo(&state, &bad).is_err());

    assert_eq!(branding_files(data.path()), before_files);
    let after = profile_service::get_profile(&state).unwrap().unwrap().logo.unwrap();
    assert_eq!(after, before);
}

#[test]
fn set_logo_without_profile_creates_draft() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let png = write_png_rgba(src.path(), "logo.png", 300, 200);
    logo_service::set_logo(&state, &png).unwrap();

    let state = reopen(state);
    let profile = profile_service::get_profile(&state).unwrap().unwrap();
    assert!(profile.logo.is_some());
    assert_eq!(profile.trade_name, "");
}

#[test]
fn copy_survives_original_being_deleted() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let png = write_png_rgba(src.path(), "logo.png", 300, 300);
    logo_service::set_logo(&state, &png).unwrap();
    std::fs::remove_file(&png).unwrap();

    let state = reopen(state);
    assert!(profile_service::get_profile(&state).unwrap().unwrap().logo.is_some());
}
