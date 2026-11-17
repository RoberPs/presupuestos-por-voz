use app_lib::domain::validation::{
    normalize_tax_id, validate_email, validate_phone, validate_postal_code, validate_tax_id,
};
use app_lib::error::ErrorCode;

#[test]
fn dni() {
    assert_eq!(validate_tax_id("12345678Z"), Ok("12345678Z".to_string()));
    assert_eq!(validate_tax_id("12345678A"), Err(ErrorCode::TaxIdInvalid));
    assert_eq!(validate_tax_id("1234567Z"), Err(ErrorCode::TaxIdInvalid));
}

#[test]
fn nie() {
    assert_eq!(validate_tax_id("X1234567L"), Ok("X1234567L".to_string()));
    assert_eq!(validate_tax_id("X1234567A"), Err(ErrorCode::TaxIdInvalid));
    assert!(validate_tax_id("Y1234567X").is_ok());
    assert!(validate_tax_id("Z1234567R").is_ok());
}

#[test]
fn cif() {
    assert_eq!(validate_tax_id("A58818501"), Ok("A58818501".to_string()));
    assert_eq!(validate_tax_id("B65410011"), Ok("B65410011".to_string()));
    assert_eq!(validate_tax_id("A58818502"), Err(ErrorCode::TaxIdInvalid));
    assert_eq!(validate_tax_id("B65410012"), Err(ErrorCode::TaxIdInvalid));
    // Letra de control (tipo P) en lugar de dígito
    assert!(validate_tax_id("P2800000H").is_ok());
    assert_eq!(validate_tax_id("P2800000E"), Err(ErrorCode::TaxIdInvalid));
    // Tipo P exige letra: el dígito equivalente (8) no vale
    assert_eq!(validate_tax_id("P28000008"), Err(ErrorCode::TaxIdInvalid));
}

#[test]
fn tax_id_normalization() {
    assert_eq!(normalize_tax_id("12.345.678-z"), "12345678Z");
    assert_eq!(normalize_tax_id(" x 1234567 l "), "X1234567L");
    assert_eq!(validate_tax_id("12.345.678-z"), Ok("12345678Z".to_string()));
    assert_eq!(validate_tax_id(""), Err(ErrorCode::TaxIdInvalid));
}

#[test]
fn postal_code() {
    assert_eq!(validate_postal_code("28001"), Ok("28001".to_string()));
    assert_eq!(validate_postal_code(" 01001 "), Ok("01001".to_string()));
    assert_eq!(validate_postal_code("52080"), Ok("52080".to_string()));
    assert_eq!(validate_postal_code("5300"), Err(ErrorCode::PostalCodeInvalid));
    assert_eq!(validate_postal_code("53001"), Err(ErrorCode::PostalCodeInvalid));
    assert_eq!(validate_postal_code("00123"), Err(ErrorCode::PostalCodeInvalid));
    assert_eq!(validate_postal_code("2800A"), Err(ErrorCode::PostalCodeInvalid));
}

#[test]
fn phone() {
    assert!(validate_phone("600 123 456").is_ok());
    assert!(validate_phone("+34 912345678").is_ok());
    assert!(validate_phone("0034 912-345-678").is_ok());
    assert!(validate_phone("+44 20 7946 0958").is_ok());
    assert_eq!(validate_phone("12345"), Err(ErrorCode::PhoneInvalid));
    assert_eq!(validate_phone("500123456"), Err(ErrorCode::PhoneInvalid));
    assert_eq!(validate_phone("+34 500123456"), Err(ErrorCode::PhoneInvalid));
    assert_eq!(validate_phone("+1 23"), Err(ErrorCode::PhoneInvalid));
    assert_eq!(validate_phone("600abc456"), Err(ErrorCode::PhoneInvalid));
}

#[test]
fn email() {
    assert!(validate_email("a@b.es").is_ok());
    assert!(validate_email(" nombre.apellido+x@correo.example.com ").is_ok());
    assert_eq!(validate_email("a@b"), Err(ErrorCode::EmailInvalid));
    assert_eq!(validate_email("@b.es"), Err(ErrorCode::EmailInvalid));
    assert_eq!(validate_email("a b@c.es"), Err(ErrorCode::EmailInvalid));
    assert_eq!(validate_email("a@@b.es"), Err(ErrorCode::EmailInvalid));
    assert_eq!(validate_email("a@.es"), Err(ErrorCode::EmailInvalid));
}
