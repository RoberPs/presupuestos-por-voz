//! Edición de líneas (FR-024 a FR-027 y FR-032; "Reglas de edición" de data-model.md).

mod common;

use std::time::Instant;

use app_lib::domain::draft::{DraftStatus, PendingReason, PriceSource};
use app_lib::error::{AppError, ErrorCode};
use app_lib::services::draft_service::{self as draft, DraftLineInput, DraftLinePatch, DraftView};
use app_lib::state::AppState;
use common::{create_rate, draft_view, example_rates, seed_draft, state_in, EXAMPLE_TEXT};

struct Fixture {
    _dir: tempfile::TempDir,
    state: AppState,
    bajante_rate: i64,
    mano_rate: i64,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let (bajante, mano) = example_rates(&state);
    seed_draft(&state, EXAMPLE_TEXT);
    Fixture {
        _dir: dir,
        state,
        bajante_rate: bajante.id,
        mano_rate: mano.id,
    }
}

impl Fixture {
    fn line_id(&self, index: usize) -> i64 {
        draft_view(&self.state).lines[index].id
    }

    fn update(&self, index: usize, patch: DraftLinePatch) -> Result<DraftView, AppError> {
        draft::line_update(&self.state, self.line_id(index), patch)
    }

    /// Borrador con una línea sin precio ni unidad.
    fn with_pending_line(self) -> Self {
        seed_draft(&self.state, "pintar techo");
        self
    }
}

fn patch() -> DraftLinePatch {
    DraftLinePatch::default()
}

fn field_codes(result: Result<DraftView, AppError>) -> Vec<(String, ErrorCode)> {
    match result {
        Err(AppError::Validation { errors }) => {
            errors.into_iter().map(|e| (e.field, e.code)).collect()
        }
        other => panic!("se esperaba un error de validación y se obtuvo {other:?}"),
    }
}

#[test]
fn changing_the_quantity_recalculates_the_amounts() {
    let f = fixture();
    let view = f
        .update(
            0,
            DraftLinePatch {
                quantity: Some("4".into()),
                ..patch()
            },
        )
        .unwrap();

    assert_eq!(view.lines[0].quantity.as_deref(), Some("4"));
    assert_eq!(view.lines[0].amount_display.as_deref(), Some("100,00 €"));
    assert_eq!(view.totals.base_display, "160,00 €");
    assert_eq!(view.totals.vat_display, "33,60 €");
    assert_eq!(view.totals.total_display, "193,60 €");
    assert!(view.has_manual_changes);
}

#[test]
fn changing_the_quantity_clears_the_pending_mark() {
    let f = fixture();
    seed_draft(&f.state, "bajante, 3 metros, no, mejor 4");
    let view = f
        .update(
            0,
            DraftLinePatch {
                quantity: Some("4".into()),
                ..patch()
            },
        )
        .unwrap();
    assert_eq!(view.lines[0].quantity_issue, None);
    assert_eq!(view.totals.pending_count, 0);
}

#[test]
fn changing_the_concept_keeps_the_price_and_its_origin() {
    let f = fixture();
    let view = f
        .update(
            0,
            DraftLinePatch {
                concept: Some("  Bajante   de PVC ".into()),
                ..patch()
            },
        )
        .unwrap();
    let line = &view.lines[0];
    assert_eq!(line.concept, "Bajante de PVC");
    assert_eq!(line.price_cents, Some(2500));
    assert_eq!(line.price_source, PriceSource::Rate);
    assert_eq!(line.rate_id, Some(f.bajante_rate));
    assert_eq!(line.rate_concept.as_deref(), Some("Bajante"));
}

#[test]
fn changing_the_price_makes_it_manual() {
    let f = fixture();
    let view = f
        .update(
            0,
            DraftLinePatch {
                price: Some("27,50".into()),
                ..patch()
            },
        )
        .unwrap();
    let line = &view.lines[0];
    assert_eq!(line.price_source, PriceSource::Manual);
    assert_eq!(line.price_cents, Some(2750));
    assert_eq!(line.rate_id, None);
    assert_eq!(line.rate_concept, None);
    assert_eq!(line.amount_display.as_deref(), Some("82,50 €"));
}

#[test]
fn changing_the_unit_of_a_rate_line_leaves_the_price_pending() {
    let f = fixture();
    let view = f
        .update(
            0,
            DraftLinePatch {
                unit_label: Some("hora".into()),
                ..patch()
            },
        )
        .unwrap();
    let line = &view.lines[0];
    assert_eq!(line.unit_label.as_deref(), Some("hora"));
    assert_eq!(line.price_cents, None);
    assert_eq!(line.price_source, PriceSource::Pending);
    assert_eq!(line.pending_reason, Some(PendingReason::UnitMismatch));
    assert_eq!(line.rate_id, None);
    assert_eq!(line.suggestions.len(), 1);
    assert_eq!(line.suggestions[0].id, f.bajante_rate);
    assert_eq!(view.totals.pending_count, 1);
}

#[test]
fn writing_the_same_unit_keeps_the_rate_price() {
    let f = fixture();
    let view = f
        .update(
            0,
            DraftLinePatch {
                unit_label: Some("METROS".into()),
                ..patch()
            },
        )
        .unwrap();
    assert_eq!(view.lines[0].price_source, PriceSource::Rate);
    assert_eq!(view.lines[0].price_cents, Some(2500));
}

#[test]
fn changing_the_unit_of_a_manual_price_keeps_the_price() {
    let f = fixture();
    f.update(
        0,
        DraftLinePatch {
            price: Some("20".into()),
            ..patch()
        },
    )
    .unwrap();
    let view = f
        .update(
            0,
            DraftLinePatch {
                unit_label: Some("hora".into()),
                ..patch()
            },
        )
        .unwrap();
    assert_eq!(view.lines[0].price_source, PriceSource::Manual);
    assert_eq!(view.lines[0].price_cents, Some(2000));
}

#[test]
fn a_price_on_a_line_without_unit_asks_for_the_unit() {
    let f = fixture().with_pending_line();
    assert_eq!(
        field_codes(f.update(
            0,
            DraftLinePatch {
                price: Some("20".into()),
                ..patch()
            }
        )),
        [("unitLabel".to_string(), ErrorCode::UnitRequired)]
    );
}

#[test]
fn a_unit_and_a_price_together_complete_the_line() {
    let f = fixture().with_pending_line();
    let view = f
        .update(
            0,
            DraftLinePatch {
                quantity: Some("2".into()),
                unit_label: Some("hora".into()),
                price: Some("20".into()),
                ..patch()
            },
        )
        .unwrap();
    let line = &view.lines[0];
    assert_eq!(line.price_source, PriceSource::Manual);
    assert_eq!(line.pending_reason, None);
    assert_eq!(line.amount_display.as_deref(), Some("40,00 €"));
    assert_eq!(view.totals.pending_count, 0);
}

#[test]
fn assigning_a_rate_copies_its_data() {
    let f = fixture().with_pending_line();
    let view = draft::line_assign_rate(&f.state, f.line_id(0), f.mano_rate).unwrap();
    let line = &view.lines[0];
    assert_eq!(line.concept, "Mano de obra");
    assert_eq!(line.unit_label.as_deref(), Some("hora"));
    assert_eq!(line.price_cents, Some(3000));
    assert_eq!(line.price_source, PriceSource::Rate);
    assert_eq!(line.rate_id, Some(f.mano_rate));
    assert_eq!(line.rate_concept.as_deref(), Some("Mano de obra"));
    assert_eq!(line.pending_reason, None);
    assert!(line.suggestions.is_empty());
    assert!(view.has_manual_changes);
}

#[test]
fn assigning_a_rate_that_no_longer_exists_fails() {
    let f = fixture();
    assert!(matches!(
        draft::line_assign_rate(&f.state, f.line_id(0), 9_999),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn adding_a_line_from_a_rate_takes_its_price() {
    let f = fixture();
    let view = draft::line_add(
        &f.state,
        DraftLineInput::Rate {
            rate_id: f.bajante_rate,
            quantity: "2".into(),
        },
    )
    .unwrap();
    assert_eq!(view.lines.len(), 3);
    let line = &view.lines[2];
    assert_eq!(line.concept, "Bajante");
    assert_eq!(line.price_source, PriceSource::Rate);
    assert_eq!(line.amount_display.as_deref(), Some("50,00 €"));
    assert_eq!(view.totals.base_display, "185,00 €");
}

#[test]
fn adding_a_line_by_hand_is_identified_as_manual() {
    let f = fixture();
    let view = draft::line_add(
        &f.state,
        DraftLineInput::Manual {
            concept: "Desplazamiento".into(),
            quantity: "1".into(),
            unit_label: "unidad".into(),
            price: "15".into(),
        },
    )
    .unwrap();
    let line = &view.lines[2];
    assert_eq!(line.price_source, PriceSource::Manual);
    assert_eq!(line.price_cents, Some(1500));
    assert_eq!(line.source_fragment, None);
    assert_eq!(view.totals.base_display, "150,00 €");
}

#[test]
fn a_manual_line_reports_every_invalid_field() {
    let f = fixture();
    let result = draft::line_add(
        &f.state,
        DraftLineInput::Manual {
            concept: " ".into(),
            quantity: "0".into(),
            unit_label: "".into(),
            price: "-3".into(),
        },
    );
    assert_eq!(
        field_codes(result),
        [
            ("concept".to_string(), ErrorCode::ConceptRequired),
            ("quantity".to_string(), ErrorCode::QuantityNotPositive),
            ("unitLabel".to_string(), ErrorCode::UnitRequired),
            ("price".to_string(), ErrorCode::PriceNotPositive),
        ]
    );
    assert_eq!(draft_view(&f.state).lines.len(), 2);
}

#[test]
fn deleting_a_line_updates_the_total() {
    let f = fixture();
    let view = draft::line_delete(&f.state, f.line_id(0)).unwrap();
    assert_eq!(view.lines.len(), 1);
    assert_eq!(view.totals.base_display, "60,00 €");
    assert!(view.has_manual_changes);
}

#[test]
fn deleting_a_line_that_no_longer_exists_fails() {
    let f = fixture();
    assert!(matches!(
        draft::line_delete(&f.state, 9_999),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn a_new_line_goes_after_a_deleted_one() {
    let f = fixture();
    draft::line_delete(&f.state, f.line_id(1)).unwrap();
    let view = draft::line_add(
        &f.state,
        DraftLineInput::Rate {
            rate_id: f.mano_rate,
            quantity: "1".into(),
        },
    )
    .unwrap();
    assert_eq!(view.lines[0].concept, "Bajante");
    assert_eq!(view.lines[1].concept, "Mano de obra");
}

#[test]
fn invalid_values_are_rejected_without_changes() {
    let f = fixture();
    let table = [
        ("", "quantity", ErrorCode::QuantityRequired),
        ("abc", "quantity", ErrorCode::QuantityInvalidFormat),
        ("1,234", "quantity", ErrorCode::QuantityTooManyDecimals),
        ("0", "quantity", ErrorCode::QuantityNotPositive),
        ("-2", "quantity", ErrorCode::QuantityNotPositive),
    ];
    for (value, field, code) in table {
        let result = f.update(
            0,
            DraftLinePatch {
                quantity: Some(value.into()),
                ..patch()
            },
        );
        assert_eq!(
            field_codes(result),
            [(field.to_string(), code)],
            "«{value}»"
        );
    }

    let result = f.update(
        0,
        DraftLinePatch {
            price: Some("12,345".into()),
            ..patch()
        },
    );
    assert_eq!(
        field_codes(result),
        [("price".to_string(), ErrorCode::PriceTooManyDecimals)]
    );

    let result = f.update(
        0,
        DraftLinePatch {
            concept: Some("   ".into()),
            ..patch()
        },
    );
    assert_eq!(
        field_codes(result),
        [("concept".to_string(), ErrorCode::ConceptRequired)]
    );

    let result = f.update(
        0,
        DraftLinePatch {
            unit_label: Some(" ".into()),
            ..patch()
        },
    );
    assert_eq!(
        field_codes(result),
        [("unitLabel".to_string(), ErrorCode::UnitRequired)]
    );

    let view = draft_view(&f.state);
    assert_eq!(view.lines[0].quantity.as_deref(), Some("3"));
    assert!(!view.has_manual_changes);
}

#[test]
fn an_amount_that_does_not_fit_is_rejected() {
    let f = fixture();
    let result = f.update(
        0,
        DraftLinePatch {
            quantity: Some("999999,99".into()),
            price: Some("999999999999".into()),
            ..patch()
        },
    );
    assert_eq!(
        field_codes(result),
        [("price".to_string(), ErrorCode::AmountTooLarge)]
    );
    assert_eq!(draft_view(&f.state).lines[0].quantity.as_deref(), Some("3"));
}

#[test]
fn editing_a_line_that_no_longer_exists_fails() {
    let f = fixture();
    assert!(matches!(
        draft::line_update(
            &f.state,
            9_999,
            DraftLinePatch {
                quantity: Some("2".into()),
                ..patch()
            }
        ),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn editing_without_a_draft_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let rate = create_rate(&state, "Bajante", "metro", "25");
    assert!(matches!(
        draft::line_add(
            &state,
            DraftLineInput::Rate {
                rate_id: rate.id,
                quantity: "1".into()
            }
        ),
        Err(AppError::InvalidState { .. })
    ));
}

#[test]
fn every_edit_is_fast_with_one_hundred_lines() {
    let f = fixture();
    for _ in 0..98 {
        draft::line_add(
            &f.state,
            DraftLineInput::Rate {
                rate_id: f.bajante_rate,
                quantity: "1".into(),
            },
        )
        .unwrap();
    }
    let line_id = f.line_id(50);

    let started = Instant::now();
    let view = draft::line_update(
        &f.state,
        line_id,
        DraftLinePatch {
            quantity: Some("7".into()),
            ..patch()
        },
    )
    .unwrap();
    let elapsed = started.elapsed();

    assert_eq!(view.lines.len(), 100);
    assert_eq!(view.status, DraftStatus::Draft);
    assert!(elapsed.as_millis() < 100, "ha tardado {elapsed:?}");
}
