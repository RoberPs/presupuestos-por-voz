//! Conversión a 16 kHz y mono, por bloques, para grabar e importar sin cargar todo en memoria.

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::audio::SAMPLE_RATE;

const CHUNK_FRAMES: usize = 1_024;

struct Engine {
    resampler: Fft<f32>,
    output: Vec<f32>,
    /// Muestras iniciales de relleno que introduce el conversor.
    delay_left: usize,
}

/// Conversor por bloques. Sin conversión si el audio ya está a 16 kHz.
pub struct StreamResampler {
    engine: Option<Engine>,
    channels: usize,
    input_rate: u64,
    pending: Vec<f32>,
    frames_in: u64,
    frames_out: u64,
}

impl StreamResampler {
    pub fn new(input_rate: u32, channels: usize) -> Result<Self, String> {
        if input_rate == 0 || channels == 0 {
            return Err("formato de audio no válido".into());
        }
        let engine = if input_rate == SAMPLE_RATE {
            None
        } else {
            let resampler = Fft::<f32>::new(
                input_rate as usize,
                SAMPLE_RATE as usize,
                CHUNK_FRAMES,
                1,
                FixedSync::Input,
            )
            .map_err(|e| e.to_string())?;
            Some(Engine {
                output: vec![0.0; resampler.output_frames_max()],
                delay_left: resampler.output_delay(),
                resampler,
            })
        };
        Ok(Self {
            engine,
            channels,
            input_rate: u64::from(input_rate),
            pending: Vec::new(),
            frames_in: 0,
            frames_out: 0,
        })
    }

    /// Mezcla a mono, convierte lo que haya disponible y lo añade a `out`.
    pub fn push(&mut self, interleaved: &[f32], out: &mut Vec<f32>) -> Result<(), String> {
        let channels = self.channels;
        let mono = interleaved
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32);

        if self.engine.is_none() {
            let before = out.len();
            out.extend(mono);
            self.frames_in += (out.len() - before) as u64;
            self.frames_out = self.frames_in;
            return Ok(());
        }

        let before = self.pending.len();
        self.pending.extend(mono);
        self.frames_in += (self.pending.len() - before) as u64;
        self.drain_full_chunks(out)
    }

    fn drain_full_chunks(&mut self, out: &mut Vec<f32>) -> Result<(), String> {
        let Some(engine) = self.engine.as_mut() else {
            return Ok(());
        };
        let mut consumed = 0;
        while self.pending.len() - consumed >= engine.resampler.input_frames_next() {
            let needed = engine.resampler.input_frames_next();
            let chunk = &self.pending[consumed..consumed + needed];
            self.frames_out += process_chunk(engine, chunk, out)?;
            consumed += needed;
        }
        self.pending.drain(..consumed);
        Ok(())
    }

    /// Procesa el audio restante y deja exactamente la duración de la entrada.
    pub fn finish(&mut self, out: &mut Vec<f32>) -> Result<(), String> {
        let expected = self.frames_in * u64::from(SAMPLE_RATE) / self.input_rate;
        let Some(engine) = self.engine.as_mut() else {
            return Ok(());
        };

        let start = out.len();
        let mut produced = self.frames_out;
        while produced < expected {
            let needed = engine.resampler.input_frames_next();
            let mut chunk = std::mem::take(&mut self.pending);
            chunk.resize(needed, 0.0);
            produced += process_chunk(engine, &chunk, out)?;
        }
        let excess = usize::try_from(produced - expected).unwrap_or(usize::MAX);
        out.truncate(out.len().saturating_sub(excess).max(start));
        self.frames_out = expected;
        Ok(())
    }
}

/// Convierte un bloque completo y devuelve las muestras añadidas a `out`.
fn process_chunk(engine: &mut Engine, chunk: &[f32], out: &mut Vec<f32>) -> Result<u64, String> {
    let input = InterleavedSlice::new(chunk, 1, chunk.len()).map_err(|e| e.to_string())?;
    let capacity = engine.output.len();
    let mut output =
        InterleavedSlice::new_mut(&mut engine.output, 1, capacity).map_err(|e| e.to_string())?;
    let (_, written) = engine
        .resampler
        .process_into_buffer(&input, &mut output, None)
        .map_err(|e| e.to_string())?;

    let skipped = engine.delay_left.min(written);
    engine.delay_left -= skipped;
    out.extend_from_slice(&engine.output[skipped..written]);
    Ok((written - skipped) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(rate: u32, channels: usize, frames: usize) -> Vec<f32> {
        let input = vec![0.25; frames * channels];
        let mut resampler = StreamResampler::new(rate, channels).unwrap();
        let mut out = Vec::new();
        for chunk in input.chunks(700 * channels) {
            resampler.push(chunk, &mut out).unwrap();
        }
        resampler.finish(&mut out).unwrap();
        out
    }

    #[test]
    fn audio_at_16k_passes_through() {
        assert_eq!(convert(16_000, 1, 5_000), vec![0.25; 5_000]);
    }

    #[test]
    fn stereo_is_mixed_to_mono() {
        let mut resampler = StreamResampler::new(16_000, 2).unwrap();
        let mut out = Vec::new();
        resampler.push(&[1.0, 0.0, 0.5, 0.5], &mut out).unwrap();
        assert_eq!(out, [0.5, 0.5]);
    }

    #[test]
    fn the_duration_is_preserved() {
        assert_eq!(convert(48_000, 1, 48_000).len(), 16_000);
        assert_eq!(convert(44_100, 2, 44_100).len(), 16_000);
        assert_eq!(convert(8_000, 1, 8_000).len(), 16_000);
    }

    #[test]
    fn a_very_short_audio_is_preserved() {
        assert_eq!(convert(48_000, 1, 300).len(), 100);
    }

    #[test]
    fn an_invalid_format_is_rejected() {
        assert!(StreamResampler::new(0, 1).is_err());
        assert!(StreamResampler::new(44_100, 0).is_err());
    }
}
