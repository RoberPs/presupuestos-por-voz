//! Borrador de presupuesto en SQLite: fila única `quote_draft`, líneas, sugerencias y fragmentos.
//!
//! Las operaciones que tocan varias tablas se ejecutan dentro de la transacción del servicio.

use std::collections::HashMap;

use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::draft::{
    BuiltDraft, BuiltLine, DraftLine, DraftStatus, PendingReason, PriceSource, QuantityIssue,
    StoredDraft, TextOrigin,
};

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
const LINE_COLUMNS: &str = "id, position, concept, quantity_centi, quantity_issue, unit_label,
                            price_cents, price_source, rate_id, rate_concept, pending_reason,
                            source_fragment";

fn invalid(column: usize, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        Type::Text,
        format!("valor no válido: {value}").into(),
    )
}

fn line_from_row(r: &Row<'_>) -> rusqlite::Result<DraftLine> {
    let quantity_issue = r
        .get::<_, Option<String>>(4)?
        .map(|v| QuantityIssue::from_db(&v).ok_or_else(|| invalid(4, &v)))
        .transpose()?;
    let source: String = r.get(7)?;
    let pending_reason = r
        .get::<_, Option<String>>(10)?
        .map(|v| PendingReason::from_db(&v).ok_or_else(|| invalid(10, &v)))
        .transpose()?;
    Ok(DraftLine {
        id: r.get(0)?,
        position: r.get(1)?,
        concept: r.get(2)?,
        quantity_centi: r.get(3)?,
        quantity_issue,
        unit_label: r.get(5)?,
        price_cents: r.get(6)?,
        price_source: PriceSource::from_db(&source).ok_or_else(|| invalid(7, &source))?,
        rate_id: r.get(8)?,
        rate_concept: r.get(9)?,
        pending_reason,
        source_fragment: r.get(11)?,
    })
}

/// Tarifas sugeridas de todas las líneas, en su orden.
fn all_suggestions(conn: &Connection) -> rusqlite::Result<HashMap<i64, Vec<i64>>> {
    let mut stmt = conn.prepare_cached(
        "SELECT line_id, rate_id FROM draft_line_suggestions ORDER BY line_id, rank",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
    let mut map: HashMap<i64, Vec<i64>> = HashMap::new();
    for row in rows {
        let (line_id, rate_id) = row?;
        map.entry(line_id).or_default().push(rate_id);
    }
    Ok(map)
}

fn load_lines(conn: &Connection) -> rusqlite::Result<Vec<BuiltLine>> {
    let mut suggestions = all_suggestions(conn)?;
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT {LINE_COLUMNS} FROM draft_lines WHERE draft_id = 1 ORDER BY position"
    ))?;
    let rows = stmt.query_map([], line_from_row)?;
    rows.map(|row| {
        let line = row?;
        let suggestion_rate_ids = suggestions.remove(&line.id).unwrap_or_default();
        Ok(BuiltLine {
            line,
            suggestion_rate_ids,
        })
    })
    .collect()
}

fn load_fragments(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT text FROM draft_unparsed_fragments WHERE draft_id = 1 ORDER BY position, id",
    )?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    rows.collect()
}

/// Lee el borrador completo, si existe.
pub fn load(conn: &Connection) -> rusqlite::Result<Option<StoredDraft>> {
    let header = conn
        .query_row(
            "SELECT status, source_text, text_origin, text_corrected, vat_rate_bp, has_manual_changes
             FROM quote_draft WHERE id = 1",
            [],
            |r| {
                let status: String = r.get(0)?;
                let origin: String = r.get(2)?;
                Ok((
                    DraftStatus::from_db(&status).ok_or_else(|| invalid(0, &status))?,
                    r.get::<_, String>(1)?,
                    TextOrigin::from_db(&origin).ok_or_else(|| invalid(2, &origin))?,
                    r.get::<_, i64>(3)? == 1,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)? == 1,
                ))
            },
        )
        .optional()?;

    let Some((status, source_text, text_origin, text_corrected, vat_rate_bp, has_manual_changes)) =
        header
    else {
        return Ok(None);
    };

    Ok(Some(StoredDraft {
        status,
        source_text,
        text_origin,
        text_corrected,
        vat_rate_bp,
        has_manual_changes,
        lines: load_lines(conn)?,
        unparsed_fragments: load_fragments(conn)?,
    }))
}

pub fn exists(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM quote_draft WHERE id = 1)",
        [],
        |r| r.get::<_, i64>(0).map(|v| v == 1),
    )
}

/// Sustituye el borrador por uno nuevo y vacía el texto pendiente. Debe llamarse en una transacción.
pub fn replace(
    conn: &Connection,
    built: &BuiltDraft,
    source_text: &str,
    text_origin: TextOrigin,
) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM quote_draft WHERE id = 1", [])?;
    conn.execute(
        "INSERT INTO quote_draft (id, status, source_text, text_origin, vat_rate_bp)
         VALUES (1, 'draft', ?1, ?2, ?3)",
        params![source_text, text_origin.as_db(), built.vat_rate_bp],
    )?;
    for built_line in &built.lines {
        insert_line(conn, &built_line.line, &built_line.suggestion_rate_ids)?;
    }
    for (position, text) in built.unparsed_fragments.iter().enumerate() {
        conn.execute(
            "INSERT INTO draft_unparsed_fragments (draft_id, position, text) VALUES (1, ?1, ?2)",
            params![position as i64, text],
        )?;
    }
    clear_pending_text(conn)
}

/// Inserta una línea en su `position` y devuelve su identificador.
pub fn insert_line(
    conn: &Connection,
    line: &DraftLine,
    suggestion_rate_ids: &[i64],
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO draft_lines (draft_id, position, concept, quantity_centi, quantity_issue,
                                  unit_label, price_cents, price_source, rate_id, rate_concept,
                                  pending_reason, source_fragment)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            line.position,
            line.concept,
            line.quantity_centi,
            line.quantity_issue.map(QuantityIssue::as_db),
            line.unit_label,
            line.price_cents,
            line.price_source.as_db(),
            line.rate_id,
            line.rate_concept,
            line.pending_reason.map(PendingReason::as_db),
            line.source_fragment,
        ],
    )?;
    let id = conn.last_insert_rowid();
    replace_suggestions(conn, id, suggestion_rate_ids)?;
    Ok(id)
}

pub fn get_line(conn: &Connection, id: i64) -> rusqlite::Result<Option<BuiltLine>> {
    let line = conn
        .query_row(
            &format!("SELECT {LINE_COLUMNS} FROM draft_lines WHERE id = ?1 AND draft_id = 1"),
            [id],
            line_from_row,
        )
        .optional()?;
    let Some(line) = line else {
        return Ok(None);
    };
    let mut stmt = conn.prepare_cached(
        "SELECT rate_id FROM draft_line_suggestions WHERE line_id = ?1 ORDER BY rank",
    )?;
    let suggestion_rate_ids = stmt
        .query_map([id], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<i64>>>()?;
    Ok(Some(BuiltLine {
        line,
        suggestion_rate_ids,
    }))
}

/// Guarda los campos editables de una línea. Devuelve las filas afectadas.
pub fn update_line(conn: &Connection, line: &DraftLine) -> rusqlite::Result<usize> {
    conn.execute(
        &format!(
            "UPDATE draft_lines SET
                concept = ?2, quantity_centi = ?3, quantity_issue = ?4, unit_label = ?5,
                price_cents = ?6, price_source = ?7, rate_id = ?8, rate_concept = ?9,
                pending_reason = ?10, updated_at = {NOW}
             WHERE id = ?1 AND draft_id = 1"
        ),
        params![
            line.id,
            line.concept,
            line.quantity_centi,
            line.quantity_issue.map(QuantityIssue::as_db),
            line.unit_label,
            line.price_cents,
            line.price_source.as_db(),
            line.rate_id,
            line.rate_concept,
            line.pending_reason.map(PendingReason::as_db),
        ],
    )
}

/// Sustituye las sugerencias de una línea (como máximo 5, en orden).
pub fn replace_suggestions(
    conn: &Connection,
    line_id: i64,
    rate_ids: &[i64],
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM draft_line_suggestions WHERE line_id = ?1",
        [line_id],
    )?;
    for (rank, rate_id) in rate_ids.iter().take(5).enumerate() {
        conn.execute(
            "INSERT INTO draft_line_suggestions (line_id, rate_id, rank) VALUES (?1, ?2, ?3)",
            params![line_id, rate_id, rank as i64],
        )?;
    }
    Ok(())
}

pub fn delete_line(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM draft_lines WHERE id = ?1 AND draft_id = 1",
        [id],
    )
}

/// Posición para una línea añadida al final.
pub fn next_position(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM draft_lines WHERE draft_id = 1",
        [],
        |r| r.get(0),
    )
}

/// Tras una edición a mano: hay cambios manuales y un borrador revisado vuelve a borrador (FR-034).
pub fn mark_edited(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        &format!(
            "UPDATE quote_draft SET has_manual_changes = 1, status = 'draft', updated_at = {NOW}
             WHERE id = 1"
        ),
        [],
    )
}

pub fn set_status(conn: &Connection, status: DraftStatus) -> rusqlite::Result<usize> {
    conn.execute(
        &format!("UPDATE quote_draft SET status = ?1, updated_at = {NOW} WHERE id = 1"),
        [status.as_db()],
    )
}

pub fn set_vat_rate(conn: &Connection, vat_rate_bp: i64) -> rusqlite::Result<usize> {
    conn.execute(
        &format!("UPDATE quote_draft SET vat_rate_bp = ?1, updated_at = {NOW} WHERE id = 1"),
        [vat_rate_bp],
    )
}

/// Guarda el texto corregido sin regenerar las líneas.
pub fn set_source_text(conn: &Connection, text: &str, corrected: bool) -> rusqlite::Result<usize> {
    conn.execute(
        &format!(
            "UPDATE quote_draft SET source_text = ?1, text_corrected = ?2, updated_at = {NOW}
             WHERE id = 1"
        ),
        params![text, i64::from(corrected)],
    )
}

pub fn delete(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM quote_draft WHERE id = 1", [])
}

/// Texto escrito que aún no tiene borrador (FR-035).
pub fn pending_text(conn: &Connection) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT text FROM visit_text_draft WHERE id = 1", [], |r| {
        r.get(0)
    })
    .optional()
}

pub fn save_pending_text(conn: &Connection, text: &str) -> rusqlite::Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO visit_text_draft (id, text) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET text = excluded.text, updated_at = {NOW}"
        ),
        [text],
    )?;
    Ok(())
}

pub fn clear_pending_text(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM visit_text_draft WHERE id = 1", [])?;
    Ok(())
}
