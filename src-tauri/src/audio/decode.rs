//! Importación de audios M4A, MP3 y WAV a un WAV propio (FR-002b y FR-002c; research.md, R4).
//!
//! La decodificación es en flujo y se detiene al superar los 10 minutos, así un archivo enorme no
//! agota la memoria.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::audio::resample::StreamResampler;
use crate::audio::wav::{to_i16, WavWriter};
use crate::audio::MAX_DURATION_MS;
use crate::error::{AppError, AudioRejectCode};

fn rejected(code: AudioRejectCode) -> AppError {
    AppError::audio_rejected(code)
}

/// Comprueba por su cabecera si el archivo dice ser un audio, para distinguir «dañado» de «otro tipo».
fn looks_like_audio(path: &Path) -> bool {
    let mut head = [0u8; 12];
    let read = File::open(path)
        .and_then(|mut f| f.read(&mut head))
        .unwrap_or(0);
    if read < 4 {
        return false;
    }
    &head[0..4] == b"RIFF"
        || &head[0..3] == b"ID3"
        || (read >= 8 && &head[4..8] == b"ftyp")
        || (head[0] == 0xFF && head[1] & 0xE0 == 0xE0)
}

/// Escribe el audio convertido y controla la duración.
struct Output {
    resampler: StreamResampler,
    writer: WavWriter,
    converted: Vec<f32>,
    pcm: Vec<i16>,
}

impl Output {
    fn store_converted(&mut self) -> Result<(), AppError> {
        self.pcm.clear();
        self.pcm.extend(self.converted.iter().map(|s| to_i16(*s)));
        self.converted.clear();
        self.writer.write(&self.pcm)?;
        Ok(())
    }

    fn push(&mut self, interleaved: &[f32]) -> Result<(), AppError> {
        self.resampler
            .push(interleaved, &mut self.converted)
            .map_err(|e| {
                log::error!("No se ha podido convertir el audio: {e}");
                rejected(AudioRejectCode::AudioCorrupt)
            })?;
        self.store_converted()
    }

    fn finish(mut self) -> Result<i64, AppError> {
        self.resampler.finish(&mut self.converted).map_err(|e| {
            log::error!("No se ha podido convertir el audio: {e}");
            rejected(AudioRejectCode::AudioCorrupt)
        })?;
        self.store_converted()?;
        Ok(self.writer.finalize()?)
    }
}

fn decode(src: &Path, dest: &Path) -> Result<i64, AppError> {
    let file = File::open(src).map_err(|e| {
        log::warn!("No se ha podido abrir el audio: {e}");
        rejected(AudioRejectCode::AudioCorrupt)
    })?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = src.extension().and_then(|e| e.to_str()) {
        hint.with_extension(extension);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| {
            log::warn!("Formato de audio no reconocido: {e}");
            rejected(if looks_like_audio(src) {
                AudioRejectCode::AudioCorrupt
            } else {
                AudioRejectCode::AudioUnsupportedFormat
            })
        })?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| rejected(AudioRejectCode::AudioUnsupportedFormat))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(|| rejected(AudioRejectCode::AudioUnsupportedFormat))?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| {
            log::warn!("Códec de audio no admitido: {e}");
            rejected(AudioRejectCode::AudioUnsupportedFormat)
        })?;

    let mut output: Option<Output> = None;
    let mut frames: u64 = 0;
    let mut interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) | Err(DecodeError::ResetRequired) => break,
            Err(e) => {
                log::warn!("El audio está dañado: {e}");
                return Err(rejected(AudioRejectCode::AudioCorrupt));
            }
        };
        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            // Un paquete defectuoso se salta: el resto del audio sigue siendo útil.
            Err(DecodeError::DecodeError(e)) => {
                log::warn!("Paquete de audio defectuoso: {e}");
                continue;
            }
            Err(e) => {
                log::warn!("El audio está dañado: {e}");
                return Err(rejected(AudioRejectCode::AudioCorrupt));
            }
        };

        let rate = decoded.spec().rate();
        let channels = decoded.spec().channels().count();
        frames += decoded.frames() as u64;
        if rate == 0 || frames * 1_000 > MAX_DURATION_MS as u64 * u64::from(rate) {
            return Err(rejected(if rate == 0 {
                AudioRejectCode::AudioCorrupt
            } else {
                AudioRejectCode::AudioTooLong
            }));
        }

        interleaved.resize(decoded.samples_interleaved(), 0.0);
        decoded.copy_to_slice_interleaved::<f32, _>(&mut interleaved);

        if output.is_none() {
            let resampler = StreamResampler::new(rate, channels).map_err(|e| {
                log::warn!("Formato de audio no admitido: {e}");
                rejected(AudioRejectCode::AudioUnsupportedFormat)
            })?;
            output = Some(Output {
                resampler,
                writer: WavWriter::create(dest)?,
                converted: Vec::new(),
                pcm: Vec::new(),
            });
        }
        if let Some(output) = output.as_mut() {
            output.push(&interleaved)?;
        }
    }

    let duration = match output {
        Some(output) => output.finish()?,
        None => 0,
    };
    if duration <= 0 {
        return Err(rejected(AudioRejectCode::AudioCorrupt));
    }
    Ok(duration)
}

/// Convierte `src` en un WAV de 16 kHz y mono en `dest` y devuelve su duración en milisegundos.
/// Si el audio se rechaza, no deja ningún archivo.
pub fn decode_to_wav(src: &Path, dest: &Path) -> Result<i64, AppError> {
    let result = decode(src, dest);
    if result.is_err() {
        if let Err(e) = std::fs::remove_file(dest) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("No se ha podido borrar el audio rechazado: {e}");
            }
        }
    }
    result
}
