mod common;

use app_lib::services::{logo_service, profile_service};
use common::*;

#[test]
fn replacing_logo_keeps_only_the_new_file() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let a = write_png_rgba(src.path(), "a.png", 300, 300);
    let b = write_jpg(src.path(), "b.jpg", 800, 400);

    logo_service::set_logo(&state, &a).unwrap();
    logo_service::set_logo(&state, &b).unwrap();

    let files = branding_files(data.path());
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(files[0].ends_with(".jpg"));
    let logo = profile_service::get_profile(&state).unwrap().unwrap().logo.unwrap();
    assert_eq!(logo.mime, "image/jpeg");
    assert_eq!((logo.width_px, logo.height_px), (800, 400));
}

#[test]
fn setting_the_same_logo_twice_keeps_the_file() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let a = write_png_rgba(src.path(), "a.png", 300, 300);
    logo_service::set_logo(&state, &a).unwrap();
    logo_service::set_logo(&state, &a).unwrap();
    assert_eq!(branding_files(data.path()).len(), 1);
    assert!(profile_service::get_profile(&state).unwrap().unwrap().logo.is_some());
}

#[test]
fn remove_logo_clears_columns_and_deletes_file() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    profile_service::save_profile(&state, complete_input()).unwrap();
    let a = write_png_rgba(src.path(), "a.png", 300, 300);
    logo_service::set_logo(&state, &a).unwrap();

    logo_service::remove_logo(&state).unwrap();

    assert!(branding_files(data.path()).is_empty());
    let state = reopen(state);
    let profile = profile_service::get_profile(&state).unwrap().unwrap();
    assert!(profile.logo.is_none());
    assert_eq!(profile.trade_name, "Fontanería López");
    let conn = state.conn().unwrap();
    let nulls: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM business_profile WHERE logo_file IS NULL AND logo_mime IS NULL
             AND logo_width_px IS NULL AND logo_height_px IS NULL AND logo_bytes IS NULL
             AND logo_sha256 IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(nulls, 1);
}

#[test]
fn invalid_replacement_keeps_previous_logo() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let a = write_png_rgba(src.path(), "a.png", 300, 300);
    logo_service::set_logo(&state, &a).unwrap();
    let before = branding_files(data.path());
    let bad = src.path().join("falso.jpg");
    std::fs::write(&bad, b"%PDF-1.4").unwrap();

    assert!(logo_service::set_logo(&state, &bad).is_err());
    assert_eq!(branding_files(data.path()), before);
    assert_eq!(
        profile_service::get_profile(&state).unwrap().unwrap().logo.unwrap().mime,
        "image/png"
    );
}

#[test]
fn cleanup_orphans_removes_unreferenced_files_only() {
    let src = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let a = write_png_rgba(src.path(), "a.png", 300, 300);
    logo_service::set_logo(&state, &a).unwrap();
    let current = branding_files(data.path());

    let branding = data.path().join("branding");
    std::fs::write(branding.join("logo-deadbeef.png"), b"x").unwrap();
    std::fs::write(branding.join(".tmp-123-456"), b"x").unwrap();
    std::fs::write(branding.join("otro.txt"), b"x").unwrap();

    let removed = {
        let conn = state.conn().unwrap();
        logo_service::cleanup_orphans(data.path(), &conn).unwrap()
    };
    assert_eq!(removed, 2);
    let mut expected = current.clone();
    expected.push("otro.txt".into());
    expected.sort();
    assert_eq!(branding_files(data.path()), expected);
}

#[test]
fn cleanup_orphans_without_branding_dir_is_noop() {
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    let conn = state.conn().unwrap();
    assert_eq!(logo_service::cleanup_orphans(data.path(), &conn).unwrap(), 0);
}

#[test]
fn edited_profile_persists_after_reopen() {
    let data = tempfile::tempdir().unwrap();
    let state = state_in(data.path());
    profile_service::save_profile(&state, complete_input()).unwrap();
    let mut input = complete_input();
    input.address.city = "Getafe".into();
    input.phone = "911 222 333".into();
    profile_service::save_profile(&state, input).unwrap();

    let state = reopen(state);
    let p = profile_service::get_profile(&state).unwrap().unwrap();
    assert_eq!(p.address.city, "Getafe");
    assert_eq!(p.phone, "911 222 333");
}
