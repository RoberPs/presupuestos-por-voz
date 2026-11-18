//! Borrador completo y determinista (FR-014, SC-004 y SC-007).

use std::time::Instant;

use app_lib::domain::draft::builder::build_draft;
use app_lib::domain::draft::totals::compute_totals;
use app_lib::domain::draft::{BuiltDraft, DraftLine, PendingReason, PriceSource, QuantityIssue};
use app_lib::domain::rate::{RateView, UnitView};

const VAT_21: i64 = 2100;
const EXAMPLE: &str = "cambiar bajante, 3 metros, mano de obra 2 horas";

fn unit(id: i64, label: &str) -> UnitView {
    UnitView {
        id,
        label: label.into(),
        is_predefined: true,
    }
}

fn units() -> Vec<UnitView> {
    vec![
        unit(1, "unidad"),
        unit(2, "hora"),
        unit(3, "metro"),
        unit(4, "m²"),
        unit(5, "metro lineal"),
    ]
}

fn rate(id: i64, concept: &str, unit_label: &str, price_cents: i64) -> RateView {
    let found = units()
        .into_iter()
        .find(|u| u.label == unit_label)
        .expect("unidad de prueba");
    RateView::new(id, concept.into(), found, price_cents)
}

fn example_rates() -> Vec<RateView> {
    vec![
        rate(1, "Bajante", "metro", 2500),
        rate(2, "Mano de obra", "hora", 3000),
    ]
}

fn build(text: &str, rates: &[RateView]) -> BuiltDraft {
    build_draft(text, rates, &units(), VAT_21)
}

fn lines(draft: &BuiltDraft) -> Vec<DraftLine> {
    draft.lines.iter().map(|l| l.line.clone()).collect()
}

#[test]
fn the_spec_example_matches_the_expected_draft() {
    let draft = build(EXAMPLE, &example_rates());
    assert_eq!(draft.vat_rate_bp, VAT_21);
    assert_eq!(draft.lines.len(), 2);

    let first = &draft.lines[0].line;
    assert_eq!(first.position, 0);
    assert_eq!(first.concept, "Bajante");
    assert_eq!(first.quantity_centi, Some(300));
    assert_eq!(first.unit_label.as_deref(), Some("metro"));
    assert_eq!(first.price_cents, Some(2500));
    assert_eq!(first.price_source, PriceSource::Rate);
    assert_eq!(first.rate_id, Some(1));
    assert_eq!(first.rate_concept.as_deref(), Some("Bajante"));
    assert_eq!(first.amount_cents(), Ok(Some(7500)));
    assert_eq!(
        first.source_fragment.as_deref(),
        Some("cambiar bajante, 3 metros")
    );

    let second = &draft.lines[1].line;
    assert_eq!(second.position, 1);
    assert_eq!(second.concept, "Mano de obra");
    assert_eq!(second.amount_cents(), Ok(Some(6000)));

    let totals = compute_totals(&lines(&draft), VAT_21).unwrap();
    assert_eq!(totals.base_cents, 13_500);
    assert_eq!(totals.vat_cents, 2_835);
    assert_eq!(totals.total_cents, 16_335);
}

#[test]
fn a_missing_unit_takes_the_one_of_the_rate() {
    let draft = build("bajante 3", &example_rates());
    assert_eq!(draft.lines[0].line.unit_label.as_deref(), Some("metro"));
    assert_eq!(draft.lines[0].line.price_cents, Some(2500));
}

#[test]
fn a_concept_without_rate_keeps_what_was_dictated_and_has_no_price() {
    let draft = build("cambiar grifo, 2 unidades", &example_rates());
    let line = &draft.lines[0].line;
    assert_eq!(line.concept, "cambiar grifo");
    assert_eq!(line.quantity_centi, Some(200));
    assert_eq!(line.unit_label.as_deref(), Some("unidad"));
    assert_eq!(line.price_cents, None);
    assert_eq!(line.price_source, PriceSource::Pending);
    assert_eq!(line.pending_reason, Some(PendingReason::NoRate));
}

#[test]
fn suggestions_are_listed_for_a_similar_rate() {
    let rates = [rate(7, "Bajante de hierro", "metro", 4000)];
    let draft = build("tubo bajante de pvc, 3 metros", &rates);
    assert_eq!(draft.lines[0].line.price_cents, None);
    assert_eq!(
        draft.lines[0].line.pending_reason,
        Some(PendingReason::SimilarRate)
    );
    assert_eq!(draft.lines[0].suggestion_rate_ids, [7]);
}

#[test]
fn a_correction_keeps_the_rate_but_leaves_the_quantity_pending() {
    let draft = build("bajante, 3 metros, no, mejor 4", &example_rates());
    let line = &draft.lines[0].line;
    assert_eq!(line.quantity_centi, None);
    assert_eq!(line.quantity_issue, Some(QuantityIssue::Rectified));
    assert_eq!(line.amount_cents(), Ok(None));
    assert!(line.is_pending());
}

#[test]
fn without_rates_every_line_is_pending() {
    let draft = build(EXAMPLE, &[]);
    assert_eq!(draft.lines.len(), 2);
    assert!(draft
        .lines
        .iter()
        .all(|l| l.line.price_source == PriceSource::Pending && l.line.price_cents.is_none()));
}

#[test]
fn fragments_that_are_not_understood_are_kept() {
    let draft = build("bueno, bajante 3 metros", &example_rates());
    assert_eq!(draft.unparsed_fragments, ["bueno"]);
}

#[test]
fn the_same_text_always_gives_the_same_draft() {
    let rates = example_rates();
    let text = "cambiar bajante, 3 metros, mano de obra 2 horas, pintar techo, unos metros de tubo";
    let first = build(text, &rates);
    for _ in 0..100 {
        assert_eq!(build(text, &rates), first);
    }
}

#[test]
fn no_line_has_a_price_different_from_its_rate() {
    let rates = example_rates();
    let draft = build(
        "bajante 3 metros, mano de obra 2 horas, bajante 2 horas, tubo 4 metros, mano obra 1 hora",
        &rates,
    );
    for built in &draft.lines {
        let line = &built.line;
        match line.price_source {
            PriceSource::Rate => {
                let source = rates
                    .iter()
                    .find(|r| Some(r.id) == line.rate_id)
                    .expect("la línea debe proceder de una tarifa");
                assert_eq!(line.price_cents, Some(source.price_cents));
            }
            PriceSource::Pending => assert_eq!(line.price_cents, None),
            PriceSource::Manual => panic!("el intérprete nunca crea precios manuales"),
        }
    }
}

#[test]
fn five_hundred_rates_are_processed_quickly() {
    let rates: Vec<RateView> = (1..=500)
        .map(|id| rate(id, &format!("Concepto número {id} de obra"), "metro", 1000))
        .collect();
    let started = Instant::now();
    let draft = build(EXAMPLE, &rates);
    let elapsed = started.elapsed();
    assert_eq!(draft.lines.len(), 2);
    assert!(elapsed.as_millis() < 100, "ha tardado {elapsed:?}");
}
