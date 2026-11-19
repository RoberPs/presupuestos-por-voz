//! Validación de los datos del cliente, el número y la validez (data-model.md).

use app_lib::domain::quote_doc::details::{validate, QuoteDetailsInput};
use app_lib::error::ErrorCode;

fn input() -> QuoteDetailsInput {
    QuoteDetailsInput {
        client_name: "Ana Ruiz".into(),
        client_address: None,
        client_phone: None,
        client_email: None,
        client_tax_id: None,
        quote_number: "2026-001".into(),
        validity_days: 30,
    }
}

fn error_on(input: &QuoteDetailsInput, field: &str) -> ErrorCode {
    let errors = validate(input).unwrap_err();
    errors
        .iter()
        .find(|e| e.field == field)
        .unwrap_or_else(|| panic!("sin error en {field}: {errors:?}"))
        .code
}

#[test]
fn spaces_are_cleaned() {
    let clean = validate(&QuoteDetailsInput {
        client_name: "  Ana   Ruiz ".into(),
        client_address: Some("  Calle  Mayor 1 ".into()),
        quote_number: " 2026-001 ".into(),
        ..input()
    })
    .unwrap();
    assert_eq!(clean.client_name, "Ana Ruiz");
    assert_eq!(clean.client_address.as_deref(), Some("Calle Mayor 1"));
    assert_eq!(clean.quote_number, "2026-001");
}

#[test]
fn empty_optional_data_becomes_none() {
    let clean = validate(&QuoteDetailsInput {
        client_address: Some("   ".into()),
        client_phone: Some(String::new()),
        client_email: Some(" ".into()),
        client_tax_id: Some("".into()),
        ..input()
    })
    .unwrap();
    assert_eq!(clean.client_address, None);
    assert_eq!(clean.client_phone, None);
    assert_eq!(clean.client_email, None);
    assert_eq!(clean.client_tax_id, None);
}

#[test]
fn an_empty_name_or_number_is_saved_and_blocks_later() {
    let clean = validate(&QuoteDetailsInput {
        client_name: "  ".into(),
        quote_number: " ".into(),
        ..input()
    })
    .unwrap();
    assert_eq!(clean.client_name, "");
    assert_eq!(clean.quote_number, "");
}

#[test]
fn the_format_of_the_data_is_not_checked() {
    let clean = validate(&QuoteDetailsInput {
        client_phone: Some("+33 (0)1 23".into()),
        client_email: Some("sin arroba".into()),
        client_tax_id: Some("FR-123".into()),
        ..input()
    })
    .unwrap();
    assert_eq!(clean.client_email.as_deref(), Some("sin arroba"));
}

#[test]
fn the_maximum_lengths_are_accepted() {
    validate(&QuoteDetailsInput {
        client_name: "n".repeat(120),
        client_address: Some("d".repeat(200)),
        client_phone: Some("1".repeat(30)),
        client_email: Some("e".repeat(254)),
        client_tax_id: Some("x".repeat(20)),
        quote_number: "9".repeat(30),
        validity_days: 30,
    })
    .unwrap();
}

#[test]
fn lengths_are_counted_in_characters() {
    validate(&QuoteDetailsInput {
        client_name: "ñ".repeat(120),
        ..input()
    })
    .unwrap();
}

#[test]
fn longer_texts_are_rejected() {
    let cases = [
        (
            "clientName",
            QuoteDetailsInput {
                client_name: "n".repeat(121),
                ..input()
            },
        ),
        (
            "clientAddress",
            QuoteDetailsInput {
                client_address: Some("d".repeat(201)),
                ..input()
            },
        ),
        (
            "clientPhone",
            QuoteDetailsInput {
                client_phone: Some("1".repeat(31)),
                ..input()
            },
        ),
        (
            "clientEmail",
            QuoteDetailsInput {
                client_email: Some("e".repeat(255)),
                ..input()
            },
        ),
        (
            "clientTaxId",
            QuoteDetailsInput {
                client_tax_id: Some("x".repeat(21)),
                ..input()
            },
        ),
        (
            "quoteNumber",
            QuoteDetailsInput {
                quote_number: "9".repeat(31),
                ..input()
            },
        ),
    ];
    for (field, case) in cases {
        assert_eq!(error_on(&case, field), ErrorCode::TooLong, "{field}");
    }
}

#[test]
fn the_validity_must_be_between_1_and_365_days() {
    for days in [0, 366, -5] {
        let case = QuoteDetailsInput {
            validity_days: days,
            ..input()
        };
        assert_eq!(
            error_on(&case, "validityDays"),
            ErrorCode::ValidityInvalid,
            "{days} días"
        );
    }
    for days in [1, 365] {
        validate(&QuoteDetailsInput {
            validity_days: days,
            ..input()
        })
        .unwrap();
    }
}

#[test]
fn every_error_is_reported_at_once() {
    let errors = validate(&QuoteDetailsInput {
        client_name: "n".repeat(121),
        validity_days: 0,
        ..input()
    })
    .unwrap_err();
    assert_eq!(errors.len(), 2);
}
