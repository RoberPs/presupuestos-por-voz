//! Tarifas en SQLite.

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::domain::rate::{RateView, UnitView};

const SELECT: &str = "SELECT r.id, r.concept, r.price_cents, u.id, u.label, u.is_predefined
                      FROM rates r JOIN units u ON u.id = r.unit_id";
const ORDER: &str = "ORDER BY r.concept_norm, r.concept";

fn from_row(r: &Row<'_>) -> rusqlite::Result<RateView> {
    Ok(RateView::new(
        r.get(0)?,
        r.get(1)?,
        UnitView { id: r.get(3)?, label: r.get(4)?, is_predefined: r.get::<_, i64>(5)? == 1 },
        r.get(2)?,
    ))
}

pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<RateView>> {
    let mut stmt = conn.prepare_cached(&format!("{SELECT} {ORDER}"))?;
    let rows = stmt.query_map([], from_row)?;
    rows.collect()
}

/// Tarifas cuyo `concept_norm` contiene `norm_query` (ya normalizada y escapada para LIKE).
pub fn search(conn: &Connection, norm_query: &str) -> rusqlite::Result<Vec<RateView>> {
    let mut stmt = conn.prepare_cached(&format!(
        "{SELECT} WHERE r.concept_norm LIKE '%' || ?1 || '%' ESCAPE '\\' {ORDER}"
    ))?;
    let rows = stmt.query_map([norm_query], from_row)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<RateView>> {
    conn.query_row(&format!("{SELECT} WHERE r.id = ?1"), [id], from_row).optional()
}

pub fn find_duplicate(
    conn: &Connection,
    concept_norm: &str,
    unit_id: i64,
    exclude_id: Option<i64>,
) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM rates WHERE concept_norm = ?1 AND unit_id = ?2 AND (?3 IS NULL OR id <> ?3)",
        params![concept_norm, unit_id, exclude_id],
        |r| r.get(0),
    )
    .optional()
}

pub fn insert(
    tx: &Transaction<'_>,
    concept: &str,
    concept_norm: &str,
    unit_id: i64,
    price_cents: i64,
) -> rusqlite::Result<i64> {
    tx.execute(
        "INSERT INTO rates (concept, concept_norm, unit_id, price_cents) VALUES (?1, ?2, ?3, ?4)",
        params![concept, concept_norm, unit_id, price_cents],
    )?;
    Ok(tx.last_insert_rowid())
}

/// Devuelve las filas afectadas (0 si el `id` no existe).
pub fn update(
    tx: &Transaction<'_>,
    id: i64,
    concept: &str,
    concept_norm: &str,
    unit_id: i64,
    price_cents: i64,
) -> rusqlite::Result<usize> {
    tx.execute(
        "UPDATE rates SET concept = ?2, concept_norm = ?3, unit_id = ?4, price_cents = ?5,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1",
        params![id, concept, concept_norm, unit_id, price_cents],
    )
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM rates WHERE id = ?1", [id])
}
