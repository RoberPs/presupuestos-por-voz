//! Dictado de principio a fin con motor y micrófono simulados (Principio III y SC-008).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use app_lib::audio::wav::WavWriter;
use app_lib::audio::{ActiveRecording, Recorder, RecordingSink};
use app_lib::domain::rate::RateInput;
use app_lib::error::{AppError, AudioRejectCode, ConfirmationReason, TranscriptionCode};
use app_lib::services::dictation_service::{self as dictation, ActivityView, ProcessOutcome};
use app_lib::services::rate_service;
use app_lib::state::AppState;
use app_lib::storage::{db, recording_store};
use app_lib::transcription::{TranscribeFailure, TranscribeOptions, Transcriber, Transcript};

const EXAMPLE: &str = "cambiar bajante, 3 metros, mano de obra 2 horas";

type Outcome = Result<Transcript, TranscribeFailure>;

/// Motor simulado: devuelve el resultado configurado y cuenta las llamadas.
struct FakeTranscriber {
    outcome: Mutex<Outcome>,
    calls: AtomicUsize,
}

impl FakeTranscriber {
    fn new(outcome: Outcome) -> Arc<Self> {
        Arc::new(Self {
            outcome: Mutex::new(outcome),
            calls: AtomicUsize::new(0),
        })
    }

    fn set(&self, outcome: Outcome) {
        *self.outcome.lock().unwrap() = outcome;
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Transcriber for FakeTranscriber {
    fn transcribe(
        &self,
        _samples: &[f32],
        _options: &TranscribeOptions,
        progress: &(dyn Fn(u8) + Send + Sync),
        _cancel: &Arc<AtomicBool>,
    ) -> Outcome {
        self.calls.fetch_add(1, Ordering::SeqCst);
        progress(50);
        self.outcome.lock().unwrap().clone()
    }
}

/// Micrófono simulado: al parar escribe `samples` muestras en el archivo temporal.
struct FakeRecorder {
    samples: usize,
}

struct FakeRecording {
    path: PathBuf,
    samples: usize,
}

impl Recorder for FakeRecorder {
    fn start(
        &self,
        part_path: &Path,
        _events: RecordingSink,
    ) -> Result<Box<dyn ActiveRecording>, AppError> {
        std::fs::write(part_path, b"").unwrap();
        Ok(Box::new(FakeRecording {
            path: part_path.to_path_buf(),
            samples: self.samples,
        }))
    }
}

impl ActiveRecording for FakeRecording {
    fn stop(self: Box<Self>) -> Result<i64, AppError> {
        let mut writer = WavWriter::create(&self.path).unwrap();
        writer.write(&speech(self.samples)).unwrap();
        Ok(writer.finalize().unwrap())
    }
}

fn speech(samples: usize) -> Vec<i16> {
    (0..samples)
        .map(|i| ((i as f32 * 0.17).sin() * 9_000.0) as i16)
        .collect()
}

fn text(value: &str) -> Outcome {
    Ok(Transcript {
        text: value.into(),
        low_quality: false,
    })
}

fn failed(code: TranscriptionCode) -> Outcome {
    Err(TranscribeFailure::Failed(code))
}

struct Fixture {
    dir: tempfile::TempDir,
    state: AppState,
    transcriber: Arc<FakeTranscriber>,
}

impl Fixture {
    fn new(outcome: Outcome) -> Self {
        Self::with_recorder(outcome, 16_000)
    }

    fn with_recorder(outcome: Outcome, recorded_samples: usize) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let transcriber = FakeTranscriber::new(outcome);
        let state = AppState::with_engines(
            db::open(dir.path()).unwrap(),
            dir.path().to_path_buf(),
            transcriber.clone(),
            Arc::new(FakeRecorder {
                samples: recorded_samples,
            }),
        );
        for (concept, unit, price) in [("Bajante", "metro", "25"), ("Mano de obra", "hora", "30")] {
            rate_service::create(
                &state,
                RateInput {
                    concept: concept.into(),
                    unit_label: unit.into(),
                    price: price.into(),
                },
            )
            .unwrap();
        }
        Self {
            dir,
            state,
            transcriber,
        }
    }

    /// Audio con sonido, listo para importar.
    fn audio_file(&self) -> PathBuf {
        let path = self.dir.path().join("visita.wav");
        let mut writer = WavWriter::create(&path).unwrap();
        writer.write(&speech(16_000)).unwrap();
        writer.finalize().unwrap();
        path
    }

    fn silent_file(&self) -> PathBuf {
        let path = self.dir.path().join("silencio.wav");
        let mut writer = WavWriter::create(&path).unwrap();
        writer.write(&vec![0; 16_000]).unwrap();
        writer.finalize().unwrap();
        path
    }

    fn import(&self, confirm: bool) {
        dictation::import_file(&self.state, &self.audio_file(), confirm).unwrap();
    }

    /// Deja un borrador generado a partir de un dictado.
    fn with_draft(self) -> Self {
        self.import(false);
        let result = dictation::process(&self.state, None).unwrap();
        assert_eq!(result.outcome, ProcessOutcome::DraftReady);
        self
    }

    fn recordings(&self) -> Vec<String> {
        let mut names: Vec<String> =
            std::fs::read_dir(recording_store::recordings_dir(self.dir.path()))
                .map(|entries| {
                    entries
                        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
        names.sort();
        names
    }

    fn total(&self) -> Option<String> {
        dictation::session(&self.state)
            .unwrap()
            .draft
            .map(|d| d.totals.total_display)
    }
}

fn confirmation_reason(result: Result<impl std::fmt::Debug, AppError>) -> ConfirmationReason {
    match result {
        Err(AppError::ConfirmationRequired { reason, .. }) => reason,
        other => panic!("se esperaba una confirmación y se obtuvo {other:?}"),
    }
}

#[test]
fn a_dictation_ends_in_a_draft() {
    let f = Fixture::new(text(EXAMPLE));
    f.import(false);

    let result = dictation::process(&f.state, None).unwrap();

    assert_eq!(result.outcome, ProcessOutcome::DraftReady);
    let draft = result.session.draft.expect("debe haber borrador");
    assert_eq!(draft.lines.len(), 2);
    assert_eq!(draft.source_text, EXAMPLE);
    assert_eq!(draft.totals.base_display, "135,00 €");
    assert_eq!(draft.totals.vat_display, "28,35 €");
    assert_eq!(draft.totals.total_display, "163,35 €");
    let dictation_view = result.session.dictation.expect("debe haber dictado");
    assert!(dictation_view.transcribed_locally);
    assert_eq!(result.session.activity, ActivityView::Idle);
}

#[test]
fn low_quality_is_reported() {
    let f = Fixture::new(Ok(Transcript {
        text: EXAMPLE.into(),
        low_quality: true,
    }));
    f.import(false);
    let result = dictation::process(&f.state, None).unwrap();
    assert!(result.session.dictation.unwrap().low_quality);
}

#[test]
fn silence_creates_no_draft_and_keeps_the_previous_one() {
    let f = Fixture::new(text(EXAMPLE)).with_draft();
    let calls = f.transcriber.calls();
    let silent = f.silent_file();
    dictation::import_file(&f.state, &silent, true).unwrap();

    let result = dictation::process(&f.state, None).unwrap();

    assert_eq!(result.outcome, ProcessOutcome::NoSpeech);
    assert_eq!(f.transcriber.calls(), calls, "no hace falta usar el motor");
    assert_eq!(f.total().as_deref(), Some("163,35 €"));
}

#[test]
fn an_empty_transcript_counts_as_no_speech() {
    let f = Fixture::new(text("   "));
    f.import(false);
    let result = dictation::process(&f.state, None).unwrap();
    assert_eq!(result.outcome, ProcessOutcome::NoSpeech);
    assert!(result.session.draft.is_none());
}

#[test]
fn a_missing_model_keeps_the_audio_and_can_be_retried() {
    let f = Fixture::new(failed(TranscriptionCode::ModelUnavailable));
    f.import(false);
    let audio = f.recordings();

    let error = dictation::process(&f.state, None).unwrap_err();

    match error {
        AppError::Transcription { code, .. } => {
            assert_eq!(code, TranscriptionCode::ModelUnavailable)
        }
        other => panic!("error inesperado: {other:?}"),
    }
    assert_eq!(f.recordings(), audio, "el audio se conserva");
    let session = dictation::session(&f.state).unwrap();
    assert_eq!(
        session.dictation.unwrap().error_code,
        Some(TranscriptionCode::ModelUnavailable)
    );
    assert_eq!(session.activity, ActivityView::Idle);

    // Al restaurar el modelo, el reintento termina bien (Principio III).
    f.transcriber.set(text(EXAMPLE));
    let retried = dictation::process(&f.state, None).unwrap();
    assert_eq!(retried.outcome, ProcessOutcome::DraftReady);
}

#[test]
fn a_failure_keeps_the_audio_and_the_previous_draft() {
    let f = Fixture::new(text(EXAMPLE)).with_draft();
    f.transcriber
        .set(failed(TranscriptionCode::TranscriptionFailed));
    f.import(true);
    let audio = f.recordings();

    assert!(dictation::process(&f.state, None).is_err());

    assert_eq!(f.recordings(), audio);
    assert_eq!(f.total().as_deref(), Some("163,35 €"));
}

#[test]
fn cancelling_keeps_the_audio_and_the_previous_draft() {
    let f = Fixture::new(text(EXAMPLE)).with_draft();
    f.transcriber.set(Err(TranscribeFailure::Cancelled));
    f.import(true);
    let audio = f.recordings();

    let result = dictation::process(&f.state, None).unwrap();

    assert_eq!(result.outcome, ProcessOutcome::Cancelled);
    assert_eq!(f.recordings(), audio);
    assert_eq!(f.total().as_deref(), Some("163,35 €"));

    // El audio sigue disponible para procesarlo otra vez.
    f.transcriber.set(text("bajante 4 metros"));
    let retried = dictation::process(&f.state, None).unwrap();
    assert_eq!(retried.outcome, ProcessOutcome::DraftReady);
    assert_eq!(f.total().as_deref(), Some("121,00 €"));
}

#[test]
fn a_new_draft_replaces_the_previous_audio() {
    let f = Fixture::new(text(EXAMPLE)).with_draft();
    f.import(true);
    dictation::process(&f.state, None).unwrap();
    assert_eq!(f.recordings().len(), 1, "solo queda el audio vigente");
}

#[test]
fn processing_without_audio_is_rejected() {
    let f = Fixture::new(text(EXAMPLE));
    assert!(matches!(
        dictation::process(&f.state, None),
        Err(AppError::InvalidState { .. })
    ));
    assert_eq!(
        dictation::session(&f.state).unwrap().activity,
        ActivityView::Idle
    );
}

#[test]
fn an_interrupted_recording_can_be_recovered() {
    let f = Fixture::new(text(EXAMPLE));
    recording_store::ensure_dir(f.dir.path()).unwrap();
    let part = recording_store::part_path(f.dir.path());
    let mut writer = WavWriter::create(&part).unwrap();
    writer.write(&speech(16_000)).unwrap();
    writer.flush().unwrap();
    drop(writer);

    let session = dictation::session(&f.state).unwrap();
    assert_eq!(session.recovery.map(|r| r.duration_ms), Some(1_000));

    let session = dictation::recover(&f.state).unwrap();
    assert!(session.recovery.is_none());
    assert!(!part.exists());
    assert_eq!(session.dictation.unwrap().duration_ms, 1_000);

    let result = dictation::process(&f.state, None).unwrap();
    assert_eq!(result.outcome, ProcessOutcome::DraftReady);
}

#[test]
fn an_interrupted_recording_can_be_discarded() {
    let f = Fixture::new(text(EXAMPLE));
    recording_store::ensure_dir(f.dir.path()).unwrap();
    let part = recording_store::part_path(f.dir.path());
    let mut writer = WavWriter::create(&part).unwrap();
    writer.write(&speech(1_000)).unwrap();
    writer.flush().unwrap();
    drop(writer);

    let session = dictation::discard(&f.state).unwrap();
    assert!(session.recovery.is_none());
    assert!(!part.exists());
}

#[test]
fn a_rejected_import_changes_nothing() {
    let f = Fixture::new(text(EXAMPLE)).with_draft();
    f.import(true);
    let audio = f.recordings();
    let bad = f.dir.path().join("malo.mp3");
    std::fs::write(&bad, "no soy un audio ".repeat(100)).unwrap();

    let error = dictation::import_file(&f.state, &bad, true).unwrap_err();

    assert!(matches!(
        error,
        AppError::AudioRejected {
            code: AudioRejectCode::AudioUnsupportedFormat,
            ..
        }
    ));
    assert_eq!(f.recordings(), audio, "el audio pendiente sigue ahí");
    assert_eq!(f.total().as_deref(), Some("163,35 €"));
    let session = dictation::session(&f.state).unwrap();
    assert_eq!(session.activity, ActivityView::Idle);
    assert!(session.dictation.is_some());
}

#[test]
fn recording_over_a_draft_needs_confirmation() {
    let f = Fixture::new(text(EXAMPLE)).with_draft();
    assert_eq!(
        confirmation_reason(dictation::start(&f.state, false, None)),
        ConfirmationReason::ReplaceDraft
    );
    assert_eq!(
        confirmation_reason(dictation::import_file(&f.state, &f.audio_file(), false)),
        ConfirmationReason::ReplaceDraft
    );
    dictation::start(&f.state, true, None).unwrap();
    dictation::stop(&f.state).unwrap();
}

#[test]
fn pending_audio_needs_confirmation() {
    let f = Fixture::new(text(EXAMPLE));
    f.import(false);
    assert_eq!(
        confirmation_reason(dictation::start(&f.state, false, None)),
        ConfirmationReason::ReplacePendingAudio
    );
    assert_eq!(
        confirmation_reason(dictation::import_file(&f.state, &f.audio_file(), false)),
        ConfirmationReason::ReplacePendingAudio
    );
}

#[test]
fn a_recoverable_recording_needs_confirmation() {
    let f = Fixture::new(text(EXAMPLE));
    recording_store::ensure_dir(f.dir.path()).unwrap();
    let mut writer = WavWriter::create(&recording_store::part_path(f.dir.path())).unwrap();
    writer.write(&speech(1_000)).unwrap();
    writer.flush().unwrap();
    drop(writer);

    assert_eq!(
        confirmation_reason(dictation::start(&f.state, false, None)),
        ConfirmationReason::ReplacePendingAudio
    );
}

#[test]
fn stopping_creates_the_incoming_audio() {
    let f = Fixture::new(text(EXAMPLE));
    dictation::start(&f.state, false, None).unwrap();
    assert!(matches!(
        dictation::session(&f.state).unwrap().activity,
        ActivityView::Recording { .. }
    ));
    assert!(matches!(
        dictation::start(&f.state, true, None),
        Err(AppError::InvalidState { .. })
    ));

    let session = dictation::stop(&f.state).unwrap();

    assert_eq!(session.activity, ActivityView::Idle);
    assert_eq!(session.dictation.unwrap().duration_ms, 1_000);
    assert!(session.recovery.is_none());
    assert!(!recording_store::part_path(f.dir.path()).exists());
}

#[test]
fn stopping_without_audio_creates_nothing() {
    let f = Fixture::with_recorder(text(EXAMPLE), 0);
    dictation::start(&f.state, false, None).unwrap();

    let session = dictation::stop(&f.state).unwrap();

    assert!(session.dictation.is_none());
    assert!(session.recovery.is_none());
    assert!(f.recordings().is_empty());
}

#[test]
fn discarding_keeps_the_current_draft() {
    let f = Fixture::new(text(EXAMPLE)).with_draft();
    f.import(true);
    let session = dictation::discard(&f.state).unwrap();
    assert!(session.draft.is_some());
    assert_eq!(
        f.recordings().len(),
        1,
        "queda el audio del borrador vigente"
    );
}

#[test]
fn a_processing_interrupted_by_a_crash_can_be_processed_again() {
    let f = Fixture::new(text(EXAMPLE));
    f.import(false);
    {
        let conn = f.state.conn().unwrap();
        conn.execute("UPDATE dictations SET status = 'processing'", [])
            .unwrap();
        app_lib::storage::dictation_repo::reset_processing(&conn).unwrap();
    }
    let result = dictation::process(&f.state, None).unwrap();
    assert_eq!(result.outcome, ProcessOutcome::DraftReady);
}
