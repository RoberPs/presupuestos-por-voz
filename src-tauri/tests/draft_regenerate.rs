//! Corregir el texto y regenerar el borrador (FR-011, FR-012, FR-029 y FR-035).

mod common;

use app_lib::domain::draft::{DraftStatus, TextOrigin};
use app_lib::domain::rate::RateInput;
use app_lib::error::{AppError, ConfirmationReason, ErrorCode};
use app_lib::services::draft_service::{self as draft, DraftLinePatch, GenerateMode};
use app_lib::services::{dictation_service, profile_service, rate_service};
use app_lib::state::AppState;
use common::{
    complete_input, draft_view, example_rates, reopen, seed_draft, state_in, EXAMPLE_TEXT,
};

const CORRECTED: &str = "cambiar bajante, 5 metros, mano de obra 2 horas";

fn fixture() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    example_rates(&state);
    seed_draft(&state, EXAMPLE_TEXT);
    (dir, state)
}

fn regenerate(state: &AppState, text: &str, confirmed: bool) -> Result<draft::DraftView, AppError> {
    draft::generate_from_text(state, text, GenerateMode::Regenerate, confirmed)
}

fn edit_by_hand(state: &AppState) {
    let line_id = draft_view(state).lines[0].id;
    draft::line_update(
        state,
        line_id,
        DraftLinePatch {
            quantity: Some("9".into()),
            ..DraftLinePatch::default()
        },
    )
    .unwrap();
}

#[test]
fn regenerating_applies_the_new_text() {
    let (_dir, state) = fixture();
    let view = regenerate(&state, CORRECTED, false).unwrap();
    assert_eq!(view.lines[0].quantity.as_deref(), Some("5"));
    assert_eq!(view.lines[0].amount_display.as_deref(), Some("125,00 €"));
    assert_eq!(view.source_text, CORRECTED);
    assert!(view.text_corrected);
    assert!(!view.has_manual_changes);
}

#[test]
fn manual_changes_need_confirmation_and_nothing_changes_without_it() {
    let (_dir, state) = fixture();
    edit_by_hand(&state);

    match regenerate(&state, CORRECTED, false) {
        Err(AppError::ConfirmationRequired { reason, .. }) => {
            assert_eq!(reason, ConfirmationReason::DiscardManualChanges)
        }
        other => panic!("resultado inesperado: {other:?}"),
    }

    let view = draft_view(&state);
    assert_eq!(view.lines[0].quantity.as_deref(), Some("9"));
    assert_eq!(view.source_text, EXAMPLE_TEXT);
}

#[test]
fn confirming_replaces_the_lines() {
    let (_dir, state) = fixture();
    edit_by_hand(&state);
    let view = regenerate(&state, CORRECTED, true).unwrap();
    assert_eq!(view.lines[0].quantity.as_deref(), Some("5"));
    assert!(!view.has_manual_changes);
}

#[test]
fn the_saved_text_survives_reopening() {
    let (_dir, state) = fixture();
    draft::save_text(&state, CORRECTED).unwrap();

    let state = reopen(state);

    let view = draft_view(&state);
    assert_eq!(view.source_text, CORRECTED);
    assert!(view.text_corrected);
    assert_eq!(
        view.lines[0].quantity.as_deref(),
        Some("3"),
        "aún no se ha regenerado"
    );
}

#[test]
fn saving_the_same_text_does_not_mark_it_as_corrected() {
    let (_dir, state) = fixture();
    draft::save_text(&state, EXAMPLE_TEXT).unwrap();
    assert!(!draft_view(&state).text_corrected);
}

#[test]
fn regenerating_a_reviewed_draft_returns_it_to_draft() {
    let (_dir, state) = fixture();
    draft::mark_reviewed(&state).unwrap();
    let view = regenerate(&state, CORRECTED, false).unwrap();
    assert_eq!(view.status, DraftStatus::Draft);
}

#[test]
fn regenerating_uses_the_current_prices_and_vat() {
    let (_dir, state) = fixture();
    let rate_id = draft_view(&state).lines[0].rate_id.unwrap();
    rate_service::update(
        &state,
        rate_id,
        RateInput {
            concept: "Bajante".into(),
            unit_label: "metro".into(),
            price: "10".into(),
        },
    )
    .unwrap();
    let mut profile = complete_input();
    profile.vat_rate_percent = "10".into();
    profile_service::save_profile(&state, profile).unwrap();

    let view = regenerate(&state, EXAMPLE_TEXT, false).unwrap();

    assert_eq!(view.lines[0].price_cents, Some(1000));
    assert_eq!(view.totals.vat_rate_percent, "10,00");
    assert_eq!(view.totals.base_display, "90,00 €");
    assert_eq!(view.totals.total_display, "99,00 €");
    assert!(view.vat_change.is_none());
}

#[test]
fn regenerating_keeps_the_origin_of_the_text() {
    let (_dir, state) = fixture();
    let view = regenerate(&state, CORRECTED, false).unwrap();
    assert_eq!(view.text_origin, TextOrigin::Typed);
}

#[test]
fn an_empty_text_cannot_be_regenerated() {
    let (_dir, state) = fixture();
    match regenerate(&state, "   ", false) {
        Err(AppError::Validation { errors }) => {
            assert_eq!(errors[0].field, "text");
            assert_eq!(errors[0].code, ErrorCode::TextRequired);
        }
        other => panic!("resultado inesperado: {other:?}"),
    }
}

#[test]
fn regenerating_without_a_draft_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    assert!(matches!(
        regenerate(&state, CORRECTED, false),
        Err(AppError::InvalidState { .. })
    ));
}

#[test]
fn a_text_without_a_draft_survives_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    draft::save_text(&state, "cambiar grifo").unwrap();

    let state = reopen(state);

    let session = dictation_service::session(&state).unwrap();
    assert_eq!(session.pending_text.as_deref(), Some("cambiar grifo"));
    assert!(session.draft.is_none());
}

#[test]
fn a_text_that_is_too_long_is_rejected() {
    let (_dir, state) = fixture();
    let text = "a".repeat(20_001);
    match draft::save_text(&state, &text) {
        Err(AppError::Validation { errors }) => assert_eq!(errors[0].code, ErrorCode::TooLong),
        other => panic!("resultado inesperado: {other:?}"),
    }
}
