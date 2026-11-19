//! Audio: escritura incremental, reparación, silencio e importación (FR-002b, FR-002c y FR-005).

use std::path::{Path, PathBuf};

use app_lib::audio::decode::decode_to_wav;
use app_lib::audio::quality::has_speech;
use app_lib::audio::wav::{self, WavWriter, MAX_SAMPLES};
use app_lib::audio::{MAX_DURATION_MS, SAMPLE_RATE};
use app_lib::error::{AppError, AudioRejectCode};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/audio")
        .join(name)
}

fn tone(samples: usize) -> Vec<i16> {
    (0..samples)
        .map(|i| ((i as f32 * 0.17).sin() * 9_000.0) as i16)
        .collect()
}

fn rejected_code(result: Result<i64, AppError>) -> AudioRejectCode {
    match result {
        Err(AppError::AudioRejected { code, .. }) => code,
        other => panic!("se esperaba un rechazo y se obtuvo {other:?}"),
    }
}

/// WAV PCM de 16 bits con cualquier frecuencia, para simular archivos importados.
fn write_raw_wav(path: &Path, rate: u32, seconds: u32) {
    let data_len = rate * seconds * 2;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.resize(44 + data_len as usize, 0);
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn repairing_a_truncated_recording_keeps_its_samples() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("incoming.wav.part");
    let samples = tone(16_000);

    let mut writer = WavWriter::create(&path).unwrap();
    writer.write(&samples).unwrap();
    writer.flush().unwrap();
    drop(writer); // cierre inesperado: la cabecera no se ha completado

    // Un corte a mitad de una muestra deja un byte suelto.
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.push(7);
    std::fs::write(&path, bytes).unwrap();

    assert_eq!(wav::repair(&path).unwrap(), 1_000);

    let read = wav::read_samples(&path).unwrap();
    assert_eq!(read.len(), samples.len());
    let expected = f32::from(samples[100]) / 32_768.0;
    assert!((read[100] - expected).abs() < 0.0001);
}

#[test]
fn repairing_a_file_without_audio_gives_zero() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("incoming.wav.part");
    std::fs::write(&path, b"RIFF").unwrap();
    assert_eq!(wav::repair(&path).unwrap(), 0);
}

#[test]
fn a_finished_recording_can_be_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audio.wav");
    let mut writer = WavWriter::create(&path).unwrap();
    writer.write(&tone(8_000)).unwrap();
    assert_eq!(writer.finalize().unwrap(), 500);
    assert_eq!(wav::read_samples(&path).unwrap().len(), 8_000);
}

#[test]
fn a_recording_past_the_limit_is_cut_at_ten_minutes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long.wav");
    let second = vec![0i16; SAMPLE_RATE as usize];

    let mut writer = WavWriter::create(&path).unwrap();
    for _ in 0..601 {
        writer.write(&second).unwrap();
    }
    assert!(writer.is_full());
    assert_eq!(writer.finalize().unwrap(), MAX_DURATION_MS);
    assert_eq!(
        std::fs::metadata(&path).unwrap().len(),
        44 + MAX_SAMPLES * 2
    );
}

#[test]
fn silence_is_not_speech() {
    assert!(!has_speech(&vec![0.0; 16_000]));
    assert!(!has_speech(&[]));
}

#[test]
fn a_very_short_sound_is_not_speech() {
    let samples: Vec<f32> = tone(1_600)
        .iter()
        .map(|s| f32::from(*s) / 32_768.0)
        .collect();
    assert!(!has_speech(&samples));
}

#[test]
fn a_sustained_sound_counts_as_speech() {
    let samples: Vec<f32> = tone(16_000)
        .iter()
        .map(|s| f32::from(*s) / 32_768.0)
        .collect();
    assert!(has_speech(&samples));
}

#[test]
fn wav_mp3_and_m4a_are_decoded_to_16k_mono() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["tone-3s.wav", "tone-3s.mp3", "tone-3s.m4a"] {
        let dest = dir.path().join(format!("{name}.out.wav"));
        let duration = decode_to_wav(&fixture(name), &dest).unwrap();
        assert!(
            (2_800..=3_300).contains(&duration),
            "{name}: duración {duration} ms"
        );
        let samples = wav::read_samples(&dest).unwrap();
        let expected = (duration * i64::from(SAMPLE_RATE) / 1_000) as usize;
        assert!(samples.len().abs_diff(expected) <= 16, "{name}");
        assert!(has_speech(&samples), "{name}: el audio debería conservarse");
    }
}

#[test]
fn a_text_renamed_to_mp3_is_not_supported() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("notas.mp3");
    std::fs::write(&src, "esto es un texto y no un audio ".repeat(200)).unwrap();
    let dest = dir.path().join("out.wav");
    assert_eq!(
        rejected_code(decode_to_wav(&src, &dest)),
        AudioRejectCode::AudioUnsupportedFormat
    );
    assert!(!dest.exists());
}

#[test]
fn a_truncated_file_is_reported_as_damaged() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = std::fs::read(fixture("tone-3s.m4a")).unwrap();
    let src = dir.path().join("cortado.m4a");
    std::fs::write(&src, &bytes[..bytes.len() / 3]).unwrap();
    let dest = dir.path().join("out.wav");
    assert_eq!(
        rejected_code(decode_to_wav(&src, &dest)),
        AudioRejectCode::AudioCorrupt
    );
    assert!(!dest.exists());
}

#[test]
fn an_audio_over_ten_minutes_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("largo.wav");
    write_raw_wav(&src, 8_000, 601);
    let dest = dir.path().join("out.wav");
    assert_eq!(
        rejected_code(decode_to_wav(&src, &dest)),
        AudioRejectCode::AudioTooLong
    );
    assert!(!dest.exists());
}

#[test]
fn an_audio_of_exactly_ten_minutes_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("limite.wav");
    write_raw_wav(&src, 8_000, 600);
    let dest = dir.path().join("out.wav");
    let duration = decode_to_wav(&src, &dest).unwrap();
    assert!((599_900..=MAX_DURATION_MS).contains(&duration));
}
