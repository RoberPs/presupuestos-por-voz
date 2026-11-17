mod common;

use std::time::{Duration, Instant};

use app_lib::domain::normalize::normalize;
use app_lib::domain::rate::RateInput;
use app_lib::services::rate_service;
use app_lib::state::AppState;
use common::*;
use rusqlite::params;

fn add(state: &AppState, concept: &str, unit: &str) {
    rate_service::create(
        state,
        RateInput { concept: concept.into(), unit_label: unit.into(), price: "10".into() },
    )
    .unwrap();
}

fn concepts(state: &AppState, q: &str) -> Vec<String> {
    rate_service::list(state, Some(q.into())).unwrap().into_iter().map(|r| r.concept).collect()
}

#[test]
fn search_ignores_case_and_accents() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    add(&state, "Instalación de grifo", "unidad");
    add(&state, "Cambio de grifo", "unidad");
    add(&state, "Pintura de pared", "m²");

    assert_eq!(concepts(&state, "grifo"), vec!["Cambio de grifo", "Instalación de grifo"]);
    assert_eq!(concepts(&state, "GRIFO").len(), 2);
    assert_eq!(concepts(&state, "instalacion"), vec!["Instalación de grifo"]);
    assert_eq!(concepts(&state, "  INSTALACIÓN  de ").len(), 1);
    assert!(concepts(&state, "zzz").is_empty());
}

#[test]
fn empty_query_returns_everything() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    add(&state, "A", "unidad");
    add(&state, "B", "unidad");
    assert_eq!(concepts(&state, "").len(), 2);
    assert_eq!(concepts(&state, "   ").len(), 2);
    assert_eq!(rate_service::list(&state, None).unwrap().len(), 2);
}

#[test]
fn like_wildcards_are_literal() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    add(&state, "Descuento 10% material", "unidad");
    add(&state, "Tubo_cobre", "metro");
    add(&state, "Tubo cobre", "metro");

    assert_eq!(concepts(&state, "%"), vec!["Descuento 10% material"]);
    assert_eq!(concepts(&state, "_"), vec!["Tubo_cobre"]);
    assert_eq!(concepts(&state, "o_c"), vec!["Tubo_cobre"]);
    assert_eq!(concepts(&state, "\\").len(), 0);
}

#[test]
fn rates_search_500() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    {
        let mut conn = state.conn().unwrap();
        let tx = conn.transaction().unwrap();
        for i in 0..500 {
            let concept = if i % 5 == 0 {
                format!("Tubo cobre {i} mm")
            } else {
                format!("Trabajo número {i}")
            };
            tx.execute(
                "INSERT INTO rates (concept, concept_norm, unit_id, price_cents) VALUES (?1, ?2, ?3, ?4)",
                params![concept, normalize(&concept), (i % 5) + 1, 1000 + i],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    // Calentamiento (prepara la sentencia en caché)
    rate_service::list(&state, Some("tubo".into())).unwrap();

    let start = Instant::now();
    let result = rate_service::list(&state, Some("tubo".into())).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(result.len(), 100);
    let limit = if cfg!(debug_assertions) { Duration::from_millis(250) } else { Duration::from_millis(50) };
    assert!(elapsed < limit, "la búsqueda ha tardado {elapsed:?} (límite {limit:?})");
}
