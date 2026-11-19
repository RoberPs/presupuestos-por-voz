//! Escribir o pegar la visita sin dictar (FR-013, FR-014 y SC-007).

mod common;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use app_lib::audio::wav::WavWriter;
use app_lib::audio::UnavailableRecorder;
use app_lib::domain::draft::TextOrigin;
use app_lib::error::{AppError, ConfirmationReason, ErrorCode};
use app_lib::services::dictation_service;
use app_lib::services::draft_service::{self as draft, GenerateMode};
use app_lib::state::AppState;
use app_lib::storage::{db, recording_store};
use app_lib::transcription::{TranscribeFailure, TranscribeOptions, Transcriber, Transcript};
use common::{draft_view, example_rates, state_in, EXAMPLE_TEXT};

struct FixedTranscriber;

impl Transcriber for FixedTranscriber {
    fn transcribe(
        &self,
        _samples: &[f32],
        _options: &TranscribeOptions,
        _progress: &(dyn Fn(u8) + Send + Sync),
        _cancel: &Arc<AtomicBool>,
    ) -> Result<Transcript, TranscribeFailure> {
        Ok(Transcript {
            text: EXAMPLE_TEXT.into(),
            low_quality: false,
        })
    }
}

fn generate(state: &AppState, text: &str, confirmed: bool) -> Result<draft::DraftView, AppError> {
    draft::generate_from_text(state, text, GenerateMode::New, confirmed)
}

/// Estado con motor simulado y un dictado ya convertido en borrador.
fn dictated_state(dir: &std::path::Path) -> AppState {
    let state = AppState::with_engines(
        db::open(dir).unwrap(),
        dir.to_path_buf(),
        Arc::new(FixedTranscriber),
        Arc::new(UnavailableRecorder),
    );
    example_rates(&state);

    let audio = dir.join("visita.wav");
    let samples: Vec<i16> = (0..16_000)
        .map(|i| ((i as f32 * 0.17).sin() * 9_000.0) as i16)
        .collect();
    let mut writer = WavWriter::create(&audio).unwrap();
    writer.write(&samples).unwrap();
    writer.finalize().unwrap();
    dictation_service::import_file(&state, &audio, false).unwrap();
    dictation_service::process(&state, None).unwrap();
    state
}

fn recordings(dir: &std::path::Path) -> usize {
    std::fs::read_dir(recording_store::recordings_dir(dir))
        .map(|entries| entries.count())
        .unwrap_or(0)
}

#[test]
fn a_typed_text_gives_the_same_draft_as_a_dictated_one() {
    let dictated_dir = tempfile::tempdir().unwrap();
    let dictated = dictated_state(dictated_dir.path());
    let from_voice = draft_view(&dictated);

    let typed_dir = tempfile::tempdir().unwrap();
    let typed = state_in(typed_dir.path());
    example_rates(&typed);
    let from_text = generate(&typed, EXAMPLE_TEXT, false).unwrap();

    assert_eq!(from_text.totals, from_voice.totals);
    assert_eq!(from_text.unparsed_fragments, from_voice.unparsed_fragments);
    assert_eq!(from_text.lines.len(), from_voice.lines.len());
    for (typed_line, voice_line) in from_text.lines.iter().zip(&from_voice.lines) {
        assert_eq!(typed_line.concept, voice_line.concept);
        assert_eq!(typed_line.quantity, voice_line.quantity);
        assert_eq!(typed_line.unit_label, voice_line.unit_label);
        assert_eq!(typed_line.price_cents, voice_line.price_cents);
        assert_eq!(typed_line.amount_cents, voice_line.amount_cents);
        assert_eq!(typed_line.price_source, voice_line.price_source);
    }
    assert_eq!(from_text.text_origin, TextOrigin::Typed);
    assert_eq!(from_voice.text_origin, TextOrigin::Transcribed);
}

#[test]
fn an_empty_text_asks_to_write_something() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    for text in ["", "   \n "] {
        match generate(&state, text, false) {
            Err(AppError::Validation { errors }) => {
                assert_eq!(errors[0].field, "text");
                assert_eq!(errors[0].code, ErrorCode::TextRequired);
            }
            other => panic!("resultado inesperado: {other:?}"),
        }
    }
}

#[test]
fn generating_clears_the_text_pending_of_a_draft() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    example_rates(&state);
    draft::save_text(&state, EXAMPLE_TEXT).unwrap();

    generate(&state, EXAMPLE_TEXT, false).unwrap();

    let session = dictation_service::session(&state).unwrap();
    assert!(session.pending_text.is_none());
    assert!(session.draft.is_some());
}

#[test]
fn an_existing_draft_needs_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let state = dictated_state(dir.path());

    match generate(&state, "bajante 9 metros", false) {
        Err(AppError::ConfirmationRequired { reason, .. }) => {
            assert_eq!(reason, ConfirmationReason::ReplaceDraft)
        }
        other => panic!("resultado inesperado: {other:?}"),
    }

    assert_eq!(draft_view(&state).totals.total_display, "163,35 €");
    assert_eq!(recordings(dir.path()), 1, "el audio sigue guardado");
}

#[test]
fn confirming_replaces_the_draft_and_deletes_the_previous_audio() {
    let dir = tempfile::tempdir().unwrap();
    let state = dictated_state(dir.path());

    let view = generate(&state, "bajante 4 metros", true).unwrap();

    assert_eq!(view.totals.total_display, "121,00 €");
    assert_eq!(view.text_origin, TextOrigin::Typed);
    assert_eq!(recordings(dir.path()), 0);
    assert!(dictation_service::session(&state)
        .unwrap()
        .dictation
        .is_none());
}
