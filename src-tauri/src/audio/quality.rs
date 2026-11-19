//! Silencio y calidad baja (research.md, R10). Los umbrales se ajustan con audios reales (T077).

use crate::audio::SAMPLE_RATE;

/// Ventanas de 20 ms.
const WINDOW_SAMPLES: usize = SAMPLE_RATE as usize / 50;
/// Energía mínima de una ventana con voz (unos -40 dBFS).
const VOICE_RMS: f32 = 0.01;
/// Con menos de 300 ms de voz se considera silencio.
const MIN_VOICE_WINDOWS: usize = 15;
/// Por debajo de esta probabilidad media de las palabras, conviene revisar el texto (FR-007).
const LOW_QUALITY_PROBABILITY: f32 = 0.5;

/// Indica si el audio contiene voz suficiente para intentar transcribirlo.
pub fn has_speech(samples: &[f32]) -> bool {
    samples
        .as_chunks::<WINDOW_SAMPLES>()
        .0
        .iter()
        .filter(|window| rms(window.as_slice()) >= VOICE_RMS)
        .take(MIN_VOICE_WINDOWS)
        .count()
        >= MIN_VOICE_WINDOWS
}

/// Nivel de energía de un grupo de muestras.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

pub fn is_low_quality(average_word_probability: f32) -> bool {
    average_word_probability < LOW_QUALITY_PROBABILITY
}
