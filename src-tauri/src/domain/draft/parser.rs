//! Del texto de la visita a los trabajos, con reglas deterministas (research.md, R6).
//!
//! Aquí no interviene ningún precio: solo se separan concepto, cantidad y unidad.

use std::ops::Range;

use crate::domain::draft::matcher::same_word;
use crate::domain::draft::numbers::{
    canonical_words, clean_word, find_quantity, is_number_word, normalized_tokens,
};
use crate::domain::draft::units::UnitCatalog;
use crate::domain::draft::QuantityIssue;
use crate::domain::normalize::normalize;

/// Un trabajo reconocido en el texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedJob {
    pub concept: String,
    pub quantity_centi: Option<i64>,
    pub quantity_issue: Option<QuantityIssue>,
    /// Unidad canónica dictada.
    pub unit: Option<String>,
    /// Lo dictado, para mostrarlo junto a la línea.
    pub fragment: String,
}

/// Trabajos en el orden del texto y fragmentos que no se han podido interpretar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedText {
    pub jobs: Vec<ParsedJob>,
    pub unparsed: Vec<String>,
}

const CONNECTORS: [&str; 7] = ["y", "e", "tambien", "ademas", "luego", "despues", "pues"];
const FILLERS: [&str; 17] = [
    "bueno", "vale", "pues", "eh", "ah", "mm", "hmm", "nada", "ok", "venga", "vamos", "entonces",
    "si", "no", "claro", "mira", "oye",
];
const CORRECTIONS: [&[&str]; 7] = [
    &["no", "mejor"],
    &["mejor", "dicho"],
    &["o", "sea"],
    &["perdon"],
    &["digo"],
    &["mejor"],
    &["no"],
];
const VAGUE: [&[&str]; 11] = [
    &["unos", "cuantos"],
    &["unas", "cuantas"],
    &["mas", "o", "menos"],
    &["unos"],
    &["unas"],
    &["algunos"],
    &["algunas"],
    &["varios"],
    &["varias"],
    &["aproximadamente"],
    &["aprox"],
];

/// Sustituye a una palabra protegida: no es un número, una unidad ni un signo de corrección.
const PROTECTED_WORD: &str = "\u{1}";

/// Nombres de tarifas con números, como «Termo 80»: esos números no son cantidades.
#[derive(Debug, Clone, Default)]
pub struct ProtectedConcepts {
    phrases: Vec<Vec<String>>,
}

impl ProtectedConcepts {
    pub fn new<'a>(concepts: impl IntoIterator<Item = &'a str>) -> Self {
        Self::from_phrases(concepts.into_iter().map(|concept| {
            canonical_words(&normalized_tokens(concept))
                .into_iter()
                .map(|word| word.text)
                .collect()
        }))
    }

    /// Conceptos ya escritos de forma comparable. Solo se guardan los que llevan números.
    pub fn from_phrases(phrases: impl IntoIterator<Item = Vec<String>>) -> Self {
        let mut phrases: Vec<Vec<String>> = phrases
            .into_iter()
            .filter(|words| words.iter().any(|word| is_number_word(word)))
            .collect();
        // Primero los nombres más largos: «Termo 80 inox» antes que «Termo 80».
        phrases.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
        phrases.dedup();
        Self { phrases }
    }

    /// Oculta los nombres protegidos que aparecen completos en el segmento.
    fn mask(&self, norm: &mut [String]) {
        let words = canonical_words(norm);
        if self.phrases.is_empty() || !words.iter().any(|word| word.is_number()) {
            return;
        }

        let mut pos = 0;
        while pos < words.len() {
            let found = self.phrases.iter().find(|phrase| {
                words.len() >= pos + phrase.len()
                    && phrase
                        .iter()
                        .zip(&words[pos..])
                        .all(|(expected, word)| same_word(expected, &word.text))
            });
            let Some(phrase) = found else {
                pos += 1;
                continue;
            };
            let last = pos + phrase.len() - 1;
            for token in &mut norm[words[pos].tokens.start..words[last].tokens.end] {
                *token = PROTECTED_WORD.to_string();
            }
            pos += phrase.len();
        }
    }
}

/// Parte el texto por signos de puntuación, respetando los decimales («1,5»).
fn split_segments(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut segments = Vec::new();
    let mut current = String::new();

    for (i, &c) in chars.iter().enumerate() {
        let between_digits = i > 0
            && chars[i - 1].is_ascii_digit()
            && chars.get(i + 1).is_some_and(char::is_ascii_digit);
        let splits = match c {
            '.' | ',' => !between_digits,
            ';' | ':' | '!' | '?' | '\n' | '\r' => true,
            _ => false,
        };
        if splits {
            segments.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    segments.push(current);
    segments
}

/// Palabras de un segmento: tal como se dictaron y normalizadas, posición a posición.
struct Words {
    raw: Vec<String>,
    norm: Vec<String>,
}

impl Words {
    fn new(segment: &str) -> Self {
        let mut raw = Vec::new();
        let mut norm = Vec::new();
        for word in segment.split_whitespace() {
            let cleaned = clean_word(word);
            let normalized = normalize(cleaned);
            if !normalized.is_empty() {
                raw.push(cleaned.to_string());
                norm.push(normalized);
            }
        }
        Self { raw, norm }
    }
}

/// Parte un segmento al final de cada grupo cantidad + unidad.
fn split_groups(norm: &[String], units: &UnitCatalog) -> Vec<Range<usize>> {
    let mut groups = Vec::new();
    let mut start = 0;
    let mut pos = 0;

    while let Some(found) = find_quantity(norm, pos, units) {
        let followed_by_de = norm.get(found.end).is_some_and(|w| w == "de" || w == "del");
        let boundary = if found.unit.is_some() && !followed_by_de {
            found.end
        } else {
            // «3 metros de bajante»: el concepto llega hasta la siguiente cantidad.
            match find_quantity(norm, found.end, units) {
                Some(next) => next.start,
                None => break,
            }
        };
        if boundary > start {
            groups.push(start..boundary);
        }
        start = boundary;
        pos = boundary;
    }

    if start < norm.len() {
        groups.push(start..norm.len());
    }
    groups
}

/// Longitud de la expresión de `patterns` que empieza en `start`, si la hay.
fn pattern_len(norm: &[String], start: usize, patterns: &[&[&str]]) -> Option<usize> {
    patterns
        .iter()
        .filter(|pattern| {
            norm.len() >= start + pattern.len()
                && pattern.iter().zip(&norm[start..]).all(|(p, w)| p == w)
        })
        .map(|pattern| pattern.len())
        .max()
}

/// Lo que dice un grupo una vez separados concepto, cantidad y unidad.
struct Analysis {
    concept_raw: Vec<String>,
    concept_is_filler: bool,
    quantity_centi: Option<i64>,
    unit: Option<String>,
    vague: bool,
}

fn analyze(raw: &[String], norm: &[String], units: &UnitCatalog) -> Analysis {
    // Se quitan las expresiones vagas: la cantidad que las acompañe no se da por buena.
    let mut kept = Vec::new();
    let mut vague = false;
    let mut pos = 0;
    while pos < norm.len() {
        match pattern_len(norm, pos, &VAGUE) {
            Some(len) => {
                vague = true;
                pos += len;
            }
            None => {
                kept.push(pos);
                pos += 1;
            }
        }
    }
    let kept_norm: Vec<String> = kept.iter().map(|&i| norm[i].clone()).collect();

    let found = find_quantity(&kept_norm, 0, units);
    let mut removed = found.as_ref().map(|m| m.start..m.end);
    let mut unit = found.as_ref().and_then(|m| m.unit.clone());
    if found.is_none() && vague {
        // «unos metros de tubo»: hay unidad aunque no haya cantidad.
        removed = (0..kept_norm.len()).find_map(|start| {
            units.match_at(&kept_norm, start).map(|(len, canonical)| {
                unit = Some(canonical);
                start..start + len
            })
        });
    }

    let mut concept: Vec<usize> = Vec::new();
    for (i, &original) in kept.iter().enumerate() {
        if removed.as_ref().is_some_and(|r| r.contains(&i)) {
            continue;
        }
        // La preposición que une la cantidad con el concepto: «3 metros de bajante».
        let joins_quantity = removed.as_ref().is_some_and(|r| i == r.end)
            && matches!(norm[original].as_str(), "de" | "del");
        if !joins_quantity {
            concept.push(original);
        }
    }
    while concept
        .last()
        .is_some_and(|&i| matches!(norm[i].as_str(), "de" | "del" | "y" | "e"))
    {
        concept.pop();
    }

    Analysis {
        concept_is_filler: concept.iter().all(|&i| FILLERS.contains(&norm[i].as_str())),
        concept_raw: concept.iter().map(|&i| raw[i].clone()).collect(),
        quantity_centi: found.map(|m| m.centi),
        unit,
        vague,
    }
}

/// Recorre los grupos del texto acumulando trabajos.
struct Interpreter<'a> {
    units: &'a UnitCatalog,
    jobs: Vec<ParsedJob>,
    unparsed: Vec<String>,
    /// Cantidad suelta que aún no se sabe a qué trabajo pertenece.
    orphan: Option<(String, Option<String>)>,
    /// Corrección suelta («no», «perdón») a la espera de la cantidad buena.
    correction: Option<String>,
}

impl Interpreter<'_> {
    fn flush_orphan(&mut self) {
        if let Some((text, _)) = self.orphan.take() {
            self.unparsed.push(text);
        }
    }

    fn flush_correction(&mut self) {
        if let Some(text) = self.correction.take() {
            self.unparsed.push(text);
        }
    }

    fn group(&mut self, raw: &[String], norm: &[String]) {
        let mut start = 0;
        while norm
            .get(start)
            .is_some_and(|w| CONNECTORS.contains(&w.as_str()))
        {
            start += 1;
        }
        let mut end = norm.len();
        while end > start && matches!(norm[end - 1].as_str(), "y" | "e") {
            end -= 1;
        }
        if start == end {
            return;
        }
        let (raw, norm) = (&raw[start..end], &norm[start..end]);
        let text = raw.join(" ");

        if let Some(len) = pattern_len(norm, 0, &CORRECTIONS) {
            let rest = len..norm.len();
            if rest.is_empty() {
                self.flush_correction();
                self.correction = Some(text);
                return;
            }
            if find_quantity(&norm[rest.clone()], 0, self.units).is_some() {
                let said = self.take_correction(text);
                self.rectify(&raw[rest.clone()], &norm[rest], said);
                return;
            }
        }

        if self.correction.is_some() {
            if find_quantity(norm, 0, self.units).is_some() {
                let said = self.take_correction(text);
                self.rectify(raw, norm, said);
                return;
            }
            self.flush_correction();
        }

        self.regular(raw, norm, text);
    }

    /// Une la corrección pendiente con lo que se acaba de dictar.
    fn take_correction(&mut self, text: String) -> String {
        match self.correction.take() {
            Some(correction) => format!("{correction}, {text}"),
            None => text,
        }
    }

    /// El profesional se ha corregido: nunca se supone cuál de las cantidades es la buena.
    fn rectify(&mut self, raw: &[String], norm: &[String], said: String) {
        let analysis = analyze(raw, norm, self.units);
        if !analysis.concept_raw.is_empty() && !analysis.concept_is_filler {
            let (fragment, orphan_unit) = match self.orphan.take() {
                Some((orphan, unit)) => (format!("{orphan}, {said}"), unit),
                None => (said, None),
            };
            self.jobs.push(ParsedJob {
                concept: analysis.concept_raw.join(" "),
                quantity_centi: None,
                quantity_issue: Some(QuantityIssue::Rectified),
                unit: analysis.unit.or(orphan_unit),
                fragment,
            });
            return;
        }

        self.flush_orphan();
        match self.jobs.last_mut() {
            Some(job) => {
                job.quantity_centi = None;
                job.quantity_issue = Some(QuantityIssue::Rectified);
                job.fragment = format!("{}, {said}", job.fragment);
            }
            None => self.unparsed.push(said),
        }
    }

    fn regular(&mut self, raw: &[String], norm: &[String], text: String) {
        self.flush_orphan();
        let analysis = analyze(raw, norm, self.units);

        if !analysis.concept_raw.is_empty() && !analysis.concept_is_filler {
            let (quantity_centi, quantity_issue) = match (analysis.vague, analysis.quantity_centi) {
                (true, _) => (None, Some(QuantityIssue::Ambiguous)),
                (false, Some(centi)) => (Some(centi), None),
                (false, None) => (None, Some(QuantityIssue::Missing)),
            };
            self.jobs.push(ParsedJob {
                concept: analysis.concept_raw.join(" "),
                quantity_centi,
                quantity_issue,
                unit: analysis.unit,
                fragment: text,
            });
            return;
        }

        if analysis.quantity_centi.is_none() && !analysis.vague {
            self.unparsed.push(text);
            return;
        }

        // Solo cantidad y unidad: se une al concepto anterior si este no tiene cantidad.
        match self.jobs.last_mut() {
            Some(job) if job.quantity_issue == Some(QuantityIssue::Missing) => {
                job.quantity_centi = analysis.quantity_centi.filter(|_| !analysis.vague);
                job.quantity_issue = analysis.vague.then_some(QuantityIssue::Ambiguous);
                job.unit = analysis.unit;
                job.fragment = format!("{}, {text}", job.fragment);
            }
            _ => self.orphan = Some((text, analysis.unit)),
        }
    }
}

/// Interpreta un texto sin tener en cuenta los nombres de las tarifas.
pub fn parse_text(text: &str, units: &UnitCatalog) -> ParsedText {
    parse_text_with(text, units, &ProtectedConcepts::default())
}

/// Interpreta el texto de la visita. A igual texto y tarifas, siempre el mismo resultado (FR-014).
pub fn parse_text_with(
    text: &str,
    units: &UnitCatalog,
    protected: &ProtectedConcepts,
) -> ParsedText {
    let mut interpreter = Interpreter {
        units,
        jobs: Vec::new(),
        unparsed: Vec::new(),
        orphan: None,
        correction: None,
    };

    for segment in split_segments(text) {
        let mut words = Words::new(&segment);
        protected.mask(&mut words.norm);
        for range in split_groups(&words.norm, units) {
            interpreter.group(&words.raw[range.clone()], &words.norm[range]);
        }
    }
    interpreter.flush_orphan();
    interpreter.flush_correction();

    ParsedText {
        jobs: interpreter.jobs,
        unparsed: interpreter.unparsed,
    }
}
