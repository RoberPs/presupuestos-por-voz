//! Ciclo de vida del borrador (FR-029b, FR-033 a FR-036).

mod common;

use app_lib::audio::wav::WavWriter;
use app_lib::domain::draft::{DraftStatus, PriceSource};
use app_lib::domain::rate::RateInput;
use app_lib::error::AppError;
use app_lib::services::draft_service::{self as draft, DraftLinePatch};
use app_lib::services::{dictation_service, profile_service, rate_service};
use app_lib::state::AppState;
use app_lib::storage::{dictation_repo, recording_store};
use common::{
    complete_input, create_rate, draft_view, example_rates, reopen, seed_draft, state_in,
    EXAMPLE_TEXT,
};

fn fixture() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    example_rates(&state);
    seed_draft(&state, EXAMPLE_TEXT);
    (dir, state)
}

fn set_vat(state: &AppState, percent: &str) {
    let mut input = complete_input();
    input.vat_rate_percent = percent.into();
    profile_service::save_profile(state, input).unwrap();
}

#[test]
fn a_modified_rate_does_not_change_the_line() {
    let (_dir, state) = fixture();
    let line = draft_view(&state).lines[0].clone();

    rate_service::update(
        &state,
        line.rate_id.unwrap(),
        RateInput {
            concept: "Bajante".into(),
            unit_label: "metro".into(),
            price: "99".into(),
        },
    )
    .unwrap();

    let view = draft_view(&state);
    assert_eq!(view.lines[0].price_cents, Some(2500));
    assert_eq!(view.totals.total_display, "163,35 €");
}

#[test]
fn a_deleted_rate_does_not_change_the_line() {
    let (_dir, state) = fixture();
    let line = draft_view(&state).lines[0].clone();

    rate_service::delete(&state, line.rate_id.unwrap()).unwrap();

    let view = draft_view(&state);
    assert_eq!(view.lines[0].price_cents, Some(2500));
    assert_eq!(view.lines[0].price_source, PriceSource::Rate);
    assert_eq!(view.lines[0].rate_id, None);
    assert_eq!(view.lines[0].rate_concept.as_deref(), Some("Bajante"));
}

#[test]
fn a_draft_with_pending_lines_cannot_be_marked_as_reviewed() {
    let (_dir, state) = fixture();
    seed_draft(&state, "bajante 3 metros, pintar techo 2 horas");
    let pending = draft_view(&state).lines[1].id;

    match draft::mark_reviewed(&state) {
        Err(AppError::DraftHasPending {
            pending_line_ids, ..
        }) => assert_eq!(pending_line_ids, [pending]),
        other => panic!("resultado inesperado: {other:?}"),
    }
    assert_eq!(draft_view(&state).status, DraftStatus::Draft);
}

#[test]
fn a_draft_without_lines_cannot_be_marked_as_reviewed() {
    let (_dir, state) = fixture();
    for line in draft_view(&state).lines {
        draft::line_delete(&state, line.id).unwrap();
    }
    assert!(matches!(
        draft::mark_reviewed(&state),
        Err(AppError::InvalidState { .. })
    ));
}

#[test]
fn a_complete_draft_can_be_marked_as_reviewed() {
    let (_dir, state) = fixture();
    let view = draft::mark_reviewed(&state).unwrap();
    assert_eq!(view.status, DraftStatus::Reviewed);
}

#[test]
fn editing_a_reviewed_draft_returns_it_to_draft() {
    let (_dir, state) = fixture();
    draft::mark_reviewed(&state).unwrap();
    let line_id = draft_view(&state).lines[0].id;

    let view = draft::line_update(
        &state,
        line_id,
        DraftLinePatch {
            quantity: Some("4".into()),
            ..DraftLinePatch::default()
        },
    )
    .unwrap();

    assert_eq!(view.status, DraftStatus::Draft);
}

#[test]
fn a_vat_change_keeps_the_total_until_it_is_accepted() {
    let (_dir, state) = fixture();
    set_vat(&state, "10");

    let view = draft_view(&state);
    assert_eq!(view.totals.vat_rate_percent, "21,00");
    assert_eq!(view.totals.total_display, "163,35 €");
    assert_eq!(
        view.vat_change.map(|change| change.current_rate_percent),
        Some("10,00".to_string())
    );

    let view = draft::apply_current_vat(&state).unwrap();
    assert_eq!(view.totals.vat_rate_percent, "10,00");
    assert_eq!(view.totals.vat_display, "13,50 €");
    assert_eq!(view.totals.total_display, "148,50 €");
    assert!(view.vat_change.is_none());
}

#[test]
fn applying_the_vat_returns_a_reviewed_draft_to_draft() {
    let (_dir, state) = fixture();
    draft::mark_reviewed(&state).unwrap();
    set_vat(&state, "10");
    let view = draft::apply_current_vat(&state).unwrap();
    assert_eq!(view.status, DraftStatus::Draft);
}

#[test]
fn the_draft_survives_reopening_the_app() {
    let (_dir, state) = fixture();
    let line_id = draft_view(&state).lines[0].id;
    draft::line_update(
        &state,
        line_id,
        DraftLinePatch {
            quantity: Some("4".into()),
            ..DraftLinePatch::default()
        },
    )
    .unwrap();
    draft::mark_reviewed(&state).unwrap();

    let state = reopen(state);

    let view = draft_view(&state);
    assert_eq!(view.status, DraftStatus::Reviewed);
    assert_eq!(view.totals.total_display, "193,60 €");
    assert!(view.has_manual_changes);
    assert_eq!(view.source_text, EXAMPLE_TEXT);
}

#[test]
fn discarding_removes_the_draft_its_text_and_its_audio() {
    let (dir, state) = fixture();
    recording_store::ensure_dir(dir.path()).unwrap();
    let audio_file = "recordings/dictation-1.wav".to_string();
    let path = recording_store::absolute(dir.path(), &audio_file);
    let mut writer = WavWriter::create(&path).unwrap();
    writer.write(&vec![100; 16_000]).unwrap();
    writer.finalize().unwrap();
    dictation_repo::put(
        &state.conn().unwrap(),
        dictation_repo::Slot::Current,
        &dictation_repo::Dictation {
            audio_file,
            duration_ms: 1_000,
            origin: dictation_repo::DictationOrigin::Recorded,
            original_file_name: None,
            status: dictation_repo::DictationStatus::Transcribed,
            low_quality: false,
            error_code: None,
        },
    )
    .unwrap();

    draft::discard(&state).unwrap();

    let session = dictation_service::session(&state).unwrap();
    assert!(session.draft.is_none());
    assert!(session.dictation.is_none());
    assert!(session.pending_text.is_none());
    assert!(!path.exists());
}

#[test]
fn a_new_rate_is_not_applied_to_an_existing_draft() {
    let (_dir, state) = fixture();
    seed_draft(&state, "pintar techo 2 horas");
    create_rate(&state, "Pintar techo", "hora", "20");
    assert_eq!(draft_view(&state).lines[0].price_cents, None);
}
