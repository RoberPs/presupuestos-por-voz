use crate::error::ErrorCode;

fn is_spanish_number(digits: &str) -> bool {
    digits.len() == 9 && matches!(digits.as_bytes()[0], b'6' | b'7' | b'8' | b'9')
}

/// Español de 9 cifras (6/7/8/9) con `+34`/`0034` opcional, o internacional de 6 a 15 cifras.
/// Devuelve el número compacto (sin separadores).
pub fn validate_phone(s: &str) -> Result<String, ErrorCode> {
    let compact: String =
        s.chars().filter(|c| !c.is_whitespace() && !matches!(c, '-' | '.' | '(' | ')')).collect();

    let (international, digits) = if let Some(rest) = compact.strip_prefix('+') {
        (true, rest)
    } else if let Some(rest) = compact.strip_prefix("00") {
        (true, rest)
    } else {
        (false, compact.as_str())
    };

    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ErrorCode::PhoneInvalid);
    }

    let valid = if !international {
        is_spanish_number(digits)
    } else if let Some(national) = digits.strip_prefix("34") {
        is_spanish_number(national)
    } else {
        (6..=15).contains(&digits.len())
    };

    if valid {
        Ok(compact)
    } else {
        Err(ErrorCode::PhoneInvalid)
    }
}
