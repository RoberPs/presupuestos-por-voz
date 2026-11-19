//! Formato y propuesta del número del presupuesto (FR-013, FR-013b y FR-013c).

use sha2::{Digest, Sha256};

/// Datos de una línea que identifican el contenido emitido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentLine<'a> {
    pub concept: &'a str,
    pub quantity_centi: i64,
    pub unit: &'a str,
    pub price_cents: i64,
}

/// Lo necesario para proponer un número.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proposal<'a> {
    pub manual: Option<&'a str>,
    pub issued_number: Option<&'a str>,
    pub issued_hash: Option<&'a str>,
    pub current_hash: &'a str,
    pub year: i32,
    pub last_number: i64,
}

/// `2026`, `7` → `"2026-007"`. Al menos tres cifras.
pub fn format_number(year: i32, number: i64) -> String {
    format!("{year:04}-{number:03}")
}

/// Año y número de un texto con el formato `^\d{4}-\d{3,9}$`. Otro formato no altera la serie.
pub fn parse_series_number(number: &str) -> Option<(i32, i64)> {
    let (year, sequence) = number.split_once('-')?;
    let digits = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    if year.len() != 4 || !(3..=9).contains(&sequence.len()) || !digits(year) || !digits(sequence) {
        return None;
    }
    let year: i32 = year.parse().ok()?;
    if year == 0 {
        return None;
    }
    Some((year, sequence.parse().ok()?))
}

/// Número escrito a mano, número ya emitido con el mismo contenido o siguiente de la serie.
pub fn propose(input: &Proposal<'_>) -> String {
    if let Some(manual) = input.manual {
        return manual.to_string();
    }
    match (input.issued_number, input.issued_hash) {
        (Some(number), Some(hash)) if hash == input.current_hash => number.to_string(),
        _ => format_number(input.year, input.last_number + 1),
    }
}

/// Huella de las líneas en orden y del tipo de IVA. No incluye cliente, fechas, validez ni número.
pub fn content_hash(lines: &[ContentLine<'_>], vat_rate_bp: i64) -> String {
    let mut hasher = Sha256::new();
    let mut text = |value: &str| {
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    };
    for line in lines {
        text(line.concept);
        text(&line.quantity_centi.to_string());
        text(line.unit);
        text(&line.price_cents.to_string());
    }
    text(&vat_rate_bp.to_string());
    hex::encode(hasher.finalize())
}
