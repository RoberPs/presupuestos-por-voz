//! Dictado de principio a fin con el motor real y una voz sintética (escenario 1 de quickstart.md).
//!
//! Necesita el modelo (`npm run model:fetch`); sin él, la prueba se omite.
#![cfg(feature = "local-transcription")]

mod common;

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use app_lib::audio::{wav, UnavailableRecorder};
use app_lib::error::{AppError, TranscriptionCode};
use app_lib::services::dictation_service::{self as dictation, ProcessOutcome};
use app_lib::state::AppState;
use app_lib::storage::db;
use app_lib::transcription::whisper::WhisperTranscriber;
use app_lib::transcription::{TranscribeFailure, TranscribeOptions, Transcriber};
use common::example_rates;

fn manifest_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn state_with_model(dir: &Path, model: std::path::PathBuf) -> AppState {
    AppState::with_engines(
        db::open(dir).unwrap(),
        dir.to_path_buf(),
        Arc::new(WhisperTranscriber::new(model)),
        Arc::new(UnavailableRecorder),
    )
}

#[test]
fn the_spoken_example_becomes_the_expected_draft() {
    let model = manifest_path("resources/models/ggml-small-q5_1.bin");
    if !model.is_file() {
        eprintln!("Prueba omitida: falta el modelo. Ejecuta `npm run model:fetch`.");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let state = state_with_model(dir.path(), model);
    example_rates(&state);

    let audio = manifest_path("tests/fixtures/audio/ejemplo-voz.wav");
    dictation::import_file(&state, &audio, false).unwrap();
    let result = dictation::process(&state, None).unwrap();

    assert_eq!(result.outcome, ProcessOutcome::DraftReady);
    let draft = result.session.draft.expect("debe haber borrador");
    assert_eq!(draft.lines.len(), 2, "texto: {}", draft.source_text);
    assert_eq!(
        draft.totals.base_display, "135,00 €",
        "texto: {}",
        draft.source_text
    );
    assert_eq!(draft.totals.vat_display, "28,35 €");
    assert_eq!(draft.totals.total_display, "163,35 €");
    assert!(result.session.dictation.unwrap().transcribed_locally);
}

#[test]
fn cancelling_stops_the_real_engine() {
    let model = manifest_path("resources/models/ggml-small-q5_1.bin");
    if !model.is_file() {
        eprintln!("Prueba omitida: falta el modelo. Ejecuta `npm run model:fetch`.");
        return;
    }
    let samples =
        wav::read_samples(&manifest_path("tests/fixtures/audio/ejemplo-voz.wav")).unwrap();
    let cancel = Arc::new(AtomicBool::new(true));

    let result = WhisperTranscriber::new(model).transcribe(
        &samples,
        &TranscribeOptions::default(),
        &|_| {},
        &cancel,
    );

    assert_eq!(result, Err(TranscribeFailure::Cancelled));
}

#[test]
fn a_missing_model_is_reported_and_keeps_the_audio() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_with_model(dir.path(), dir.path().join("no-existe.bin"));

    let audio = manifest_path("tests/fixtures/audio/ejemplo-voz.wav");
    dictation::import_file(&state, &audio, false).unwrap();
    let error = dictation::process(&state, None).unwrap_err();

    assert!(matches!(
        error,
        AppError::Transcription {
            code: TranscriptionCode::ModelUnavailable,
            ..
        }
    ));
    let session = dictation::session(&state).unwrap();
    assert_eq!(
        session.dictation.unwrap().error_code,
        Some(TranscriptionCode::ModelUnavailable)
    );
}
