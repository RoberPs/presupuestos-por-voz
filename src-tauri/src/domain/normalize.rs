//! Normalización de texto para duplicados y búsqueda (R5).

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// NFD, sin marcas diacríticas, minúsculas, sin espacios sobrantes y con los internos colapsados.
pub fn normalize(s: &str) -> String {
    let folded: String =
        s.nfd().filter(|c| !is_combining_mark(*c)).flat_map(char::to_lowercase).collect();
    collapse_spaces(&folded)
}

/// `trim` y colapso de espacios internos, conservando mayúsculas y tildes.
pub fn collapse_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Escapa `\`, `%` y `_` para usar el texto en un `LIKE ... ESCAPE '\'`.
pub fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_accents_case_and_spaces() {
        assert_eq!(normalize("  Instalación  de GRIFO "), "instalacion de grifo");
        assert_eq!(normalize("Pintura"), normalize("pintura"));
        assert_eq!(normalize("ÁBACO\tñu"), "abaco nu");
    }

    #[test]
    fn keeps_superscripts_stable() {
        assert_eq!(normalize("m²"), "m²");
        assert_eq!(normalize(" M² "), "m²");
    }

    #[test]
    fn escapes_like_wildcards() {
        assert_eq!(escape_like(r"50%_a\b"), r"50\%\_a\\b");
        assert_eq!(escape_like("grifo"), "grifo");
    }
}
