//! Núcleo de la demo web de Presupuestos por Voz.
//!
//! Compila a WebAssembly el **mismo código** que la app de escritorio para el dominio (interpretar
//! el texto, calcular importes, numerar, bloquear y maquetar) y para el PDF. Solo cambia el
//! almacenamiento: en lugar de SQLite, los datos viven en memoria mientras la pestaña está abierta.
//!
//! La interfaz React llama a [`invoke`] con el mismo nombre de comando y los mismos argumentos que
//! usaría con Tauri (contrato IPC de `src/shared/ipc/types.ts`).

#[path = "../../../src-tauri/src/domain/mod.rs"]
pub mod domain;
#[path = "../../../src-tauri/src/error.rs"]
pub mod error;
#[path = "../../../src-tauri/src/pdf/mod.rs"]
pub mod pdf;

pub mod store;

use std::cell::RefCell;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use time::{Date, Month};
use wasm_bindgen::prelude::*;

use crate::domain::profile::BusinessProfileInput;
use crate::domain::quote_doc::details::QuoteDetailsInput;
use crate::domain::rate::RateInput;
use crate::error::AppError;
use crate::store::{DraftLineInput, DraftLinePatch, GenerateMode, Store};

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::demo());
}

const BAD_REQUEST: &str = "La demo ha recibido una petición que no entiende.";

fn bad_request() -> AppError {
    AppError::invalid_state(BAD_REQUEST)
}

fn arg<T: DeserializeOwned>(args: &Value, name: &str) -> Result<T, AppError> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null))
        .map_err(|_| bad_request())
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

fn error_json(error: &AppError) -> String {
    to_json(error)
}

/// Fecha local del navegador en formato `aaaa-mm-dd`.
pub fn parse_today(today: &str) -> Result<Date, AppError> {
    let mut parts = today.split('-').map(str::parse::<i64>);
    let (Some(Ok(year)), Some(Ok(month)), Some(Ok(day))) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(bad_request());
    };
    let month = Month::try_from(month as u8).map_err(|_| bad_request())?;
    Date::from_calendar_date(year as i32, month, day as u8).map_err(|_| bad_request())
}

/// Ejecuta un comando del contrato IPC sobre los datos en memoria.
pub fn dispatch(store: &mut Store, command: &str, args: &Value, today: Date) -> Result<String, AppError> {
    let allow_logo = || args.get("allowMissingLogo").and_then(Value::as_bool).unwrap_or(false);
    Ok(match command {
        "profile_get" => to_json(&store.profile_get()),
        "profile_save" => to_json(&store.profile_save(arg::<BusinessProfileInput>(args, "input")?)?),
        "profile_validate" => to_json(&Store::profile_validate(&arg(args, "input")?)),
        "logo_remove" => to_json(&store.logo_remove()),

        "units_list" => to_json(&store.units_list()),
        "rates_list" => to_json(&store.rates_list(arg(args, "query")?)),
        "rates_create" => to_json(&store.rates_create(arg::<RateInput>(args, "input")?)?),
        "rates_update" => to_json(&store.rates_update(arg(args, "id")?, arg(args, "input")?)?),
        "rates_delete" => {
            store.rates_delete(arg(args, "id")?)?;
            "null".into()
        }

        "session_get" => to_json(&store.session()?),
        "draft_generate_from_text" => to_json(&store.generate_from_text(
            &arg::<String>(args, "text")?,
            arg::<GenerateMode>(args, "mode")?,
            arg(args, "confirmed")?,
        )?),
        "draft_text_save" => {
            store.save_text(&arg::<String>(args, "text")?)?;
            "null".into()
        }
        "draft_line_add" => to_json(&store.line_add(arg::<DraftLineInput>(args, "input")?)?),
        "draft_line_update" => to_json(
            &store.line_update(arg(args, "lineId")?, arg::<DraftLinePatch>(args, "patch")?)?,
        ),
        "draft_line_assign_rate" => {
            to_json(&store.line_assign_rate(arg(args, "lineId")?, arg(args, "rateId")?)?)
        }
        "draft_line_delete" => to_json(&store.line_delete(arg(args, "lineId")?)?),
        "draft_apply_current_vat" => to_json(&store.apply_current_vat()?),
        "draft_mark_reviewed" => to_json(&store.mark_reviewed()?),
        "draft_discard" => to_json(&store.discard()?),

        "quote_prepare" => to_json(&store.quote_prepare(today, allow_logo())?),
        "quote_details_save" => to_json(&store.quote_details_save(
            arg::<QuoteDetailsInput>(args, "details")?,
            today,
            allow_logo(),
        )?),
        _ => return Err(bad_request()),
    })
}

/// Punto de entrada de la interfaz: devuelve el resultado en JSON o un `AppError` en JSON.
#[wasm_bindgen]
pub fn invoke(command: &str, args: &str, today: &str) -> Result<String, String> {
    let run = || -> Result<String, AppError> {
        let args: Value = serde_json::from_str(args).map_err(|_| bad_request())?;
        let today = parse_today(today)?;
        STORE.with(|store| dispatch(&mut store.borrow_mut(), command, &args, today))
    };
    run().map_err(|e| error_json(&e))
}

/// Sustituye el logo con los bytes del archivo elegido en el navegador.
#[wasm_bindgen]
pub fn logo_set(bytes: Vec<u8>) -> Result<String, String> {
    STORE
        .with(|store| store.borrow_mut().logo_set(bytes))
        .map(|result| to_json(&result))
        .map_err(|e| error_json(&e))
}

/// PDF generado, listo para descargar.
#[wasm_bindgen]
pub struct DemoPdf {
    file_name: String,
    page_count: usize,
    bytes: Vec<u8>,
}

#[wasm_bindgen]
impl DemoPdf {
    #[wasm_bindgen(getter, js_name = fileName)]
    pub fn file_name(&self) -> String {
        self.file_name.clone()
    }

    #[wasm_bindgen(getter, js_name = pageCount)]
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

/// Genera el PDF confirmado. Mismas comprobaciones que `quote_generate` en la app.
#[wasm_bindgen]
pub fn quote_generate(
    confirmation_token: &str,
    allow_missing_logo: bool,
    today: &str,
) -> Result<DemoPdf, String> {
    let run = || -> Result<DemoPdf, AppError> {
        let today = parse_today(today)?;
        let generated = STORE.with(|store| {
            store
                .borrow_mut()
                .quote_generate(confirmation_token, today, allow_missing_logo)
        })?;
        Ok(DemoPdf {
            file_name: generated.file_name,
            page_count: generated.rendered.page_count,
            bytes: generated.rendered.bytes,
        })
    };
    run().map_err(|e| error_json(&e))
}

/// Vuelve a los datos de ejemplo.
#[wasm_bindgen]
pub fn reset() {
    STORE.with(|store| *store.borrow_mut() = Store::demo());
}
