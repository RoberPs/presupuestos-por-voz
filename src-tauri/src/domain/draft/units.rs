//! Unidades que se reconocen al dictar (research.md, R8). Nunca se convierten entre sí.

use std::collections::HashMap;

use crate::domain::normalize::normalize;
use crate::domain::rate::UnitView;

/// Alias de las unidades predefinidas: (alias normalizado, unidad canónica).
const SEED_ALIASES: [(&str, &str); 17] = [
    ("metro", "metro"),
    ("metros", "metro"),
    ("m", "metro"),
    ("hora", "hora"),
    ("horas", "hora"),
    ("h", "hora"),
    ("unidad", "unidad"),
    ("unidades", "unidad"),
    ("ud", "unidad"),
    ("uds", "unidad"),
    ("m²", "m²"),
    ("m2", "m²"),
    ("metro cuadrado", "m²"),
    ("metros cuadrados", "m²"),
    ("metro lineal", "metro lineal"),
    ("metros lineales", "metro lineal"),
    ("ml", "metro lineal"),
];

/// Unidades conocidas, con sus alias y su texto para mostrar.
#[derive(Debug, Clone)]
pub struct UnitCatalog {
    aliases: HashMap<String, String>,
    labels: HashMap<String, String>,
    max_words: usize,
}

impl Default for UnitCatalog {
    fn default() -> Self {
        Self::new(&[])
    }
}

impl UnitCatalog {
    /// Catálogo con las unidades predefinidas y las de `units`.
    pub fn new(units: &[UnitView]) -> Self {
        let mut aliases: HashMap<String, String> = SEED_ALIASES
            .iter()
            .map(|(alias, canonical)| ((*alias).to_string(), (*canonical).to_string()))
            .collect();
        let mut labels = HashMap::new();

        for unit in units {
            let norm = normalize(&unit.label);
            if norm.is_empty() {
                continue;
            }
            let canonical = aliases.get(&norm).cloned().unwrap_or_else(|| norm.clone());
            labels.insert(canonical.clone(), unit.label.clone());
            for alias in [norm.clone(), format!("{norm}s"), format!("{norm}es")] {
                aliases.entry(alias).or_insert_with(|| canonical.clone());
            }
        }

        let max_words = aliases
            .keys()
            .map(|alias| alias.split(' ').count())
            .max()
            .unwrap_or(1);
        Self {
            aliases,
            labels,
            max_words,
        }
    }

    /// Unidad canónica de un texto normalizado, si se conoce.
    pub fn canonical(&self, phrase: &str) -> Option<&str> {
        self.aliases.get(phrase).map(String::as_str)
    }

    /// Unidad canónica de una etiqueta cualquiera; si no se conoce, su texto normalizado.
    pub fn canonical_of_label(&self, label: &str) -> String {
        let norm = normalize(label);
        self.canonical(&norm).map(str::to_string).unwrap_or(norm)
    }

    /// Texto para mostrar de una unidad canónica.
    pub fn label_of(&self, canonical: &str) -> String {
        self.labels
            .get(canonical)
            .cloned()
            .unwrap_or_else(|| canonical.to_string())
    }

    /// Unidad que empieza en `tokens[start]`: (palabras que ocupa, unidad canónica). Gana la más larga.
    pub fn match_at(&self, tokens: &[String], start: usize) -> Option<(usize, String)> {
        let available = tokens.len().saturating_sub(start).min(self.max_words);
        (1..=available).rev().find_map(|len| {
            let phrase = tokens[start..start + len].join(" ");
            self.canonical(&phrase)
                .map(|canonical| (len, canonical.to_string()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<String> {
        text.split(' ').map(str::to_string).collect()
    }

    #[test]
    fn the_longest_unit_wins() {
        let catalog = UnitCatalog::default();
        assert_eq!(
            catalog.match_at(&tokens("metros lineales de tubo"), 0),
            Some((2, "metro lineal".to_string()))
        );
        assert_eq!(
            catalog.match_at(&tokens("metros de tubo"), 0),
            Some((1, "metro".to_string()))
        );
    }

    #[test]
    fn custom_units_are_recognized_in_singular_and_plural() {
        let catalog = UnitCatalog::new(&[UnitView {
            id: 9,
            label: "Saco".into(),
            is_predefined: false,
        }]);
        assert_eq!(catalog.canonical("sacos"), Some("saco"));
        assert_eq!(catalog.label_of("saco"), "Saco");
    }

    #[test]
    fn predefined_labels_keep_their_text() {
        let catalog = UnitCatalog::new(&[UnitView {
            id: 4,
            label: "m²".into(),
            is_predefined: true,
        }]);
        assert_eq!(catalog.canonical_of_label("M2"), "m²");
        assert_eq!(catalog.label_of("m²"), "m²");
    }
}
