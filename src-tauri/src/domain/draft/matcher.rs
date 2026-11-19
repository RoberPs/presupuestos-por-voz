//! Coincidencia de lo dictado con las tarifas (FR-017 a FR-021; research.md, R7).
//!
//! Una tarifa solo se aplica cuando encaja sin dudas. Cualquier otra situación deja la línea
//! pendiente: el precio nunca se deduce (Principio IV de la constitución).

use std::cmp::Reverse;
use std::collections::HashMap;

use crate::domain::draft::numbers::{canonical_words, clean_word, is_measure_word};
use crate::domain::draft::parser::ProtectedConcepts;
use crate::domain::draft::units::UnitCatalog;
use crate::domain::draft::PendingReason;
use crate::domain::normalize::normalize;
use crate::domain::rate::RateView;

const MAX_SUGGESTIONS: usize = 5;
const STOP_WORDS: [&str; 13] = [
    "de", "del", "la", "el", "los", "las", "para", "con", "por", "y", "en", "a", "un",
];
/// Longitud mínima para considerar parecidas dos palabras casi iguales.
const MIN_SIMILAR_LEN: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateMatch {
    Applied(i64),
    Pending {
        reason: PendingReason,
        suggestions: Vec<i64>,
    },
}

/// Palabras de un concepto ya normalizado, con los números y las medidas de una única forma.
fn comparable_words(concept_norm: &str) -> Vec<String> {
    let tokens: Vec<String> = concept_norm
        .split(' ')
        .map(clean_word)
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect();
    canonical_words(&tokens)
        .into_iter()
        .map(|word| word.text)
        .collect()
}

/// Dos palabras coinciden si son iguales salvo singular y plural.
pub(crate) fn same_word(a: &str, b: &str) -> bool {
    fn plural_of(singular: &str, plural: &str) -> bool {
        plural
            .strip_prefix(singular)
            .is_some_and(|rest| rest == "s" || rest == "es")
            || singular
                .strip_suffix('z')
                .is_some_and(|stem| plural.strip_prefix(stem).is_some_and(|rest| rest == "ces"))
    }
    // El singular y el plural empiezan igual: la mayoría de las palabras se descartan aquí.
    if a.as_bytes().first() != b.as_bytes().first() {
        return false;
    }
    a == b || plural_of(a, b) || plural_of(b, a)
}

/// Distancia de edición entre dos palabras.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

fn nearly_same_word(a: &str, b: &str) -> bool {
    let (len_a, len_b) = (a.chars().count(), b.chars().count());
    // Con más de dos letras de diferencia no hace falta calcular la distancia.
    len_a.min(len_b) >= MIN_SIMILAR_LEN && len_a.abs_diff(len_b) <= 2 && edit_distance(a, b) <= 2
}

/// Tarifa preparada para compararla.
struct IndexedRate {
    id: i64,
    concept_norm: String,
    words: Vec<String>,
    /// Palabras que deciden si dos conceptos se parecen.
    significant: Vec<String>,
    /// Números y medidas, que solo ayudan a ordenar las sugerencias.
    measures: Vec<String>,
    unit: String,
}

/// Tarifas preparadas una sola vez por cada borrador.
pub struct RateIndex {
    rates: Vec<IndexedRate>,
}

impl RateIndex {
    pub fn new(rates: &[RateView], units: &UnitCatalog) -> Self {
        let mut unit_names: HashMap<&str, String> = HashMap::new();
        let mut rates: Vec<IndexedRate> = rates
            .iter()
            .map(|rate| {
                let concept_norm = normalize(&rate.concept);
                let words = comparable_words(&concept_norm);
                let (measures, others): (Vec<String>, Vec<String>) = words
                    .iter()
                    .cloned()
                    .partition(|word| is_measure_word(word));
                let unit = unit_names
                    .entry(rate.unit.label.as_str())
                    .or_insert_with(|| units.canonical_of_label(&rate.unit.label))
                    .clone();
                IndexedRate {
                    id: rate.id,
                    significant: others
                        .into_iter()
                        .filter(|word| !STOP_WORDS.contains(&word.as_str()))
                        .collect(),
                    measures,
                    words,
                    concept_norm,
                    unit,
                }
            })
            .collect();
        // Orden estable: concepto normalizado e identificador.
        rates.sort_by(|a, b| (&a.concept_norm, a.id).cmp(&(&b.concept_norm, b.id)));
        Self { rates }
    }

    /// Nombres de tarifas cuyos números no deben interpretarse como cantidades.
    pub fn protected_concepts(&self) -> ProtectedConcepts {
        ProtectedConcepts::from_phrases(self.rates.iter().map(|rate| rate.words.clone()))
    }

    /// Busca la tarifa de un trabajo. `unit` es la unidad canónica dictada.
    pub fn match_job(&self, concept: &str, unit: Option<&str>) -> RateMatch {
        let comparable = comparable_words(&normalize(concept));
        let words: Vec<&str> = comparable.iter().map(String::as_str).collect();

        let full: Vec<&IndexedRate> = self
            .rates
            .iter()
            .filter(|rate| {
                rate.words
                    .iter()
                    .all(|word| words.iter().any(|dictated| same_word(word, dictated)))
            })
            .collect();

        if full.is_empty() {
            return self.similar(&words);
        }

        let compatible: Vec<&IndexedRate> = full
            .iter()
            .copied()
            .filter(|rate| unit.is_none_or(|u| rate.unit == u))
            .collect();

        match compatible.as_slice() {
            [only] => RateMatch::Applied(only.id),
            [] => pending(PendingReason::UnitMismatch, &full),
            several => pending(PendingReason::AmbiguousRate, several),
        }
    }

    /// Tarifas que solo se parecen: se sugieren sin aplicar su precio.
    fn similar(&self, words: &[&str]) -> RateMatch {
        // Muchas tarifas repiten palabras: cada una se compara una sola vez.
        let mut word_scores: HashMap<&str, usize> = HashMap::new();
        let mut scored: Vec<(usize, &IndexedRate)> = self
            .rates
            .iter()
            .filter_map(|rate| {
                let score = similarity(rate, words, &mut word_scores);
                (score > 0).then_some((score, rate))
            })
            .collect();
        // `sort_by_key` es estable: a igual parecido se conserva el orden de las tarifas.
        scored.sort_by_key(|(score, _)| Reverse(*score));

        let suggestions: Vec<i64> = scored
            .iter()
            .take(MAX_SUGGESTIONS)
            .map(|(_, rate)| rate.id)
            .collect();
        let reason = if suggestions.is_empty() {
            PendingReason::NoRate
        } else {
            PendingReason::SimilarRate
        };
        RateMatch::Pending {
            reason,
            suggestions,
        }
    }
}

fn pending(reason: PendingReason, rates: &[&IndexedRate]) -> RateMatch {
    RateMatch::Pending {
        reason,
        suggestions: rates.iter().take(MAX_SUGGESTIONS).map(|r| r.id).collect(),
    }
}

/// Las palabras compartidas pesan más que las casi iguales.
fn word_score(word: &str, words: &[&str]) -> usize {
    if words.iter().any(|dictated| same_word(word, dictated)) {
        10
    } else if words
        .iter()
        .any(|dictated| nearly_same_word(word, dictated))
    {
        1
    } else {
        0
    }
}

/// Parecido de una tarifa con lo dictado. Los números y las medidas ayudan a ordenar, pero
/// compartir solo un «20 mm» no hace parecidos dos conceptos.
fn similarity<'a>(
    rate: &'a IndexedRate,
    words: &[&str],
    word_scores: &mut HashMap<&'a str, usize>,
) -> usize {
    let text_score: usize = rate
        .significant
        .iter()
        .map(|word| {
            *word_scores
                .entry(word.as_str())
                .or_insert_with(|| word_score(word, words))
        })
        .sum();
    if text_score == 0 {
        return 0;
    }

    let shared_measures = rate
        .measures
        .iter()
        .filter(|word| words.iter().any(|dictated| same_word(word, dictated)))
        .count();
    text_score + shared_measures * 10
}

/// Busca la tarifa de un único trabajo.
pub fn match_job(
    concept: &str,
    unit: Option<&str>,
    rates: &[RateView],
    units: &UnitCatalog,
) -> RateMatch {
    RateIndex::new(rates, units).match_job(concept, unit)
}
