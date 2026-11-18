//! Coincidencia de lo dictado con las tarifas (FR-017 a FR-021 y FR-023; research.md, R7).

use app_lib::domain::draft::matcher::{match_job, RateMatch};
use app_lib::domain::draft::units::UnitCatalog;
use app_lib::domain::draft::PendingReason;
use app_lib::domain::rate::{RateView, UnitView};

fn rate(id: i64, concept: &str, unit: &str) -> RateView {
    RateView::new(
        id,
        concept.into(),
        UnitView {
            id: 1,
            label: unit.into(),
            is_predefined: true,
        },
        1000,
    )
}

fn matched(concept: &str, unit: Option<&str>, rates: &[RateView]) -> RateMatch {
    match_job(concept, unit, rates, &UnitCatalog::default())
}

fn pending(reason: PendingReason, suggestions: &[i64]) -> RateMatch {
    RateMatch::Pending {
        reason,
        suggestions: suggestions.to_vec(),
    }
}

#[test]
fn all_the_words_of_a_single_rate_apply_it() {
    let rates = [rate(1, "Bajante", "metro")];
    assert_eq!(
        matched("cambiar bajante", Some("metro"), &rates),
        RateMatch::Applied(1)
    );
}

#[test]
fn capitals_accents_and_plurals_do_not_matter() {
    let rates = [
        rate(1, "Instalación de grifo", "unidad"),
        rate(2, "Luz de emergencia", "unidad"),
    ];
    assert_eq!(
        matched("INSTALACION de grifos", None, &rates),
        RateMatch::Applied(1)
    );
    assert_eq!(
        matched("luces de emergencia", None, &rates),
        RateMatch::Applied(2)
    );
}

#[test]
fn every_word_counts_even_the_short_ones() {
    let rates = [rate(1, "Mano de obra", "hora")];
    assert_eq!(
        matched("mano obra", Some("hora"), &rates),
        pending(PendingReason::SimilarRate, &[1])
    );
}

#[test]
fn without_a_dictated_unit_a_single_rate_is_applied() {
    let rates = [rate(1, "Bajante", "metro")];
    assert_eq!(matched("bajante", None, &rates), RateMatch::Applied(1));
}

#[test]
fn several_rates_without_a_unit_stay_pending_to_choose() {
    let rates = [rate(1, "Pintura", "metro"), rate(2, "Pintura", "hora")];
    assert_eq!(
        matched("pintura", None, &rates),
        pending(PendingReason::AmbiguousRate, &[1, 2])
    );
}

#[test]
fn the_dictated_unit_breaks_the_tie() {
    let rates = [rate(1, "Pintura", "metro"), rate(2, "Pintura", "hora")];
    assert_eq!(
        matched("pintura", Some("hora"), &rates),
        RateMatch::Applied(2)
    );
}

#[test]
fn a_different_unit_is_never_converted() {
    let rates = [rate(1, "Bajante", "metro")];
    assert_eq!(
        matched("bajante", Some("hora"), &rates),
        pending(PendingReason::UnitMismatch, &[1])
    );
}

#[test]
fn several_rates_with_another_unit_are_only_suggested() {
    let rates = [rate(1, "Pintura", "metro"), rate(2, "Pintura", "hora")];
    assert_eq!(
        matched("pintura", Some("unidad"), &rates),
        pending(PendingReason::UnitMismatch, &[1, 2])
    );
}

#[test]
fn a_similar_rate_is_suggested_without_applying_it() {
    let rates = [rate(1, "Bajante de hierro", "metro")];
    assert_eq!(
        matched("tubo bajante de pvc", Some("metro"), &rates),
        pending(PendingReason::SimilarRate, &[1])
    );
}

#[test]
fn a_word_with_a_small_typo_is_only_similar() {
    let rates = [rate(1, "Bajante", "metro")];
    assert_eq!(
        matched("bajanta", Some("metro"), &rates),
        pending(PendingReason::SimilarRate, &[1])
    );
}

#[test]
fn nothing_similar_gives_no_suggestions() {
    let rates = [rate(1, "Bajante", "metro")];
    assert_eq!(
        matched("pintar techo", None, &rates),
        pending(PendingReason::NoRate, &[])
    );
}

#[test]
fn without_rates_the_line_stays_pending() {
    assert_eq!(
        matched("bajante", Some("metro"), &[]),
        pending(PendingReason::NoRate, &[])
    );
}

#[test]
fn at_most_five_suggestions_in_a_stable_order() {
    let rates: Vec<RateView> = (1..=8)
        .rev()
        .map(|id| rate(id, &format!("Bajante tipo{id}"), "metro"))
        .collect();
    assert_eq!(
        matched("bajante", Some("metro"), &rates),
        pending(PendingReason::SimilarRate, &[1, 2, 3, 4, 5])
    );
}

#[test]
fn the_more_alike_suggestions_come_first() {
    let rates = [
        rate(1, "Tubo de hierro", "metro"),
        rate(2, "Tubo de cobre grande", "metro"),
    ];
    assert_eq!(
        matched("tubo cobre", Some("metro"), &rates),
        pending(PendingReason::SimilarRate, &[2, 1])
    );
}

#[test]
fn a_shorter_rate_does_not_steal_the_longer_one() {
    let rates = [
        rate(1, "Mano de obra", "hora"),
        rate(2, "Mano de obra urgente", "hora"),
    ];
    assert_eq!(
        matched("mano de obra", Some("hora"), &rates),
        RateMatch::Applied(1)
    );
    assert_eq!(
        matched("mano de obra urgente", Some("hora"), &rates),
        pending(PendingReason::AmbiguousRate, &[1, 2])
    );
}
