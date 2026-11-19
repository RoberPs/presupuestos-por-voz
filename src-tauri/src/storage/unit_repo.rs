//! Unidades: predefinidas (semillas) y personalizadas.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::domain::rate::UnitView;

/// Predefinidas en el orden de las semillas; después las personalizadas por orden alfabético.
pub fn list(conn: &Connection) -> rusqlite::Result<Vec<UnitView>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, is_predefined FROM units
         ORDER BY is_predefined DESC,
                  CASE WHEN is_predefined = 1 THEN id END,
                  label_norm, label",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(UnitView {
            id: r.get(0)?,
            label: r.get(1)?,
            is_predefined: r.get::<_, i64>(2)? == 1,
        })
    })?;
    rows.collect()
}

pub fn find_by_norm(conn: &Connection, label_norm: &str) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM units WHERE label_norm = ?1",
        [label_norm],
        |r| r.get(0),
    )
    .optional()
}

pub fn insert_custom(tx: &Transaction<'_>, label: &str, label_norm: &str) -> rusqlite::Result<i64> {
    tx.execute(
        "INSERT INTO units (label, label_norm, is_predefined) VALUES (?1, ?2, 0)",
        params![label, label_norm],
    )?;
    Ok(tx.last_insert_rowid())
}
