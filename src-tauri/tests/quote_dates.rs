//! Fecha de emisión y fecha de validez (FR-014 y FR-014b).

use app_lib::domain::quote_doc::dates::{format_date, format_iso, valid_until};
use time::{Date, Month};

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}

#[test]
fn thirty_days_from_the_spec_example() {
    assert_eq!(
        valid_until(date(2026, Month::November, 18), 30),
        date(2026, Month::December, 18)
    );
}

#[test]
fn crosses_the_end_of_the_month() {
    assert_eq!(
        valid_until(date(2026, Month::January, 31), 1),
        date(2026, Month::February, 1)
    );
}

#[test]
fn crosses_the_end_of_the_year() {
    assert_eq!(
        valid_until(date(2026, Month::December, 15), 30),
        date(2027, Month::January, 14)
    );
}

#[test]
fn handles_the_29th_of_february() {
    assert_eq!(
        valid_until(date(2028, Month::February, 28), 1),
        date(2028, Month::February, 29)
    );
    assert_eq!(
        valid_until(date(2028, Month::February, 29), 1),
        date(2028, Month::March, 1)
    );
    assert_eq!(
        valid_until(date(2027, Month::February, 28), 1),
        date(2027, Month::March, 1)
    );
}

#[test]
fn accepts_the_limits_of_the_validity() {
    let issue = date(2026, Month::November, 18);
    assert_eq!(valid_until(issue, 1), date(2026, Month::November, 19));
    assert_eq!(valid_until(issue, 365), date(2027, Month::November, 18));
}

#[test]
fn dates_are_shown_as_day_month_year() {
    assert_eq!(format_date(date(2026, Month::December, 18)), "18/12/2026");
    assert_eq!(format_date(date(2026, Month::March, 5)), "05/03/2026");
}

#[test]
fn file_names_use_the_iso_format() {
    assert_eq!(format_iso(date(2026, Month::March, 5)), "2026-03-05");
}
