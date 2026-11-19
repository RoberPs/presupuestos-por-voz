mod common;

use app_lib::domain::profile::ProfileStatus;
use app_lib::error::{AppError, ErrorCode};
use app_lib::services::profile_service;
use common::*;

#[test]
fn new_database_has_no_profile() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    assert!(profile_service::get_profile(&state).unwrap().is_none());
}

#[test]
fn empty_draft_is_saved_as_incomplete_with_exact_missing_fields() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let view = profile_service::save_profile(&state, empty_input()).unwrap();
    assert_eq!(view.status, ProfileStatus::Incomplete);
    let fields: Vec<&str> = view
        .missing_fields
        .iter()
        .map(|f| f.field.as_str())
        .collect();
    assert_eq!(
        fields,
        vec![
            "tradeName",
            "taxId",
            "phone",
            "address.street",
            "address.postalCode",
            "address.city",
            "address.province"
        ]
    );
    assert!(view
        .missing_fields
        .iter()
        .all(|f| f.code == ErrorCode::Required));
}

#[test]
fn complete_and_valid_profile_is_complete() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let view = profile_service::save_profile(&state, complete_input()).unwrap();
    assert_eq!(view.status, ProfileStatus::Complete);
    assert!(view.missing_fields.is_empty());
    assert_eq!(view.currency, "EUR");
}

#[test]
fn invalid_tax_id_is_saved_but_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let mut input = complete_input();
    input.tax_id = "12345678a".into();
    let view = profile_service::save_profile(&state, input).unwrap();
    assert_eq!(view.status, ProfileStatus::Incomplete);
    assert_eq!(view.tax_id, "12345678A");
    assert_eq!(view.missing_fields.len(), 1);
    assert_eq!(view.missing_fields[0].field, "taxId");
    assert_eq!(view.missing_fields[0].code, ErrorCode::TaxIdInvalid);
}

#[test]
fn invalid_optional_email_makes_profile_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let mut input = complete_input();
    input.email = Some("a@b".into());
    let view = profile_service::save_profile(&state, input).unwrap();
    assert_eq!(view.status, ProfileStatus::Incomplete);
    assert_eq!(view.missing_fields[0].code, ErrorCode::EmailInvalid);
}

#[test]
fn hard_errors_block_saving() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());

    let mut input = complete_input();
    input.vat_rate_percent = "150".into();
    match profile_service::save_profile(&state, input) {
        Err(AppError::Validation { errors }) => {
            assert_eq!(errors[0].field, "vatRatePercent");
            assert_eq!(errors[0].code, ErrorCode::VatOutOfRange);
        }
        other => panic!("se esperaba Validation, llegó {other:?}"),
    }

    let mut input = complete_input();
    input.trade_name = "x".repeat(121);
    match profile_service::save_profile(&state, input) {
        Err(AppError::Validation { errors }) => {
            assert_eq!(errors[0].field, "tradeName");
            assert_eq!(errors[0].code, ErrorCode::TooLong);
        }
        other => panic!("se esperaba Validation, llegó {other:?}"),
    }

    // Nada se ha guardado
    assert!(profile_service::get_profile(&state).unwrap().is_none());
}

#[test]
fn data_survives_reopen_and_vat_defaults_to_21() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let saved = profile_service::save_profile(&state, complete_input()).unwrap();
    assert_eq!(saved.vat_rate_percent, "21,00");

    let state = reopen(state);
    let loaded = profile_service::get_profile(&state).unwrap().unwrap();
    assert_eq!(loaded, saved);
}

#[test]
fn edits_overwrite_previous_values() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    profile_service::save_profile(&state, complete_input()).unwrap();
    let mut input = complete_input();
    input.phone = "+34 912345678".into();
    input.vat_rate_percent = "10,5".into();
    input.legal_name = Some("   ".into());
    profile_service::save_profile(&state, input).unwrap();

    let state = reopen(state);
    let loaded = profile_service::get_profile(&state).unwrap().unwrap();
    assert_eq!(loaded.phone, "+34 912345678");
    assert_eq!(loaded.vat_rate_percent, "10,50");
    assert_eq!(loaded.legal_name, None);
}

#[test]
fn validate_does_not_save() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let mut input = complete_input();
    input.address.postal_code = "99999".into();
    let errors = profile_service::validate_profile(&input);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "address.postalCode");
    assert_eq!(errors[0].code, ErrorCode::PostalCodeInvalid);
    assert!(profile_service::get_profile(&state).unwrap().is_none());
}
