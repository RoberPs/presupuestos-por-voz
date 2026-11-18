//! Fecha de emisión y fecha de validez (FR-014 y FR-014b).
//!
//! El dominio recibe la fecha como parámetro: solo el servicio consulta el reloj (R12).

use time::{Date, Duration};

/// Fecha de emisión más los días de validez.
pub fn valid_until(issue_date: Date, validity_days: i64) -> Date {
    issue_date.saturating_add(Duration::days(validity_days))
}

/// `18/12/2026`.
pub fn format_date(date: Date) -> String {
    format!(
        "{:02}/{:02}/{:04}",
        date.day(),
        u8::from(date.month()),
        date.year()
    )
}

/// `2026-12-18`, para el nombre del archivo.
pub fn format_iso(date: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}
