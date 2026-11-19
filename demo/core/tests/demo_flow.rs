//! Recorrido completo de la demo sobre los datos de ejemplo, sin navegador.

use presupuestos_demo_core::dispatch;
use presupuestos_demo_core::domain::draft::{PendingReason, PriceSource};
use presupuestos_demo_core::error::{AppError, QuoteBlocker};
use presupuestos_demo_core::store::{GenerateMode, Store, EXAMPLE_TEXT};
use serde_json::{json, Value};
use time::{Date, Month};

fn today() -> Date {
    Date::from_calendar_date(2026, Month::November, 19).unwrap()
}

fn call(store: &mut Store, command: &str, args: Value) -> Value {
    let json = dispatch(store, command, &args, today()).expect(command);
    serde_json::from_str(&json).unwrap()
}

#[test]
fn the_example_text_prices_known_rates_and_leaves_the_rest_pending() {
    let mut store = Store::demo();
    let session = store.session().unwrap();
    assert_eq!(session.pending_text.as_deref(), Some(EXAMPLE_TEXT));

    let view = store
        .generate_from_text(EXAMPLE_TEXT, GenerateMode::New, false)
        .unwrap();
    let priced: Vec<(&str, &str)> = view
        .lines
        .iter()
        .filter(|l| l.price_source == PriceSource::Rate)
        .map(|l| (l.concept.as_str(), l.amount_display.as_deref().unwrap()))
        .collect();
    assert_eq!(
        priced,
        [
            ("Bajante", "75,00 €"),
            ("Grifo monomando", "45,00 €"),
            ("Mano de obra", "60,00 €"),
        ]
    );
    let pending: Vec<_> = view.lines.iter().filter(|l| l.price_cents.is_none()).collect();
    assert_eq!(pending.len(), 1, "la caldera no tiene tarifa");
    assert_eq!(pending[0].pending_reason, Some(PendingReason::NoRate));
    assert_eq!(view.totals.base_display, "180,00 €");
    assert_eq!(view.totals.pending_count, 1);
    assert!(store.session().unwrap().pending_text.is_none());
}

#[test]
fn the_whole_flow_produces_a_pdf_and_spends_the_number() {
    let mut store = Store::demo();
    let view = store
        .generate_from_text(EXAMPLE_TEXT, GenerateMode::New, false)
        .unwrap();
    assert!(matches!(
        store.mark_reviewed(),
        Err(AppError::DraftHasPending { .. })
    ));

    // Completar la línea pendiente con un precio escrito a mano, por JSON como la interfaz.
    let pending_id = view.lines.iter().find(|l| l.price_cents.is_none()).unwrap().id;
    let updated = call(
        &mut store,
        "draft_line_update",
        json!({ "lineId": pending_id, "patch": { "quantity": "1", "unitLabel": "unidad", "price": "60" } }),
    );
    assert_eq!(updated["totals"]["totalDisplay"], "290,40 €");
    call(&mut store, "draft_mark_reviewed", json!({}));

    let prepared = call(&mut store, "quote_prepare", json!({ "allowMissingLogo": false }));
    assert_eq!(prepared["details"]["quoteNumber"], "2026-001");
    assert_eq!(prepared["blockers"], json!([{ "kind": "clientNameRequired" }]));
    assert_eq!(prepared["warnings"], json!([{ "kind": "noLogo" }]));
    assert!(prepared["confirmationToken"].is_null());

    let mut details = prepared["details"].clone();
    details["clientName"] = json!("Ana Ruiz");
    let saved = call(
        &mut store,
        "quote_details_save",
        json!({ "details": details, "allowMissingLogo": false }),
    );
    let token = saved["confirmationToken"].as_str().unwrap().to_string();

    let pdf = store.quote_generate(&token, today(), false).unwrap();
    assert_eq!(pdf.file_name, "Presupuesto 2026-001 2026-11-19.pdf");
    assert_eq!(pdf.rendered.page_count, 1);
    assert!(pdf.rendered.bytes.starts_with(b"%PDF"));

    // Sin cambios se propone el mismo número; tras cambiar una línea, el siguiente.
    let again = call(&mut store, "quote_prepare", json!({ "allowMissingLogo": false }));
    assert_eq!(again["details"]["quoteNumber"], "2026-001");
    call(
        &mut store,
        "draft_line_update",
        json!({ "lineId": pending_id, "patch": { "price": "65" } }),
    );
    let changed = call(&mut store, "quote_prepare", json!({ "allowMissingLogo": false }));
    assert_eq!(changed["details"]["quoteNumber"], "2026-002");
    assert!(changed["blockers"]
        .as_array()
        .unwrap()
        .contains(&json!({ "kind": "notReviewed" })));
}

#[test]
fn an_outdated_token_is_rejected() {
    let mut store = Store::demo();
    store
        .generate_from_text("cambiar bajante, 3 metros", GenerateMode::New, false)
        .unwrap();
    store.mark_reviewed().unwrap();
    let mut details = store.quote_prepare(today(), false).unwrap().details;
    details.client_name = "Ana Ruiz".into();
    let token = store
        .quote_details_save(details.clone(), today(), false)
        .unwrap()
        .confirmation_token
        .unwrap();
    details.client_name = "Otra persona".into();
    store.quote_details_save(details, today(), false).unwrap();
    assert!(matches!(
        store.quote_generate(&token, today(), false),
        Err(AppError::QuoteChanged { .. })
    ));
}

#[test]
fn rates_keep_the_rules_of_the_app() {
    let mut store = Store::demo();
    let duplicate = dispatch(
        &mut store,
        "rates_create",
        &json!({ "input": { "concept": "  bajante ", "unitLabel": "Metro", "price": "30" } }),
        today(),
    );
    assert!(duplicate.is_err());
    let found = call(&mut store, "rates_list", json!({ "query": "PLASTICA" }));
    assert_eq!(found[0]["concept"], "Pintura plástica");
    let custom = call(
        &mut store,
        "rates_create",
        json!({ "input": { "concept": "Termo eléctrico", "unitLabel": "pieza", "price": "180" } }),
    );
    assert_eq!(custom["unit"]["isPredefined"], false);
    let units = call(&mut store, "units_list", json!({}));
    assert_eq!(units.as_array().unwrap().last().unwrap()["label"], "pieza");
}

#[test]
fn a_missing_client_name_blocks_generation() {
    let mut store = Store::demo();
    store
        .generate_from_text("mano de obra 2 horas", GenerateMode::New, false)
        .unwrap();
    store.mark_reviewed().unwrap();
    let result = store.quote_generate("cualquiera", today(), false);
    match result {
        Err(AppError::QuoteBlocked { blockers, .. }) => {
            assert!(blockers.contains(&QuoteBlocker::ClientNameRequired));
        }
        other => panic!("se esperaba un bloqueo: {:?}", other.err()),
    }
}

#[test]
fn a_png_logo_is_accepted_and_reaches_the_document() {
    let png = |width, height| {
        let mut bytes = Vec::new();
        image::RgbaImage::from_pixel(width, height, image::Rgba([20, 90, 160, 255]))
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    };
    let mut store = Store::demo();
    // Un logo con un lado menor de 200 px se acepta, pero con aviso (misma regla que la app).
    assert_eq!(store.logo_set(png(300, 120)).unwrap().warnings.len(), 1);
    let result = store.logo_set(png(400, 240)).unwrap();
    assert!(result.warnings.is_empty());
    assert!(result.logo.data_url.starts_with("data:image/png;base64,"));
    assert!(store
        .logo_set(b"no es una imagen".to_vec())
        .is_err());
    assert!(store.profile_get().unwrap().logo.is_some());
}
