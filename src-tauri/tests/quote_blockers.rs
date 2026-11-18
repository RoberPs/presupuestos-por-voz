//! Condiciones para generar el PDF y avisos (FR-001 a FR-005b y FR-018b).

use app_lib::domain::draft::{DraftLine, DraftStatus, PriceSource};
use app_lib::domain::quote_doc::blockers::{
    evaluate, warnings, BlockerInput, CheckedText, QuoteWarning, WarningInput,
};
use app_lib::error::{ErrorCode, FieldError, PendingLineRef, QuoteBlocker};
use app_lib::pdf::fonts::PdfFonts;

fn line(id: i64, concept: &str) -> DraftLine {
    DraftLine {
        id,
        position: id,
        concept: concept.into(),
        quantity_centi: Some(300),
        quantity_issue: None,
        unit_label: Some("metro".into()),
        price_cents: Some(2500),
        price_source: PriceSource::Manual,
        rate_id: None,
        rate_concept: None,
        pending_reason: None,
        source_fragment: None,
    }
}

fn supported(character: char) -> bool {
    character.is_ascii() || "ñáéíóúü€¿¡".contains(character)
}

fn input<'a>(lines: &'a [DraftLine]) -> BlockerInput<'a> {
    BlockerInput {
        lines,
        status: DraftStatus::Reviewed,
        missing_profile_fields: &[],
        client_name: "Ana Ruiz",
        quote_number: "2026-001",
        texts: &[],
        logo_unreadable: false,
    }
}

#[test]
fn a_complete_quote_has_no_blockers() {
    let lines = [line(1, "Bajante")];
    assert!(evaluate(&input(&lines), &supported).is_empty());
}

#[test]
fn a_quote_without_lines_is_blocked() {
    assert_eq!(evaluate(&input(&[]), &supported), [QuoteBlocker::NoLines]);
}

#[test]
fn lines_without_price_or_quantity_are_listed_by_concept() {
    let mut no_price = line(1, "Grifo");
    no_price.price_cents = None;
    no_price.price_source = PriceSource::Pending;
    let mut no_quantity = line(2, "Bajante");
    no_quantity.quantity_centi = None;
    let lines = [no_price, line(3, "Mano de obra"), no_quantity];

    assert_eq!(
        evaluate(&input(&lines), &supported),
        [QuoteBlocker::PendingLines {
            lines: vec![
                PendingLineRef {
                    id: 1,
                    concept: "Grifo".into()
                },
                PendingLineRef {
                    id: 2,
                    concept: "Bajante".into()
                },
            ]
        }]
    );
}

#[test]
fn a_draft_that_is_not_reviewed_is_blocked() {
    let lines = [line(1, "Bajante")];
    let case = BlockerInput {
        status: DraftStatus::Draft,
        ..input(&lines)
    };
    assert_eq!(evaluate(&case, &supported), [QuoteBlocker::NotReviewed]);
}

#[test]
fn an_incomplete_profile_is_blocked_with_its_missing_fields() {
    let lines = [line(1, "Bajante")];
    let missing = [FieldError::new("taxId", ErrorCode::Required)];
    let case = BlockerInput {
        missing_profile_fields: &missing,
        ..input(&lines)
    };
    assert_eq!(
        evaluate(&case, &supported),
        [QuoteBlocker::ProfileIncomplete {
            missing_fields: missing.to_vec()
        }]
    );
}

#[test]
fn an_empty_client_name_is_blocked() {
    let lines = [line(1, "Bajante")];
    let case = BlockerInput {
        client_name: "  ",
        ..input(&lines)
    };
    assert_eq!(
        evaluate(&case, &supported),
        [QuoteBlocker::ClientNameRequired]
    );
}

#[test]
fn an_empty_number_is_blocked() {
    let lines = [line(1, "Bajante")];
    let case = BlockerInput {
        quote_number: "",
        ..input(&lines)
    };
    assert_eq!(
        evaluate(&case, &supported),
        [QuoteBlocker::QuoteNumberRequired]
    );
}

#[test]
fn an_unreadable_logo_is_blocked() {
    let lines = [line(1, "Bajante")];
    let case = BlockerInput {
        logo_unreadable: true,
        ..input(&lines)
    };
    assert_eq!(evaluate(&case, &supported), [QuoteBlocker::LogoUnreadable]);
}

fn text<'a>(field: &'a str, label: &'a str, value: &'a str) -> CheckedText<'a> {
    CheckedText {
        field,
        label,
        text: value,
    }
}

#[test]
fn unsupported_characters_report_the_field_and_the_character() {
    let lines = [line(1, "Bajante")];
    let texts = [
        text("lines.1.concept", "Concepto de una línea", "Bajante 😀"),
        text("clientName", "Nombre del cliente", "Ana 🎉 😀 🎉"),
        text("tradeName", "Nombre comercial", "Fontanería Ñandú"),
        text("business.email", "Email del negocio", "pepe🔧@correo.es"),
    ];
    let case = BlockerInput {
        texts: &texts,
        ..input(&lines)
    };

    let blockers = evaluate(&case, &|c| supported(c) || c == 'Ñ');
    let [QuoteBlocker::UnsupportedCharacters { fields }] = blockers.as_slice() else {
        panic!("resultado inesperado: {blockers:?}");
    };
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].field, "lines.1.concept");
    assert_eq!(fields[0].label, "Concepto de una línea");
    assert_eq!(fields[0].characters, ["😀"]);
    assert_eq!(fields[1].characters, ["🎉", "😀"], "sin repetir caracteres");
    assert_eq!(fields[2].field, "business.email");
    assert_eq!(fields[2].characters, ["🔧"]);
}

#[test]
fn spaces_are_never_reported() {
    let lines = [line(1, "Bajante")];
    let texts = [text(
        "clientName",
        "Nombre del cliente",
        "Ana\u{a0}Ruiz\tLópez",
    )];
    let case = BlockerInput {
        texts: &texts,
        ..input(&lines)
    };
    assert!(evaluate(&case, &supported).is_empty());
}

#[test]
fn every_blocker_is_reported_at_once() {
    let mut pending = line(1, "Grifo");
    pending.price_cents = None;
    let lines = [pending];
    let missing = [FieldError::new("phone", ErrorCode::Required)];
    let texts = [text("clientName", "Nombre del cliente", "😀")];
    let case = BlockerInput {
        lines: &lines,
        status: DraftStatus::Draft,
        missing_profile_fields: &missing,
        client_name: "",
        quote_number: "",
        texts: &texts,
        logo_unreadable: true,
    };

    let kinds: Vec<&str> = evaluate(&case, &supported)
        .iter()
        .map(|blocker| match blocker {
            QuoteBlocker::NoLines => "noLines",
            QuoteBlocker::PendingLines { .. } => "pendingLines",
            QuoteBlocker::NotReviewed => "notReviewed",
            QuoteBlocker::ProfileIncomplete { .. } => "profileIncomplete",
            QuoteBlocker::ClientNameRequired => "clientNameRequired",
            QuoteBlocker::QuoteNumberRequired => "quoteNumberRequired",
            QuoteBlocker::UnsupportedCharacters { .. } => "unsupportedCharacters",
            QuoteBlocker::LogoUnreadable => "logoUnreadable",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "pendingLines",
            "notReviewed",
            "profileIncomplete",
            "clientNameRequired",
            "quoteNumberRequired",
            "unsupportedCharacters",
            "logoUnreadable",
        ]
    );
}

fn warning_input() -> WarningInput {
    WarningInput {
        has_logo: true,
        accepted_unreadable_logo: false,
        applied_vat_bp: 2100,
        current_vat_bp: 2100,
        unparsed_fragments: 0,
    }
}

#[test]
fn a_normal_quote_has_no_warnings() {
    assert!(warnings(&warning_input()).is_empty());
}

#[test]
fn a_missing_logo_is_warned() {
    let case = WarningInput {
        has_logo: false,
        ..warning_input()
    };
    assert_eq!(warnings(&case), [QuoteWarning::NoLogo]);
}

#[test]
fn an_accepted_unreadable_logo_is_warned_only_once() {
    let case = WarningInput {
        has_logo: false,
        accepted_unreadable_logo: true,
        ..warning_input()
    };
    assert_eq!(warnings(&case), [QuoteWarning::LogoUnreadable]);
}

#[test]
fn a_vat_change_is_warned_with_both_rates() {
    let case = WarningInput {
        current_vat_bp: 1000,
        ..warning_input()
    };
    assert_eq!(
        warnings(&case),
        [QuoteWarning::VatChanged {
            applied_rate_percent: "21".into(),
            current_rate_percent: "10".into(),
        }]
    );
}

#[test]
fn unparsed_fragments_are_warned_with_their_count() {
    let case = WarningInput {
        unparsed_fragments: 3,
        ..warning_input()
    };
    assert_eq!(
        warnings(&case),
        [QuoteWarning::UnparsedFragments { count: 3 }]
    );
}

#[test]
fn the_embedded_font_covers_spanish_but_not_emoji() {
    let fonts = PdfFonts::load().unwrap();
    for character in "ñÑáéíóúÁÉÍÓÚüÜ¿¡€ºª·".chars() {
        assert!(fonts.supports(character), "falta «{character}»");
    }
    assert!(!fonts.supports('😀'));
    assert!(!fonts.supports('🔧'));
}
