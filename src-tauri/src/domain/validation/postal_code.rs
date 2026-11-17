use crate::error::ErrorCode;

/// 5 dígitos con prefijo de provincia 01–52.
pub fn validate_postal_code(s: &str) -> Result<String, ErrorCode> {
    let cp = s.trim();
    let valid = cp.len() == 5
        && cp.bytes().all(|b| b.is_ascii_digit())
        && matches!(cp[..2].parse::<u8>(), Ok(1..=52));
    if valid {
        Ok(cp.to_string())
    } else {
        Err(ErrorCode::PostalCodeInvalid)
    }
}
