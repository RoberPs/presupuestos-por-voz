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

/// Cantidad máxima de una línea, en centésimas (data-model.md).
pub const MAX_QUANTITY_CENTI: i64 = 99_999_999;

/// Cantidad en centésimas enteras (> 0, como máximo dos decimales): `"1,5"` → `150`.
pub fn parse_quantity_centi(input: &str) -> Result<i64, ErrorCode> {
    match parse_hundredths(input) {
        Err(DecimalError::Empty) => Err(ErrorCode::QuantityRequired),
        Err(DecimalError::Format) => Err(ErrorCode::QuantityInvalidFormat),
        Err(DecimalError::TooManyDecimals) => Err(ErrorCode::QuantityTooManyDecimals),
        Ok(v) if v <= 0 => Err(ErrorCode::QuantityNotPositive),
        Ok(v) if v > MAX_QUANTITY_CENTI => Err(ErrorCode::QuantityInvalidFormat),
        Ok(v) => Ok(v),
    }
}

/// `150` → `"1,5"`; `300` → `"3"`. Sin ceros decimales sobrantes.
pub fn format_quantity(quantity_centi: i64) -> String {
    let full = format_hundredths(quantity_centi);
    match full.strip_suffix(",00") {
        Some(int) => int.to_string(),
        None => full.strip_suffix('0').unwrap_or(&full).to_string(),
    }
}

/// Importe de una línea: cantidad × precio, redondeado al céntimo más cercano (R9).
pub fn line_amount_cents(quantity_centi: i64, price_cents: i64) -> Result<i64, ErrorCode> {
    let amount = (i128::from(quantity_centi) * i128::from(price_cents) + 50) / 100;
    i64::try_from(amount).map_err(|_| ErrorCode::AmountTooLarge)
}

/// IVA de una base: base × puntos básicos, redondeado al céntimo más cercano (R9).
pub fn vat_amount_cents(base_cents: i64, vat_rate_bp: i64) -> Result<i64, ErrorCode> {
    let vat = (i128::from(base_cents) * i128::from(vat_rate_bp) + 5_000) / 10_000;
    i64::try_from(vat).map_err(|_| ErrorCode::AmountTooLarge)
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

    #[test]
    fn quantity_table_from_data_model() {
        assert_eq!(parse_quantity_centi(""), Err(ErrorCode::QuantityRequired));
        assert_eq!(
            parse_quantity_centi("   "),
            Err(ErrorCode::QuantityRequired)
        );
        assert_eq!(
            parse_quantity_centi("abc"),
            Err(ErrorCode::QuantityInvalidFormat)
        );
        assert_eq!(
            parse_quantity_centi("1,234"),
            Err(ErrorCode::QuantityTooManyDecimals)
        );
        assert_eq!(
            parse_quantity_centi("0"),
            Err(ErrorCode::QuantityNotPositive)
        );
        assert_eq!(
            parse_quantity_centi("-2"),
            Err(ErrorCode::QuantityNotPositive)
        );
        assert_eq!(parse_quantity_centi("3"), Ok(300));
        assert_eq!(parse_quantity_centi("1,5"), Ok(150));
        assert_eq!(parse_quantity_centi("2.25"), Ok(225));
    }

    #[test]
    fn quantity_upper_limit() {
        assert_eq!(parse_quantity_centi("999999,99"), Ok(MAX_QUANTITY_CENTI));
        assert_eq!(
            parse_quantity_centi("1000000"),
            Err(ErrorCode::QuantityInvalidFormat)
        );
    }

    #[test]
    fn quantity_formatting() {
        assert_eq!(format_quantity(300), "3");
        assert_eq!(format_quantity(150), "1,5");
        assert_eq!(format_quantity(225), "2,25");
        assert_eq!(format_quantity(5), "0,05");
        assert_eq!(format_quantity(123_456_700), "1.234.567");
    }

    #[test]
    fn line_amounts_round_to_the_cent() {
        assert_eq!(line_amount_cents(300, 2500), Ok(7500));
        assert_eq!(line_amount_cents(200, 3000), Ok(6000));
        assert_eq!(line_amount_cents(150, 3333), Ok(5000));
        assert_eq!(line_amount_cents(1, 49), Ok(0));
        assert_eq!(line_amount_cents(1, 50), Ok(1));
    }

    #[test]
    fn vat_amounts_round_to_the_cent() {
        assert_eq!(vat_amount_cents(13_500, 2100), Ok(2835));
        assert_eq!(vat_amount_cents(16_000, 2100), Ok(3360));
        assert_eq!(vat_amount_cents(0, 2100), Ok(0));
        assert_eq!(vat_amount_cents(1, 5000), Ok(1));
    }

    #[test]
    fn overflow_is_rejected() {
        assert_eq!(
            line_amount_cents(i64::MAX, i64::MAX),
            Err(ErrorCode::AmountTooLarge)
        );
        assert_eq!(vat_amount_cents(i64::MAX, 10_000), Ok(i64::MAX));
        assert_eq!(
            vat_amount_cents(i64::MAX, 20_000),
            Err(ErrorCode::AmountTooLarge)
        );
    }
}
