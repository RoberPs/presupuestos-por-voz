//! Numeración de los presupuestos (FR-013, FR-013b y FR-013c).

use app_lib::domain::quote_doc::numbering::{
    content_hash, format_number, parse_series_number, propose, ContentLine, Proposal,
};
use app_lib::storage::{db, quote_repo};

fn proposal<'a>(last_number: i64) -> Proposal<'a> {
    Proposal {
        manual: None,
        issued_number: None,
        issued_hash: None,
        current_hash: "huella-actual",
        year: 2026,
        last_number,
    }
}

#[test]
fn the_first_number_of_the_year_is_001() {
    assert_eq!(propose(&proposal(0)), "2026-001");
}

#[test]
fn the_next_number_follows_the_last_one() {
    assert_eq!(propose(&proposal(7)), "2026-008");
}

#[test]
fn the_series_continues_past_999() {
    assert_eq!(propose(&proposal(999)), "2026-1000");
}

#[test]
fn numbers_have_at_least_three_digits() {
    assert_eq!(format_number(2026, 1), "2026-001");
    assert_eq!(format_number(2026, 42), "2026-042");
    assert_eq!(format_number(2026, 1234), "2026-1234");
}

#[test]
fn only_the_year_number_format_belongs_to_the_series() {
    assert_eq!(parse_series_number("2026-050"), Some((2026, 50)));
    assert_eq!(parse_series_number("2027-1000"), Some((2027, 1000)));
    assert_eq!(
        parse_series_number("2026-999999999"),
        Some((2026, 999_999_999))
    );

    for other in [
        "2026-07",
        "OBRA-7",
        "2026-",
        "-001",
        "26-001",
        "2026-0000000001",
        "2026/001",
        " 2026-001",
        "0000-001",
        "",
    ] {
        assert_eq!(parse_series_number(other), None, "«{other}»");
    }
}

#[test]
fn a_handwritten_number_comes_first() {
    let input = Proposal {
        manual: Some("OBRA-7"),
        issued_number: Some("2026-003"),
        issued_hash: Some("huella-actual"),
        ..proposal(9)
    };
    assert_eq!(propose(&input), "OBRA-7");
}

#[test]
fn an_erased_number_stays_empty() {
    let input = Proposal {
        manual: Some(""),
        ..proposal(9)
    };
    assert_eq!(propose(&input), "");
}

#[test]
fn the_issued_number_is_reused_while_the_content_is_the_same() {
    let input = Proposal {
        issued_number: Some("2026-003"),
        issued_hash: Some("huella-actual"),
        ..proposal(9)
    };
    assert_eq!(propose(&input), "2026-003");
}

#[test]
fn a_different_content_gets_the_next_number() {
    let input = Proposal {
        issued_number: Some("2026-003"),
        issued_hash: Some("otra-huella"),
        ..proposal(9)
    };
    assert_eq!(propose(&input), "2026-010");
}

fn line(concept: &str, quantity: i64, price: i64) -> ContentLine<'_> {
    ContentLine {
        concept,
        quantity_centi: quantity,
        unit: "metro",
        price_cents: price,
    }
}

#[test]
fn the_content_hash_depends_on_lines_order_and_vat() {
    let base = content_hash(&[line("A", 100, 500), line("B", 200, 700)], 2100);
    assert_eq!(
        base,
        content_hash(&[line("A", 100, 500), line("B", 200, 700)], 2100)
    );
    assert_ne!(
        base,
        content_hash(&[line("B", 200, 700), line("A", 100, 500)], 2100)
    );
    assert_ne!(
        base,
        content_hash(&[line("A", 150, 500), line("B", 200, 700)], 2100)
    );
    assert_ne!(
        base,
        content_hash(&[line("A", 100, 501), line("B", 200, 700)], 2100)
    );
    assert_ne!(
        base,
        content_hash(&[line("A", 100, 500), line("B", 200, 700)], 1000)
    );
}

#[test]
fn the_content_hash_does_not_mix_adjacent_fields() {
    assert_ne!(
        content_hash(&[line("AB", 100, 500), line("C", 100, 500)], 2100),
        content_hash(&[line("A", 100, 500), line("BC", 100, 500)], 2100)
    );
}

#[test]
fn the_series_keeps_the_highest_number_used() {
    let conn = db::open_in_memory().unwrap();
    assert_eq!(quote_repo::last_number(&conn, 2026).unwrap(), 0);

    quote_repo::record_number(&conn, 2026, 50).unwrap();
    assert_eq!(quote_repo::last_number(&conn, 2026).unwrap(), 50);

    quote_repo::record_number(&conn, 2026, 7).unwrap();
    assert_eq!(quote_repo::last_number(&conn, 2026).unwrap(), 50);

    assert_eq!(quote_repo::last_number(&conn, 2027).unwrap(), 0);
}
