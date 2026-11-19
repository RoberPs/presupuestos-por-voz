//! DNI, NIE y CIF españoles con su carácter de control oficial.

use crate::error::ErrorCode;

const DNI_LETTERS: &[u8; 23] = b"TRWAGMYFPDXBNJZSQVHLCKE";
const CIF_CONTROL_LETTERS: &[u8; 10] = b"JABCDEFGHI";

/// Mayúsculas y sin espacios, guiones ni puntos.
pub fn normalize_tax_id(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '.')
        .flat_map(char::to_uppercase)
        .collect()
}

pub fn validate_tax_id(s: &str) -> Result<String, ErrorCode> {
    let id = normalize_tax_id(s);
    let ok = id.len() == 9 && id.is_ascii() && (is_dni(&id) || is_nie(&id) || is_cif(&id));
    if ok {
        Ok(id)
    } else {
        Err(ErrorCode::TaxIdInvalid)
    }
}

fn dni_letter(number: u32) -> u8 {
    DNI_LETTERS[(number % 23) as usize]
}

fn is_dni(id: &str) -> bool {
    let (num, letter) = id.split_at(8);
    num.bytes().all(|b| b.is_ascii_digit())
        && num
            .parse::<u32>()
            .map(|n| dni_letter(n) == letter.as_bytes()[0])
            .unwrap_or(false)
}

fn is_nie(id: &str) -> bool {
    let prefix = match id.as_bytes()[0] {
        b'X' => '0',
        b'Y' => '1',
        b'Z' => '2',
        _ => return false,
    };
    let number = format!("{prefix}{}", &id[1..8]);
    number.bytes().all(|b| b.is_ascii_digit())
        && number
            .parse::<u32>()
            .map(|n| dni_letter(n) == id.as_bytes()[8])
            .unwrap_or(false)
}

fn is_cif(id: &str) -> bool {
    let bytes = id.as_bytes();
    let org = bytes[0];
    if !b"ABCDEFGHJNPQRSUVW".contains(&org) {
        return false;
    }
    let digits = &bytes[1..8];
    if !digits.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let mut sum = 0u32;
    for (i, d) in digits.iter().map(|b| u32::from(b - b'0')).enumerate() {
        if i % 2 == 0 {
            // posiciones impares (1ª, 3ª, 5ª, 7ª): doblar y sumar sus cifras
            let doubled = d * 2;
            sum += doubled / 10 + doubled % 10;
        } else {
            sum += d;
        }
    }
    let control = ((10 - sum % 10) % 10) as usize;
    let control_digit = b'0' + control as u8;
    let control_letter = CIF_CONTROL_LETTERS[control];
    let given = bytes[8];

    match org {
        // Siempre letra
        b'P' | b'Q' | b'R' | b'S' | b'N' | b'W' => given == control_letter,
        // Siempre dígito
        b'A' | b'B' | b'E' | b'H' => given == control_digit,
        // Cualquiera de los dos
        _ => given == control_digit || given == control_letter,
    }
}
