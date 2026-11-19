//! Semilla de desarrollo: inserta N tarifas sintéticas en la base de datos real de la app.
//!
//! Uso: `npm run seed:rates -- 500` (por defecto 500). Respeta `PRESUPUESTOS_DATA_DIR`.
//! Debe ejecutarse con la app cerrada.

use std::path::PathBuf;

use app_lib::domain::rate::RateInput;
use app_lib::error::AppError;
use app_lib::services::rate_service;
use app_lib::state::AppState;
use app_lib::storage::db;

const IDENTIFIER: &str = "es.presupuestosvoz.app";

/// Misma carpeta que `app_data_dir` de Tauri para el identificador de la app.
fn app_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var(app_lib::DATA_DIR_ENV) {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    base.expect("no se encuentra la carpeta de datos del usuario")
        .join(IDENTIFIER)
}

const MATERIALS: &[&str] = &[
    "Tubo cobre",
    "Tubo multicapa",
    "Codo PVC",
    "Cable eléctrico",
    "Pintura plástica",
    "Azulejo",
    "Grifo monomando",
    "Llave de paso",
    "Interruptor",
    "Enchufe",
];
const UNITS: &[&str] = &["unidad", "hora", "metro", "m²", "metro lineal"];

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(500);
    let dir = app_data_dir();
    println!("Base de datos: {}", dir.join(db::DB_FILE_NAME).display());
    let state = AppState::new(
        db::open(&dir).expect("no se puede abrir la base de datos"),
        dir,
    );

    let (mut created, mut skipped) = (0, 0);
    for i in 1..=n {
        let material = MATERIALS[i % MATERIALS.len()];
        let input = RateInput {
            concept: format!("{material} {} mm {i}", 10 + (i % 40)),
            unit_label: UNITS[i % UNITS.len()].to_string(),
            price: format!("{},{:02}", 1 + (i * 7) % 300, (i * 13) % 100),
        };
        match rate_service::create(&state, input) {
            Ok(_) => created += 1,
            Err(AppError::DuplicateRate { .. }) => skipped += 1,
            Err(e) => panic!("error al crear la tarifa {i}: {e:?}"),
        }
    }
    println!("Tarifas creadas: {created}; ya existían: {skipped}");
}
