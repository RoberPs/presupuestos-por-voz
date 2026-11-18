//! Dictados en SQLite: ranura `current` (audio del borrador vigente) e `incoming` (audio nuevo).

use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;

use crate::error::TranscriptionCode;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Current,
    Incoming,
}

impl Slot {
    pub fn as_db(self) -> &'static str {
        match self {
            Slot::Current => "current",
            Slot::Incoming => "incoming",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DictationOrigin {
    Recorded,
    Imported,
}

impl DictationOrigin {
    pub fn as_db(self) -> &'static str {
        match self {
            DictationOrigin::Recorded => "recorded",
            DictationOrigin::Imported => "imported",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "recorded" => Some(DictationOrigin::Recorded),
            "imported" => Some(DictationOrigin::Imported),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DictationStatus {
    Recorded,
    Processing,
    Transcribed,
    Failed,
    NoSpeech,
}

impl DictationStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            DictationStatus::Recorded => "recorded",
            DictationStatus::Processing => "processing",
            DictationStatus::Transcribed => "transcribed",
            DictationStatus::Failed => "failed",
            DictationStatus::NoSpeech => "no_speech",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "recorded" => Some(DictationStatus::Recorded),
            "processing" => Some(DictationStatus::Processing),
            "transcribed" => Some(DictationStatus::Transcribed),
            "failed" => Some(DictationStatus::Failed),
            "no_speech" => Some(DictationStatus::NoSpeech),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dictation {
    pub audio_file: String,
    pub duration_ms: i64,
    pub origin: DictationOrigin,
    pub original_file_name: Option<String>,
    pub status: DictationStatus,
    pub low_quality: bool,
    pub error_code: Option<TranscriptionCode>,
}

fn invalid(column: usize, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        Type::Text,
        format!("valor no válido: {value}").into(),
    )
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<Dictation> {
    let origin: String = r.get(2)?;
    let status: String = r.get(4)?;
    let error_code = r
        .get::<_, Option<String>>(6)?
        .map(|v| TranscriptionCode::from_db(&v).ok_or_else(|| invalid(6, &v)))
        .transpose()?;
    Ok(Dictation {
        audio_file: r.get(0)?,
        duration_ms: r.get(1)?,
        origin: DictationOrigin::from_db(&origin).ok_or_else(|| invalid(2, &origin))?,
        original_file_name: r.get(3)?,
        status: DictationStatus::from_db(&status).ok_or_else(|| invalid(4, &status))?,
        low_quality: r.get::<_, i64>(5)? == 1,
        error_code,
    })
}

pub fn get(conn: &Connection, slot: Slot) -> rusqlite::Result<Option<Dictation>> {
    conn.query_row(
        "SELECT audio_file, duration_ms, origin, original_file_name, status, low_quality, error_code
         FROM dictations WHERE slot = ?1",
        [slot.as_db()],
        from_row,
    )
    .optional()
}

/// Crea o sustituye la fila de una ranura.
pub fn put(conn: &Connection, slot: Slot, d: &Dictation) -> rusqlite::Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO dictations (slot, audio_file, duration_ms, origin, original_file_name,
                                     status, low_quality, error_code)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(slot) DO UPDATE SET
                audio_file = excluded.audio_file,
                duration_ms = excluded.duration_ms,
                origin = excluded.origin,
                original_file_name = excluded.original_file_name,
                status = excluded.status,
                low_quality = excluded.low_quality,
                error_code = excluded.error_code,
                updated_at = {NOW}"
        ),
        params![
            slot.as_db(),
            d.audio_file,
            d.duration_ms,
            d.origin.as_db(),
            d.original_file_name,
            d.status.as_db(),
            i64::from(d.low_quality),
            d.error_code.map(TranscriptionCode::as_db),
        ],
    )?;
    Ok(())
}

pub fn set_status(
    conn: &Connection,
    slot: Slot,
    status: DictationStatus,
    error_code: Option<TranscriptionCode>,
) -> rusqlite::Result<usize> {
    conn.execute(
        &format!(
            "UPDATE dictations SET status = ?2, error_code = ?3, updated_at = {NOW} WHERE slot = ?1"
        ),
        params![
            slot.as_db(),
            status.as_db(),
            error_code.map(TranscriptionCode::as_db)
        ],
    )
}

pub fn set_low_quality(
    conn: &Connection,
    slot: Slot,
    low_quality: bool,
) -> rusqlite::Result<usize> {
    conn.execute(
        &format!("UPDATE dictations SET low_quality = ?2, updated_at = {NOW} WHERE slot = ?1"),
        params![slot.as_db(), i64::from(low_quality)],
    )
}

pub fn delete(conn: &Connection, slot: Slot) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM dictations WHERE slot = ?1", [slot.as_db()])
}

/// La ranura `incoming` pasa a ser `current`, sustituyendo a la anterior.
pub fn promote_incoming(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM dictations WHERE slot = 'current'", [])?;
    conn.execute(
        &format!(
            "UPDATE dictations SET slot = 'current', updated_at = {NOW} WHERE slot = 'incoming'"
        ),
        [],
    )
}

/// Al arrancar, un proceso interrumpido vuelve a `recorded`.
pub fn reset_processing(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        &format!("UPDATE dictations SET status = 'recorded', updated_at = {NOW} WHERE status = 'processing'"),
        [],
    )
}

/// Audios referenciados por alguna ranura.
pub fn audio_files(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT audio_file FROM dictations")?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    rows.collect()
}
