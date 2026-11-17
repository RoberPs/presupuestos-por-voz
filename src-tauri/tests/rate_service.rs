mod common;

use app_lib::domain::rate::RateInput;
use app_lib::error::{AppError, ErrorCode};
use app_lib::services::rate_service;
use common::*;

fn input(concept: &str, unit: &str, price: &str) -> RateInput {
    RateInput { concept: concept.into(), unit_label: unit.into(), price: price.into() }
}

fn validation_codes(r: Result<impl std::fmt::Debug, AppError>) -> Vec<(String, ErrorCode)> {
    match r {
        Err(AppError::Validation { errors }) => errors.into_iter().map(|e| (e.field, e.code)).collect(),
        other => panic!("se esperaba Validation, llegó {other:?}"),
    }
}

#[test]
fn creates_a_valid_rate() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let rate = rate_service::create(&state, input("  Instalación   de grifo ", "unidad", "45,50")).unwrap();
    assert_eq!(rate.concept, "Instalación de grifo");
    assert_eq!(rate.unit.label, "unidad");
    assert!(rate.unit.is_predefined);
    assert_eq!(rate.price_cents, 4550);
    assert_eq!(rate.price_display, "45,50 €");
}

#[test]
fn rejects_each_invalid_price() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let cases = [
        ("", ErrorCode::PriceRequired),
        ("abc", ErrorCode::PriceInvalidFormat),
        ("12,345", ErrorCode::PriceTooManyDecimals),
        ("0", ErrorCode::PriceNotPositive),
        ("-3", ErrorCode::PriceNotPositive),
    ];
    for (price, code) in cases {
        let got = validation_codes(rate_service::create(&state, input("Grifo", "unidad", price)));
        assert_eq!(got, vec![("price".to_string(), code)], "precio {price:?}");
    }
    assert!(rate_service::list(&state, None).unwrap().is_empty());
}

#[test]
fn requires_concept_and_unit() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let got = validation_codes(rate_service::create(&state, input("  ", " ", "10")));
    assert_eq!(
        got,
        vec![
            ("concept".to_string(), ErrorCode::ConceptRequired),
            ("unitLabel".to_string(), ErrorCode::UnitRequired)
        ]
    );
    let got = validation_codes(rate_service::create(&state, input(&"x".repeat(161), &"u".repeat(31), "10")));
    assert_eq!(
        got,
        vec![("concept".to_string(), ErrorCode::TooLong), ("unitLabel".to_string(), ErrorCode::TooLong)]
    );
}

#[test]
fn detects_duplicates_ignoring_case_accents_and_spaces() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let first = rate_service::create(&state, input("Instalación de grifo", "unidad", "45")).unwrap();
    match rate_service::create(&state, input("instalacion  de GRIFO", "Unidad", "50")) {
        Err(AppError::DuplicateRate { existing_rate_id, message }) => {
            assert_eq!(existing_rate_id, first.id);
            assert!(message.contains("Instalación de grifo"));
        }
        other => panic!("se esperaba DuplicateRate, llegó {other:?}"),
    }
}

#[test]
fn same_concept_with_different_unit_is_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    rate_service::create(&state, input("Pintura", "hora", "30")).unwrap();
    rate_service::create(&state, input("Pintura", "m²", "8,5")).unwrap();
    assert_eq!(rate_service::list(&state, None).unwrap().len(), 2);
}

#[test]
fn custom_unit_is_created_and_listed_last_alphabetically() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    rate_service::create(&state, input("Toma de corriente", "punto de luz", "25")).unwrap();
    rate_service::create(&state, input("Rodapié", "Caja", "3")).unwrap();
    // Reutiliza la unidad existente aunque cambie mayúsculas o tildes
    let again = rate_service::create(&state, input("Interruptor", "PUNTO DE LUZ", "20")).unwrap();
    assert_eq!(again.unit.label, "punto de luz");

    let labels: Vec<String> =
        rate_service::units(&state).unwrap().into_iter().map(|u| u.label).collect();
    assert_eq!(
        labels,
        vec!["unidad", "hora", "metro", "m²", "metro lineal", "Caja", "punto de luz"]
    );
}

#[test]
fn update_applies_changes_and_checks_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let a = rate_service::create(&state, input("Pintura", "hora", "25")).unwrap();
    let b = rate_service::create(&state, input("Alicatado", "m²", "20")).unwrap();

    let updated = rate_service::update(&state, a.id, input("Pintura", "hora", "30")).unwrap();
    assert_eq!(updated.price_display, "30,00 €");

    // Sin cambios sobre sí misma: no es duplicado
    assert!(rate_service::update(&state, a.id, input("pintura", "hora", "30")).is_ok());

    match rate_service::update(&state, b.id, input("PINTURA", "hora", "1")) {
        Err(AppError::DuplicateRate { existing_rate_id, .. }) => assert_eq!(existing_rate_id, a.id),
        other => panic!("se esperaba DuplicateRate, llegó {other:?}"),
    }

    let invalid = validation_codes(rate_service::update(&state, a.id, input("Pintura", "hora", "0")));
    assert_eq!(invalid, vec![("price".to_string(), ErrorCode::PriceNotPositive)]);
}

#[test]
fn delete_and_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let a = rate_service::create(&state, input("Pintura", "m²", "8")).unwrap();
    rate_service::delete(&state, a.id).unwrap();
    assert!(rate_service::list(&state, None).unwrap().is_empty());

    assert!(matches!(rate_service::delete(&state, a.id), Err(AppError::NotFound { .. })));
    assert!(matches!(
        rate_service::update(&state, 999, input("X", "hora", "1")),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn list_is_sorted_by_normalized_concept() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    for c in ["Brida", "abrazadera", "Ábaco", "zócalo"] {
        rate_service::create(&state, input(c, "unidad", "1")).unwrap();
    }
    let concepts: Vec<String> =
        rate_service::list(&state, None).unwrap().into_iter().map(|r| r.concept).collect();
    assert_eq!(concepts, vec!["Ábaco", "abrazadera", "Brida", "zócalo"]);
}

#[test]
fn rates_and_units_persist_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    rate_service::create(&state, input("Toma", "punto de luz", "25")).unwrap();
    let state = reopen(state);
    let rates = rate_service::list(&state, None).unwrap();
    assert_eq!(rates.len(), 1);
    assert_eq!(rates[0].unit.label, "punto de luz");
    assert!(rate_service::units(&state).unwrap().iter().any(|u| u.label == "punto de luz"));
}
