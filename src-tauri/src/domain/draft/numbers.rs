//! Cantidades dictadas en cifras y en palabras, en centésimas enteras (research.md, R6).

use crate::domain::draft::units::UnitCatalog;
use crate::domain::money::{parse_quantity_centi, MAX_QUANTITY_CENTI};
use crate::domain::normalize::normalize;

/// Cantidad encontrada en `tokens[start..end]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantityMatch {
    pub start: usize,
    pub end: usize,
    pub centi: i64,
    pub unit: Option<String>,
}

/// Quita los signos de los extremos de una palabra.
pub fn clean_word(word: &str) -> &str {
    word.trim_matches(|c: char| !c.is_alphanumeric())
}

/// Palabras normalizadas de un texto.
pub fn normalized_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| normalize(clean_word(word)))
        .filter(|word| !word.is_empty())
        .collect()
}

fn units_value(word: &str) -> Option<i64> {
    Some(match word {
        "un" | "una" | "uno" => 1,
        "dos" => 2,
        "tres" => 3,
        "cuatro" => 4,
        "cinco" => 5,
        "seis" => 6,
        "siete" => 7,
        "ocho" => 8,
        "nueve" => 9,
        _ => return None,
    })
}

fn teens_value(word: &str) -> Option<i64> {
    Some(match word {
        "diez" => 10,
        "once" => 11,
        "doce" => 12,
        "trece" => 13,
        "catorce" => 14,
        "quince" => 15,
        "dieciseis" => 16,
        "diecisiete" => 17,
        "dieciocho" => 18,
        "diecinueve" => 19,
        "veintiun" | "veintiuno" | "veintiuna" => 21,
        "veintidos" => 22,
        "veintitres" => 23,
        "veinticuatro" => 24,
        "veinticinco" => 25,
        "veintiseis" => 26,
        "veintisiete" => 27,
        "veintiocho" => 28,
        "veintinueve" => 29,
        _ => return None,
    })
}

fn tens_value(word: &str) -> Option<i64> {
    Some(match word {
        "veinte" => 20,
        "treinta" => 30,
        "cuarenta" => 40,
        "cincuenta" => 50,
        "sesenta" => 60,
        "setenta" => 70,
        "ochenta" => 80,
        "noventa" => 90,
        _ => return None,
    })
}

fn hundreds_value(word: &str) -> Option<i64> {
    Some(match word {
        "cien" | "ciento" => 100,
        "doscientos" | "doscientas" => 200,
        "trescientos" | "trescientas" => 300,
        "cuatrocientos" | "cuatrocientas" => 400,
        "quinientos" | "quinientas" => 500,
        "seiscientos" | "seiscientas" => 600,
        "setecientos" | "setecientas" => 700,
        "ochocientos" | "ochocientas" => 800,
        "novecientos" | "novecientas" => 900,
        _ => return None,
    })
}

/// Número en palabras desde `start`: (valor entero, siguiente posición).
fn words_at(tokens: &[String], start: usize) -> Option<(i64, usize)> {
    let mut total = 0;
    let mut current = 0;
    // Magnitud de la última palabra: cada número solo admite palabras de orden menor.
    let mut last_magnitude = i64::MAX;
    let mut seen_thousands = false;
    let mut pos = start;

    while let Some(word) = tokens.get(pos).map(String::as_str) {
        if word == "mil" && !seen_thousands {
            total += current.max(1) * 1_000;
            current = 0;
            seen_thousands = true;
            last_magnitude = 1_000;
            pos += 1;
        } else if let Some(value) = hundreds_value(word).filter(|_| last_magnitude > 100) {
            current += value;
            last_magnitude = 100;
            pos += 1;
        } else if let Some(value) = tens_value(word).filter(|_| last_magnitude > 10) {
            current += value;
            last_magnitude = 10;
            pos += 1;
            let unit = tokens.get(pos + 1).and_then(|w| units_value(w));
            if let (Some("y"), Some(unit)) = (tokens.get(pos).map(String::as_str), unit) {
                current += unit;
                last_magnitude = 1;
                pos += 2;
            }
        } else if let Some(value) = teens_value(word).filter(|_| last_magnitude > 10) {
            current += value;
            last_magnitude = 1;
            pos += 1;
        } else if let Some(value) = units_value(word).filter(|_| last_magnitude > 1) {
            current += value;
            last_magnitude = 1;
            pos += 1;
        } else {
            break;
        }
    }

    (pos > start).then_some((total + current, pos))
}

/// Número en cifras o en palabras desde `start`: (centésimas, siguiente posición).
fn number_at(tokens: &[String], start: usize) -> Option<(i64, usize)> {
    let word = tokens.get(start)?;
    if word.starts_with(|c: char| c.is_ascii_digit()) {
        return parse_quantity_centi(word)
            .ok()
            .map(|centi| (centi, start + 1));
    }
    let (value, next) = words_at(tokens, start)?;
    Some((value.checked_mul(100)?, next))
}

fn fraction(word: Option<&String>) -> Option<i64> {
    match word.map(String::as_str) {
        Some("medio" | "media") => Some(50),
        Some("cuarto") => Some(25),
        _ => None,
    }
}

fn is_word(tokens: &[String], pos: usize, word: &str) -> bool {
    tokens.get(pos).is_some_and(|t| t == word)
}

/// Completa la cantidad con la unidad y con «y medio» o «y cuarto», si los hay.
fn finish(
    tokens: &[String],
    start: usize,
    mut end: usize,
    mut centi: i64,
    units: &UnitCatalog,
) -> QuantityMatch {
    let mut unit = None;
    if let Some((len, canonical)) = units.match_at(tokens, end) {
        unit = Some(canonical);
        end += len;
    }
    if is_word(tokens, end, "y") {
        if let Some(extra) = fraction(tokens.get(end + 1)) {
            centi += extra;
            end += 2;
        }
    }
    QuantityMatch {
        start,
        end,
        centi,
        unit,
    }
}

/// Cantidad que empieza exactamente en `start`.
fn match_at(tokens: &[String], start: usize, units: &UnitCatalog) -> Option<QuantityMatch> {
    let word = tokens.get(start)?.as_str();

    // «un par de horas», «par de horas»
    let par_at = if matches!(word, "un" | "una") {
        start + 1
    } else {
        start
    };
    if is_word(tokens, par_at, "par") && is_word(tokens, par_at + 1, "de") {
        return Some(finish(tokens, start, par_at + 2, 200, units));
    }

    // «media hora», «medio metro»
    if matches!(word, "media" | "medio") {
        if let Some((len, canonical)) = units.match_at(tokens, start + 1) {
            return Some(QuantityMatch {
                start,
                end: start + 1 + len,
                centi: 50,
                unit: Some(canonical),
            });
        }
    }

    // «cuarto de hora»
    if word == "cuarto" && is_word(tokens, start + 1, "de") {
        if let Some((len, canonical)) = units.match_at(tokens, start + 2) {
            return Some(QuantityMatch {
                start,
                end: start + 2 + len,
                centi: 25,
                unit: Some(canonical),
            });
        }
    }

    // «metro y medio», «hora y cuarto»
    if let Some((len, canonical)) = units.match_at(tokens, start) {
        if is_word(tokens, start + len, "y") {
            if let Some(extra) = fraction(tokens.get(start + len + 1)) {
                return Some(QuantityMatch {
                    start,
                    end: start + len + 2,
                    centi: 100 + extra,
                    unit: Some(canonical),
                });
            }
        }
    }

    let (centi, next) = number_at(tokens, start)?;

    // «un», «una» y «uno» suelen ser artículos: solo cuentan al empezar o delante de una unidad.
    if matches!(word, "un" | "una" | "uno")
        && next == start + 1
        && start != 0
        && units.match_at(tokens, next).is_none()
    {
        return None;
    }

    // «tres cuartos de hora»
    if is_word(tokens, next, "cuartos") && is_word(tokens, next + 1, "de") {
        if let Some((len, canonical)) = units.match_at(tokens, next + 2) {
            return Some(QuantityMatch {
                start,
                end: next + 2 + len,
                centi: centi / 4,
                unit: Some(canonical),
            });
        }
    }

    // «dos y medio»
    if is_word(tokens, next, "y") {
        if let Some(extra) = fraction(tokens.get(next + 1)) {
            return Some(finish(tokens, start, next + 2, centi + extra, units));
        }
    }

    Some(finish(tokens, start, next, centi, units))
}

/// Primera cantidad válida a partir de `from`.
pub fn find_quantity(tokens: &[String], from: usize, units: &UnitCatalog) -> Option<QuantityMatch> {
    (from..tokens.len())
        .filter_map(|start| match_at(tokens, start, units))
        .find(|m| (1..=MAX_QUANTITY_CENTI).contains(&m.centi))
}

/// Primera cantidad de un texto, en centésimas.
pub fn parse_quantity_text(text: &str, units: &UnitCatalog) -> Option<i64> {
    find_quantity(&normalized_tokens(text), 0, units).map(|m| m.centi)
}
