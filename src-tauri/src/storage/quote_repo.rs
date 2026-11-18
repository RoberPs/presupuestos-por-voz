//! Datos del presupuesto en curso (`quote_details`) y numeración por año (`quote_number_sequences`).

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::quote_doc::QuoteDetails;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

/// Datos guardados del borrador en curso, si existen.
pub fn get_details(conn: &Connection) -> rusqlite::Result<Option<QuoteDetails>> {
    conn.query_row(
        "SELECT client_name, client_address, client_phone, client_email, client_tax_id,
                quote_number, issued_number, issued_content_hash, validity_days
         FROM quote_details WHERE draft_id = 1",
        [],
        |r| {
            Ok(QuoteDetails {
                client_name: r.get(0)?,
                client_address: r.get(1)?,
                client_phone: r.get(2)?,
                client_email: r.get(3)?,
                client_tax_id: r.get(4)?,
                quote_number: r.get(5)?,
                issued_number: r.get(6)?,
                issued_content_hash: r.get(7)?,
                validity_days: r.get(8)?,
            })
        },
    )
    .optional()
}

/// Crea o sustituye los datos del borrador en curso. El borrador debe existir.
pub fn save_details(conn: &Connection, details: &QuoteDetails) -> rusqlite::Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO quote_details (
                draft_id, client_name, client_address, client_phone, client_email, client_tax_id,
                quote_number, issued_number, issued_content_hash, validity_days
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(draft_id) DO UPDATE SET
                client_name = excluded.client_name,
                client_address = excluded.client_address,
                client_phone = excluded.client_phone,
                client_email = excluded.client_email,
                client_tax_id = excluded.client_tax_id,
                quote_number = excluded.quote_number,
                issued_number = excluded.issued_number,
                issued_content_hash = excluded.issued_content_hash,
                validity_days = excluded.validity_days,
                updated_at = {NOW}"
        ),
        params![
            details.client_name,
            details.client_address,
            details.client_phone,
            details.client_email,
            details.client_tax_id,
            details.quote_number,
            details.issued_number,
            details.issued_content_hash,
            details.validity_days,
        ],
    )?;
    Ok(())
}

/// Último número usado en un año; 0 si aún no se ha usado ninguno.
pub fn last_number(conn: &Connection, year: i32) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT last_number FROM quote_number_sequences WHERE year = ?1",
        [year],
        |r| r.get(0),
    )
    .optional()
    .map(|value| value.unwrap_or(0))
}

/// Actualiza la serie de un año con `MAX(last_number, usado)` (FR-013c).
pub fn record_number(conn: &Connection, year: i32, used: i64) -> rusqlite::Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO quote_number_sequences (year, last_number) VALUES (?1, ?2)
             ON CONFLICT(year) DO UPDATE SET
                last_number = MAX(last_number, excluded.last_number),
                updated_at = {NOW}"
        ),
        params![year, used],
    )?;
    Ok(())
}
