//! Mide la generación del PDF del presupuesto para SC-002 (tarea T059).
//!
//! Uso:
//!   cargo run --release --no-default-features --example quote_pdf_benchmark -- [líneas] [repeticiones]
//!
//! Por defecto, 50 líneas y 5 repeticiones. Trabaja en carpetas temporales: no toca la base de
//! datos real de la app. Cada repetición cronometra `generate_to_path`, que es lo que mide SC-002
//! ("desde que se elige la ubicación"): maquetación, dibujo con krilla, escritura atómica y
//! numeración. El diálogo nativo de guardado queda fuera.

use std::time::{Duration, Instant};

use app_lib::domain::draft::TextOrigin;
use app_lib::domain::profile::{Address, BusinessProfileInput};
use app_lib::domain::rate::{RateInput, MAX_CONCEPT_CHARS};
use app_lib::services::draft_service::{self as draft, DraftLineInput};
use app_lib::services::{logo_service, profile_service, quote_service as quote, rate_service};
use app_lib::state::AppState;
use app_lib::storage::{db, draft_repo};
use image::{ImageBuffer, ImageFormat, Rgba};
use time::{Date, Month};

const LIMIT: Duration = Duration::from_secs(5);

fn main() {
    let mut args = std::env::args().skip(1);
    let lines: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(50);
    let runs: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(5);
    let today = Date::from_calendar_date(2026, Month::November, 19).unwrap();

    let data = tempfile::tempdir().expect("carpeta temporal de datos");
    let output = tempfile::tempdir().expect("carpeta temporal de salida");
    let state = AppState::new(
        db::open(data.path()).expect("base de datos"),
        data.path().into(),
    );

    profile_service::save_profile(&state, profile()).expect("perfil");
    let logo = data.path().join("logo-origen.png");
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(1200, 600, |x, y| {
        Rgba([
            (x % 256) as u8,
            (y % 256) as u8,
            160,
            if (x / 40 + y / 40) % 2 == 0 { 0 } else { 255 },
        ])
    });
    img.save_with_format(&logo, ImageFormat::Png).expect("logo");
    logo_service::set_logo(&state, &logo).expect("asignar logo");

    // Borrador sin líneas y después una línea por tarifa, con conceptos de longitud máxima.
    empty_draft(&state);
    for i in 1..=lines {
        let concept = long_concept(i);
        let rate = rate_service::create(
            &state,
            RateInput {
                concept,
                unit_label: ["metro", "hora", "unidad", "m²"][i % 4].into(),
                price: format!("{},{:02}", 10 + (i * 7) % 290, (i * 13) % 100),
            },
        )
        .expect("tarifa");
        draft::line_add(
            &state,
            DraftLineInput::Rate {
                rate_id: rate.id,
                quantity: format!("{}", 1 + i % 9),
            },
        )
        .expect("línea");
    }
    let view = draft::mark_reviewed(&state).expect("dar por bueno");
    assert_eq!(
        view.totals.pending_count, 0,
        "no debe haber líneas pendientes"
    );
    println!(
        "Borrador: {} líneas, total {}",
        view.lines.len(),
        view.totals.total_display
    );

    let mut details = quote::prepare_on(&state, today, false)
        .expect("preparar")
        .details;
    details.client_name = "Cliente de prueba".into();
    quote::save_details_on(&state, details, today, false).expect("datos del presupuesto");

    let mut times = Vec::with_capacity(runs);
    for run in 1..=runs {
        let token = quote::prepare_on(&state, today, false)
            .expect("preparar")
            .confirmation_token
            .expect("debe haber token");
        let path = output.path().join(format!("benchmark-{run}.pdf"));
        let start = Instant::now();
        let saved =
            quote::generate_to_path_on(&state, &token, false, &path, today).expect("generar");
        let elapsed = start.elapsed();
        let size = std::fs::metadata(&saved.full_path)
            .map(|m| m.len())
            .unwrap_or(0);
        println!(
            "Repetición {run}: {} ms · {} páginas · {} KB",
            elapsed.as_millis(),
            saved.page_count,
            size / 1024
        );
        times.push(elapsed);
    }

    times.sort();
    let median = times[times.len() / 2];
    let worst = *times.last().unwrap();
    println!(
        "Mediana: {} ms · Peor: {} ms · Límite SC-002: {} ms → {}",
        median.as_millis(),
        worst.as_millis(),
        LIMIT.as_millis(),
        if worst < LIMIT { "CUMPLE" } else { "NO CUMPLE" }
    );
}

/// Crea un borrador desde un texto cualquiera y quita sus líneas.
fn empty_draft(state: &AppState) {
    let text = "desplazamiento";
    {
        let mut conn = state.conn().expect("conexión");
        let tx = conn.transaction().expect("transacción");
        let built = draft::build_from_text(&tx, text).expect("borrador");
        draft_repo::replace(&tx, &built, text, TextOrigin::Typed).expect("guardar borrador");
        tx.commit().expect("confirmar");
    }
    let conn = state.conn().expect("conexión");
    let ids: Vec<i64> = draft::load_view(&conn)
        .expect("leer borrador")
        .map(|v| v.lines.iter().map(|l| l.id).collect())
        .unwrap_or_default();
    drop(conn);
    for id in ids {
        draft::line_delete(state, id).expect("quitar línea");
    }
}

/// Concepto único de exactamente `MAX_CONCEPT_CHARS` caracteres, con tildes y eñes.
fn long_concept(i: usize) -> String {
    let base = format!(
        "Partida {i:03}: sustitución de tubería de cobre por multicapa en baño y cocina, \
         incluida la retirada de azulejos, el montaje de codos y la prueba de presión final "
    );
    base.chars()
        .cycle()
        .take(MAX_CONCEPT_CHARS)
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn profile() -> BusinessProfileInput {
    BusinessProfileInput {
        trade_name: "Fontanería de prueba".into(),
        legal_name: Some("Titular de prueba".into()),
        tax_id: "12345678Z".into(),
        phone: "600 000 000".into(),
        email: Some("prueba@example.com".into()),
        website: None,
        address: Address {
            street: "Calle Mayor 1".into(),
            postal_code: "28001".into(),
            city: "Madrid".into(),
            province: "Madrid".into(),
        },
        vat_rate_percent: "21".into(),
    }
}
