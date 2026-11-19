#![allow(dead_code)]

use std::path::{Path, PathBuf};

use app_lib::domain::draft::TextOrigin;
use app_lib::domain::profile::{Address, BusinessProfileInput};
use app_lib::domain::rate::{RateInput, RateView};
use app_lib::services::{draft_service, rate_service};
use app_lib::state::AppState;
use app_lib::storage::{db, draft_repo};
use image::{ImageBuffer, ImageFormat, Rgb, Rgba};

pub const EXAMPLE_TEXT: &str = "cambiar bajante, 3 metros, mano de obra 2 horas";

pub fn create_rate(state: &AppState, concept: &str, unit: &str, price: &str) -> RateView {
    rate_service::create(
        state,
        RateInput {
            concept: concept.into(),
            unit_label: unit.into(),
            price: price.into(),
        },
    )
    .unwrap()
}

/// Tarifas de los ejemplos de la spec: Bajante (25 €/metro) y Mano de obra (30 €/hora).
pub fn example_rates(state: &AppState) -> (RateView, RateView) {
    (
        create_rate(state, "Bajante", "metro", "25"),
        create_rate(state, "Mano de obra", "hora", "30"),
    )
}

/// Guarda un borrador generado a partir de `text`.
pub fn seed_draft(state: &AppState, text: &str) {
    let mut conn = state.conn().unwrap();
    let tx = conn.transaction().unwrap();
    let built = draft_service::build_from_text(&tx, text).unwrap();
    draft_repo::replace(&tx, &built, text, TextOrigin::Typed).unwrap();
    tx.commit().unwrap();
}

/// Vista del borrador en curso.
pub fn draft_view(state: &AppState) -> draft_service::DraftView {
    let conn = state.conn().unwrap();
    draft_service::load_view(&conn)
        .unwrap()
        .expect("debe haber borrador")
}

/// Estado con base de datos en un archivo dentro de `dir`.
pub fn state_in(dir: &Path) -> AppState {
    AppState::new(db::open(dir).unwrap(), dir.to_path_buf())
}

/// Simula cerrar y reabrir la app.
pub fn reopen(state: AppState) -> AppState {
    let dir = state.data_dir.clone();
    drop(state);
    state_in(&dir)
}

pub fn complete_input() -> BusinessProfileInput {
    BusinessProfileInput {
        trade_name: "Fontanería López".into(),
        legal_name: Some("José López García".into()),
        tax_id: "12345678Z".into(),
        phone: "600 123 456".into(),
        email: Some("jose@fontanerialopez.es".into()),
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

pub fn empty_input() -> BusinessProfileInput {
    BusinessProfileInput {
        trade_name: String::new(),
        legal_name: None,
        tax_id: String::new(),
        phone: String::new(),
        email: None,
        website: None,
        address: Address {
            street: String::new(),
            postal_code: String::new(),
            city: String::new(),
            province: String::new(),
        },
        vat_rate_percent: "21".into(),
    }
}

pub fn write_png_rgba(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(w, h, |x, _| {
        Rgba([200, 30, 30, if x % 2 == 0 { 0 } else { 255 }])
    });
    let path = dir.join(name);
    img.save_with_format(&path, ImageFormat::Png).unwrap();
    path
}

pub fn write_jpg(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(w, h, |x, y| Rgb([(x % 255) as u8, (y % 255) as u8, 120]));
    let path = dir.join(name);
    img.save_with_format(&path, ImageFormat::Jpeg).unwrap();
    path
}

pub fn branding_files(data_dir: &Path) -> Vec<String> {
    let dir = data_dir.join("branding");
    let mut names: Vec<String> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into())
            .collect(),
        Err(_) => vec![],
    };
    names.sort();
    names
}
