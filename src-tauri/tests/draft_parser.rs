//! Del texto de la visita a los trabajos (FR-015, FR-020 y FR-022; research.md, R6).

use app_lib::domain::draft::parser::{parse_text, ParsedJob, ParsedText};
use app_lib::domain::draft::units::UnitCatalog;
use app_lib::domain::draft::QuantityIssue;

fn parse(text: &str) -> ParsedText {
    parse_text(text, &UnitCatalog::default())
}

fn summary(job: &ParsedJob) -> (&str, Option<i64>, Option<&str>) {
    (
        job.concept.as_str(),
        job.quantity_centi,
        job.unit.as_deref(),
    )
}

#[test]
fn spec_example_gives_two_jobs() {
    let parsed = parse("cambiar bajante, 3 metros, mano de obra 2 horas");
    assert_eq!(parsed.jobs.len(), 2);
    assert_eq!(
        summary(&parsed.jobs[0]),
        ("cambiar bajante", Some(300), Some("metro"))
    );
    assert_eq!(
        summary(&parsed.jobs[1]),
        ("mano de obra", Some(200), Some("hora"))
    );
    assert!(parsed.unparsed.is_empty());
}

#[test]
fn the_fragment_keeps_what_was_dictated() {
    let parsed = parse("cambiar bajante, 3 metros");
    assert_eq!(parsed.jobs[0].fragment, "cambiar bajante, 3 metros");
}

#[test]
fn supported_patterns() {
    let table = [
        ("bajante 3 metros", "bajante", Some(300), Some("metro")),
        ("3 metros de bajante", "bajante", Some(300), Some("metro")),
        ("2 grifos", "grifos", Some(200), None),
        (
            "mano de obra, hora y media",
            "mano de obra",
            Some(150),
            Some("hora"),
        ),
        (
            "tubo 2 metros lineales",
            "tubo",
            Some(200),
            Some("metro lineal"),
        ),
        (
            "alicatado 4 metros cuadrados",
            "alicatado",
            Some(400),
            Some("m²"),
        ),
    ];
    for (text, concept, quantity, unit) in table {
        let parsed = parse(text);
        assert_eq!(parsed.jobs.len(), 1, "«{text}»");
        assert_eq!(
            summary(&parsed.jobs[0]),
            (concept, quantity, unit),
            "«{text}»"
        );
    }
}

#[test]
fn mixed_jobs_keep_their_order_without_commas() {
    let parsed = parse("bajante 3 metros mano de obra 2 horas grifo 1 unidad");
    let concepts: Vec<&str> = parsed.jobs.iter().map(|j| j.concept.as_str()).collect();
    assert_eq!(concepts, ["bajante", "mano de obra", "grifo"]);
}

#[test]
fn a_repeated_concept_gives_two_lines() {
    let parsed = parse("bajante 3 metros. Bajante 2 metros.");
    assert_eq!(parsed.jobs.len(), 2);
    assert_eq!(parsed.jobs[0].quantity_centi, Some(300));
    assert_eq!(parsed.jobs[1].quantity_centi, Some(200));
}

#[test]
fn a_missing_quantity_stays_pending() {
    let parsed = parse("revisar el calentador");
    assert_eq!(parsed.jobs.len(), 1);
    assert_eq!(parsed.jobs[0].quantity_centi, None);
    assert_eq!(parsed.jobs[0].quantity_issue, Some(QuantityIssue::Missing));
}

#[test]
fn vague_quantities_are_ambiguous() {
    for text in [
        "unos cuantos metros de tubo",
        "varios metros de tubo",
        "aproximadamente 3 metros de tubo",
        "más o menos 3 metros de tubo",
    ] {
        let parsed = parse(text);
        assert_eq!(parsed.jobs.len(), 1, "«{text}»");
        let job = &parsed.jobs[0];
        assert_eq!(job.concept, "tubo", "«{text}»");
        assert_eq!(job.quantity_centi, None, "«{text}»");
        assert_eq!(
            job.quantity_issue,
            Some(QuantityIssue::Ambiguous),
            "«{text}»"
        );
        assert_eq!(job.unit.as_deref(), Some("metro"), "«{text}»");
    }
}

#[test]
fn a_correction_while_dictating_leaves_the_quantity_pending() {
    for text in [
        "bajante, 3 metros, no, mejor 4",
        "bajante 3 metros no mejor 4",
        "bajante 3 metros, perdón, 4 metros",
        "bajante 3 metros, digo, 4",
        "bajante 3 metros, mejor dicho, 4",
        "bajante 3 metros, o sea, 4",
    ] {
        let parsed = parse(text);
        assert_eq!(parsed.jobs.len(), 1, "«{text}»");
        let job = &parsed.jobs[0];
        assert_eq!(job.concept, "bajante", "«{text}»");
        assert_eq!(job.quantity_centi, None, "«{text}»");
        assert_eq!(
            job.quantity_issue,
            Some(QuantityIssue::Rectified),
            "«{text}»"
        );
        assert!(job.fragment.contains('4'), "«{text}»: {}", job.fragment);
        assert!(job.fragment.contains('3'), "«{text}»: {}", job.fragment);
    }
}

#[test]
fn a_correction_before_the_concept_keeps_the_concept() {
    let parsed = parse("3 metros, no, mejor 4 de bajante");
    assert_eq!(parsed.jobs.len(), 1);
    assert_eq!(parsed.jobs[0].concept, "bajante");
    assert_eq!(
        parsed.jobs[0].quantity_issue,
        Some(QuantityIssue::Rectified)
    );
    assert!(parsed.jobs[0].fragment.contains("3 metros"));
    assert!(parsed.unparsed.is_empty());
}

#[test]
fn what_cannot_be_understood_is_kept_apart() {
    let parsed = parse("bueno, bajante 3 metros, 12 metros");
    assert_eq!(parsed.jobs.len(), 1);
    assert_eq!(parsed.unparsed, ["bueno", "12 metros"]);
}

#[test]
fn an_empty_text_gives_nothing() {
    let parsed = parse("  \n ");
    assert!(parsed.jobs.is_empty());
    assert!(parsed.unparsed.is_empty());
}
