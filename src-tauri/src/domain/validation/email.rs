use crate::error::ErrorCode;

/// Comprobación sintáctica básica `local@dominio.tld` (sin verificar la entrega).
pub fn validate_email(s: &str) -> Result<String, ErrorCode> {
    let email = s.trim();
    let mut parts = email.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(ErrorCode::EmailInvalid);
    };
    let labels: Vec<&str> = domain.split('.').collect();
    let valid = !local.is_empty()
        && !email.chars().any(char::is_whitespace)
        && labels.len() >= 2
        && labels.iter().all(|l| !l.is_empty())
        && labels.last().is_some_and(|tld| tld.chars().count() >= 2);
    if valid {
        Ok(email.to_string())
    } else {
        Err(ErrorCode::EmailInvalid)
    }
}
