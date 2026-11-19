//! Audios del dictado en `app_data_dir/recordings/` (research.md, R5).

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppError;

pub const RECORDINGS_DIR: &str = "recordings";
const PART_FILE: &str = "incoming.wav.part";

pub fn recordings_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(RECORDINGS_DIR)
}

/// Archivo temporal de la grabación en curso o interrumpida.
pub fn part_path(data_dir: &Path) -> PathBuf {
    recordings_dir(data_dir).join(PART_FILE)
}

pub fn ensure_dir(data_dir: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(recordings_dir(data_dir))?;
    Ok(())
}

/// Ruta absoluta de un audio guardado con ruta relativa en `dictations.audio_file`.
pub fn absolute(data_dir: &Path, audio_file: &str) -> PathBuf {
    data_dir.join(audio_file)
}

/// Reserva un nombre nuevo, `recordings/dictation-<ms>.wav`, que no exista todavía.
pub fn new_audio_file(data_dir: &Path) -> String {
    let mut stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    loop {
        let relative = format!("{RECORDINGS_DIR}/dictation-{stamp}.wav");
        if !absolute(data_dir, &relative).exists() {
            return relative;
        }
        stamp += 1;
    }
}

/// Convierte la grabación temporal en un audio definitivo, de forma atómica.
pub fn promote_part(data_dir: &Path, audio_file: &str) -> Result<(), AppError> {
    std::fs::rename(part_path(data_dir), absolute(data_dir, audio_file))?;
    Ok(())
}

/// Borra un archivo; que ya no exista no es un error.
fn remove_if_exists(path: &Path) -> Result<(), AppError> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

pub fn remove_audio(data_dir: &Path, audio_file: &str) -> Result<(), AppError> {
    remove_if_exists(&absolute(data_dir, audio_file))
}

pub fn remove_part(data_dir: &Path) -> Result<(), AppError> {
    remove_if_exists(&part_path(data_dir))
}

/// Borra los `.wav` que ninguna ranura referencia. La grabación temporal no se toca, porque se
/// ofrece recuperarla. Devuelve cuántos archivos se han borrado.
pub fn cleanup_orphans(data_dir: &Path, referenced: &[String]) -> Result<usize, AppError> {
    let dir = recordings_dir(data_dir);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.into()),
    };

    let mut removed = 0;
    for entry in entries {
        let path = entry?.path();
        let is_wav = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let relative = format!("{RECORDINGS_DIR}/{name}");
        if is_wav && !referenced.contains(&relative) {
            remove_if_exists(&path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_keeps_referenced_audio_and_the_partial_recording() {
        let dir = tempfile::tempdir().unwrap();
        ensure_dir(dir.path()).unwrap();
        let kept = "recordings/dictation-1.wav".to_string();
        std::fs::write(absolute(dir.path(), &kept), b"a").unwrap();
        std::fs::write(absolute(dir.path(), "recordings/dictation-2.wav"), b"b").unwrap();
        std::fs::write(part_path(dir.path()), b"c").unwrap();

        let removed = cleanup_orphans(dir.path(), std::slice::from_ref(&kept)).unwrap();

        assert_eq!(removed, 1);
        assert!(absolute(dir.path(), &kept).exists());
        assert!(!absolute(dir.path(), "recordings/dictation-2.wav").exists());
        assert!(part_path(dir.path()).exists());
    }

    #[test]
    fn cleanup_without_a_folder_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(cleanup_orphans(dir.path(), &[]).unwrap(), 0);
    }

    #[test]
    fn promoting_moves_the_partial_recording() {
        let dir = tempfile::tempdir().unwrap();
        ensure_dir(dir.path()).unwrap();
        std::fs::write(part_path(dir.path()), b"audio").unwrap();
        let audio_file = new_audio_file(dir.path());

        promote_part(dir.path(), &audio_file).unwrap();

        assert!(!part_path(dir.path()).exists());
        assert_eq!(
            std::fs::read(absolute(dir.path(), &audio_file)).unwrap(),
            b"audio"
        );
    }
}
