//! Validadores de datos fiscales y de contacto (R6). Cada uno devuelve el valor normalizado.

mod email;
mod nif;
mod phone;
mod postal_code;

pub use email::validate_email;
pub use nif::{normalize_tax_id, validate_tax_id};
pub use phone::validate_phone;
pub use postal_code::validate_postal_code;
