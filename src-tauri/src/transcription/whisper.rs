//! Motor real: whisper.cpp en el propio equipo (research.md, R1, R10 y R11).
//!
//! El modelo se carga al empezar a transcribir y se libera al terminar, para no ocupar memoria.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::audio::quality::is_low_quality;
use crate::error::TranscriptionCode;
use crate::transcription::{TranscribeFailure, TranscribeOptions, Transcriber, Transcript};

const MAX_THREADS: usize = 4;
/// Se descartan los segmentos que probablemente no son voz, para no inventar texto con ruido.
const MAX_NO_SPEECH_PROBABILITY: f32 = 0.6;
const PROGRESS_POLL: Duration = Duration::from_millis(200);

pub struct WhisperTranscriber {
    model_path: PathBuf,
}

impl WhisperTranscriber {
    pub fn new(model_path: PathBuf) -> Self {
        Self { model_path }
    }
}

fn failed() -> TranscribeFailure {
    TranscribeFailure::Failed(TranscriptionCode::TranscriptionFailed)
}

fn model_unavailable() -> TranscribeFailure {
    TranscribeFailure::Failed(TranscriptionCode::ModelUnavailable)
}

/// Une los segmentos con voz y calcula la confianza media de sus palabras.
fn collect(state: &WhisperState) -> Result<Transcript, TranscribeFailure> {
    let mut text = String::new();
    let mut probability_sum = 0.0;
    let mut tokens = 0u32;

    for segment in state.as_iter() {
        if segment.no_speech_probability() > MAX_NO_SPEECH_PROBABILITY {
            continue;
        }
        let piece = segment.to_str_lossy().map_err(|e| {
            log::error!("No se ha podido leer el texto transcrito: {e}");
            failed()
        })?;
        text.push_str(&piece);
        for index in 0..segment.n_tokens() {
            if let Some(token) = segment.get_token(index) {
                probability_sum += token.token_probability();
                tokens += 1;
            }
        }
    }

    let average = if tokens == 0 {
        1.0
    } else {
        probability_sum / tokens as f32
    };
    Ok(Transcript {
        text: text.trim().to_string(),
        low_quality: is_low_quality(average),
    })
}

impl Transcriber for WhisperTranscriber {
    fn transcribe(
        &self,
        samples: &[f32],
        options: &TranscribeOptions,
        progress: &(dyn Fn(u8) + Send + Sync),
        cancel: &Arc<AtomicBool>,
    ) -> Result<Transcript, TranscribeFailure> {
        if !self.model_path.is_file() {
            log::error!(
                "No se encuentra el modelo de voz: {}",
                self.model_path.display()
            );
            return Err(model_unavailable());
        }
        let model_path = self.model_path.to_str().ok_or_else(model_unavailable)?;
        let context =
            WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
                .map_err(|e| {
                    log::error!("No se ha podido cargar el modelo de voz: {e}");
                    model_unavailable()
                })?;
        let mut state = context.create_state().map_err(|e| {
            log::error!("No se ha podido preparar el motor de voz: {e}");
            failed()
        })?;

        let threads =
            std::thread::available_parallelism().map_or(MAX_THREADS, |n| n.get().min(MAX_THREADS));
        let percent = Arc::new(AtomicU8::new(0));

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("es"));
        params.set_n_threads(threads as i32);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        if !options.vocabulary.is_empty() {
            params.set_initial_prompt(&options.vocabulary);
        }
        {
            let percent = percent.clone();
            params.set_progress_callback_safe(move |value: i32| {
                percent.store(value.clamp(0, 100) as u8, Ordering::Relaxed);
            });
        }
        {
            let cancel = cancel.clone();
            // `whisper-rs` 0.16 guarda el aviso como `Box<dyn FnMut>` pero lo lee con el tipo
            // recibido: hay que pasárselo ya como `Box<dyn FnMut>` para que ambos coincidan.
            let abort: Box<dyn FnMut() -> bool> = Box::new(move || cancel.load(Ordering::Relaxed));
            params.set_abort_callback_safe(abort);
        }

        // El motor solo admite avisos sin referencias prestadas: otro hilo los reenvía.
        let finished = AtomicBool::new(false);
        let result = std::thread::scope(|scope| {
            scope.spawn(|| {
                let mut last = 0;
                while !finished.load(Ordering::Relaxed) {
                    let current = percent.load(Ordering::Relaxed);
                    if current != last {
                        last = current;
                        progress(current);
                    }
                    std::thread::sleep(PROGRESS_POLL);
                }
            });
            let result = state.full(params, samples);
            finished.store(true, Ordering::Relaxed);
            result
        });

        if cancel.load(Ordering::SeqCst) {
            return Err(TranscribeFailure::Cancelled);
        }
        result.map_err(|e| {
            log::error!("La transcripción ha fallado: {e}");
            failed()
        })?;
        collect(&state)
    }
}
