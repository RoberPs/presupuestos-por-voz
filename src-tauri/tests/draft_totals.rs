//! Base, IVA y total del borrador (FR-028, FR-029 y FR-031).

use app_lib::domain::draft::totals::compute_totals;
use app_lib::domain::draft::{DraftLine, PendingReason, PriceSource, QuantityIssue};
use app_lib::error::ErrorCode;

const VAT_21: i64 = 2100;

fn priced(concept: &str, quantity_centi: i64, unit: &str, price_cents: i64) -> DraftLine {
    DraftLine {
        id: 0,
        position: 0,
        concept: concept.into(),
        quantity_centi: Some(quantity_centi),
        quantity_issue: None,
        unit_label: Some(unit.into()),
        price_cents: Some(price_cents),
        price_source: PriceSource::Rate,
        rate_id: Some(1),
        rate_concept: Some(concept.into()),
        pending_reason: None,
        source_fragment: None,
    }
}

fn price_pending(concept: &str, quantity_centi: i64) -> DraftLine {
    DraftLine {
        price_cents: None,
        price_source: PriceSource::Pending,
        rate_id: None,
        rate_concept: None,
        pending_reason: Some(PendingReason::NoRate),
        unit_label: None,
        ..priced(concept, quantity_centi, "metro", 1)
    }
}

fn quantity_pending(concept: &str, price_cents: i64) -> DraftLine {
    DraftLine {
        quantity_centi: None,
        quantity_issue: Some(QuantityIssue::Ambiguous),
        ..priced(concept, 100, "metro", price_cents)
    }
}

#[test]
fn spec_example_with_three_metres() {
    let lines = [
        priced("Bajante", 300, "metro", 2500),
        priced("Mano de obra", 200, "hora", 3000),
    ];
    let totals = compute_totals(&lines, VAT_21).unwrap();
    assert_eq!(totals.base_cents, 13_500);
    assert_eq!(totals.vat_cents, 2_835);
    assert_eq!(totals.total_cents, 16_335);
    assert_eq!(totals.pending_count, 0);
    assert!(!totals.is_provisional);
}

#[test]
fn spec_example_with_four_metres() {
    let lines = [
        priced("Bajante", 400, "metro", 2500),
        priced("Mano de obra", 200, "hora", 3000),
    ];
    let totals = compute_totals(&lines, VAT_21).unwrap();
    assert_eq!(totals.base_cents, 16_000);
    assert_eq!(totals.vat_cents, 3_360);
    assert_eq!(totals.total_cents, 19_360);
}

#[test]
fn pending_lines_stay_out_of_the_total() {
    let lines = [
        priced("Bajante", 300, "metro", 2500),
        price_pending("Canalón", 200),
        priced("Mano de obra", 200, "hora", 3000),
        quantity_pending("Codo", 1200),
    ];
    let totals = compute_totals(&lines, VAT_21).unwrap();
    assert_eq!(totals.base_cents, 13_500);
    assert_eq!(totals.vat_cents, 2_835);
    assert_eq!(totals.total_cents, 16_335);
    assert_eq!(totals.pending_count, 2);
    assert!(totals.is_provisional);
}

#[test]
fn an_empty_draft_totals_zero() {
    let totals = compute_totals(&[], VAT_21).unwrap();
    assert_eq!(totals.base_cents, 0);
    assert_eq!(totals.vat_cents, 0);
    assert_eq!(totals.total_cents, 0);
    assert_eq!(totals.pending_count, 0);
    assert!(!totals.is_provisional);
}

#[test]
fn a_total_that_does_not_fit_is_rejected() {
    let lines = [
        priced("Uno", 100, "unidad", i64::MAX),
        priced("Dos", 100, "unidad", i64::MAX),
    ];
    assert_eq!(
        compute_totals(&lines, VAT_21),
        Err(ErrorCode::AmountTooLarge)
    );
}
