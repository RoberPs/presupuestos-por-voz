//! Dinero en céntimos enteros y porcentajes en puntos básicos (R4).
//!
//! Reglas de entrada (data-model.md): la coma es el separador decimal. El punto también lo es si
//! aparece una sola vez y le siguen 1 o 2 dígitos al final; en otro caso es separador de miles.

use crate::error::ErrorCode;

#[derive(Debug, PartialEq, Eq)]
enum DecimalError {
    Empty,
    Format,
    TooManyDecimals,
}

/// Máximo de dígitos enteros aceptados (evita desbordamientos; 999.999.999.999 €).
const MAX_INT_DIGITS: usize = 12;

/// Convierte un texto es-ES en centésimas (con signo).
fn parse_hundredths(input: &str) -> Result<i64, DecimalError> {
    let s: String = input
        .trim()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if s.is_empty() {
        return Err(DecimalError::Empty);
    }
    let (negative, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(&s)),
    };
    if body.is_empty()
        || !body
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '.')
    {
        return Err(DecimalError::Format);
    }

    let commas = body.matches(',').count();
    let (int_part, dec_part): (&str, Option<&str>) = match commas {
        0 => {
            let dots = body.matches('.').count();
            match body.rfind('.') {
                Some(pos) if dots == 1 => {
                    let after = &body[pos + 1..];
                    if (1..=2).contains(&after.len()) {
                        (&body[..pos], Some(after))
                    } else {
                        (body, None)
                    }
                }
                _ => (body, None),
            }
        }
        1 => {
            let pos = body.find(',').unwrap_or(0);
            let dec = &body[pos + 1..];
            if dec.contains('.') {
                return Err(DecimalError::Format);
            }
            (&body[..pos], Some(dec))
        }
        _ => return Err(DecimalError::Format),
    };

    let int_digits = parse_grouped_int(int_part)?;
    let cents_part = match dec_part {
        None => 0,
        Some("") => return Err(DecimalError::Format),
        Some(d) if d.len() > 2 => return Err(DecimalError::TooManyDecimals),
        Some(d) => {
            let v: i64 = d.parse().map_err(|_| DecimalError::Format)?;
            if d.len() == 1 {
                v * 10
            } else {
                v
            }
        }
    };

    let value = int_digits * 100 + cents_part;
    Ok(if negative { -value } else { value })
}

/// Parte entera, opcionalmente con puntos de miles bien agrupados ("1.234.567").
fn parse_grouped_int(s: &str) -> Result<i64, DecimalError> {
    if s.is_empty() {
        return Err(DecimalError::Format);
    }
    let digits: String = if s.contains('.') {
        let groups: Vec<&str> = s.split('.').collect();
        let first_ok = (1..=3).contains(&groups[0].len());
        let rest_ok = groups[1..].iter().all(|g| g.len() == 3);
        if !first_ok || !rest_ok {
            return Err(DecimalError::Format);
        }
        groups.concat()
    } else {
        s.to_string()
    };
    let trimmed = digits.trim_start_matches('0');
    if trimmed.len() > MAX_INT_DIGITS {
        return Err(DecimalError::Format);
    }
    if trimmed.is_empty() {
        return Ok(0);
    }
    trimmed.parse().map_err(|_| DecimalError::Format)
}

/// Precio de tarifa en céntimos (> 0, como máximo dos decimales).
pub fn parse_price_cents(input: &str) -> Result<i64, ErrorCode> {
    match parse_hundredths(input) {
        Err(DecimalError::Empty) => Err(ErrorCode::PriceRequired),
        Err(DecimalError::Format) => Err(ErrorCode::PriceInvalidFormat),
        Err(DecimalError::TooManyDecimals) => Err(ErrorCode::PriceTooManyDecimals),
        Ok(v) if v <= 0 => Err(ErrorCode::PriceNotPositive),
        Ok(v) => Ok(v),
    }
}

/// Porcentaje (0–100, hasta dos decimales) en puntos básicos.
pub fn parse_percent_bp(input: &str) -> Result<i64, ErrorCode> {
    match parse_hundredths(input) {
        Ok(v) if (0..=10_000).contains(&v) => Ok(v),
        _ => Err(ErrorCode::VatOutOfRange),
    }
}

/// Número con separador de miles "." y dos decimales con ",".
fn format_hundredths(value: i64) -> String {
    let negative = value < 0;
    let abs = value.unsigned_abs();
    let int = (abs / 100).to_string();
    let dec = abs % 100;
    let mut grouped = String::with_capacity(int.len() + int.len() / 3);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(c);
    }
    format!("{}{grouped},{dec:02}", if negative { "-" } else { "" })
}

/// `123456` → `"1.234,56 €"`.
pub fn format_eur(cents: i64) -> String {
    format!("{} €", format_hundredths(cents))
}

/// `2100` → `"21,00"`.
pub fn format_percent_bp(bp: i64) -> String {
    format_hundredths(bp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_table_from_data_model() {
        assert_eq!(parse_price_cents(""), Err(ErrorCode::PriceRequired));
        assert_eq!(parse_price_cents("   "), Err(ErrorCode::PriceRequired));
        assert_eq!(parse_price_cents("abc"), Err(ErrorCode::PriceInvalidFormat));
        assert_eq!(
            parse_price_cents("12,3,4"),
            Err(ErrorCode::PriceInvalidFormat)
        );
        assert_eq!(
            parse_price_cents("12,345"),
            Err(ErrorCode::PriceTooManyDecimals)
        );
        assert_eq!(parse_price_cents("0"), Err(ErrorCode::PriceNotPositive));
        assert_eq!(parse_price_cents("0,00"), Err(ErrorCode::PriceNotPositive));
        assert_eq!(parse_price_cents("-5"), Err(ErrorCode::PriceNotPositive));
        assert_eq!(parse_price_cents("12"), Ok(1200));
        assert_eq!(parse_price_cents("12,5"), Ok(1250));
        assert_eq!(parse_price_cents("12.50"), Ok(1250));
        assert_eq!(parse_price_cents("1.234,56"), Ok(123456));
    }

    #[test]
    fn price_edge_cases() {
        assert_eq!(parse_price_cents("45,50"), Ok(4550));
        assert_eq!(parse_price_cents(" 1 234,56 "), Ok(123456));
        assert_eq!(parse_price_cents("12.345"), Ok(1_234_500)); // punto de miles
        assert_eq!(parse_price_cents("1.234.567"), Ok(123_456_700));
        assert_eq!(parse_price_cents("12,"), Err(ErrorCode::PriceInvalidFormat));
        assert_eq!(parse_price_cents(",5"), Err(ErrorCode::PriceInvalidFormat));
        assert_eq!(
            parse_price_cents("12.3456"),
            Err(ErrorCode::PriceInvalidFormat)
        );
        assert_eq!(
            parse_price_cents("1.23.4"),
            Err(ErrorCode::PriceInvalidFormat)
        );
        assert_eq!(parse_price_cents("12€"), Err(ErrorCode::PriceInvalidFormat));
        assert_eq!(
            parse_price_cents("9999999999999999"),
            Err(ErrorCode::PriceInvalidFormat)
        );
        assert_eq!(parse_price_cents("0,01"), Ok(1));
    }

    #[test]
    fn percent_parsing() {
        assert_eq!(parse_percent_bp("21"), Ok(2100));
        assert_eq!(parse_percent_bp("21,5"), Ok(2150));
        assert_eq!(parse_percent_bp("10.00"), Ok(1000));
        assert_eq!(parse_percent_bp("0"), Ok(0));
        assert_eq!(parse_percent_bp("100"), Ok(10_000));
        assert_eq!(parse_percent_bp("150"), Err(ErrorCode::VatOutOfRange));
        assert_eq!(parse_percent_bp("-1"), Err(ErrorCode::VatOutOfRange));
        assert_eq!(parse_percent_bp("21,555"), Err(ErrorCode::VatOutOfRange));
        assert_eq!(parse_percent_bp(""), Err(ErrorCode::VatOutOfRange));
        assert_eq!(parse_percent_bp("abc"), Err(ErrorCode::VatOutOfRange));
    }

    #[test]
    fn formatting() {
        assert_eq!(format_eur(123456), "1.234,56 €");
        assert_eq!(format_eur(4550), "45,50 €");
        assert_eq!(format_eur(5), "0,05 €");
        assert_eq!(format_eur(12_345_678_900), "123.456.789,00 €");
        assert_eq!(format_percent_bp(2100), "21,00");
        assert_eq!(format_percent_bp(0), "0,00");
        assert_eq!(format_percent_bp(10_000), "100,00");
    }
}
