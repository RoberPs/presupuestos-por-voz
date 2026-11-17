//! Estado compartido de la aplicación.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::AppError;

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(conn: Connection, data_dir: PathBuf) -> Self {
        Self {
            conn: Mutex::new(conn),
            data_dir,
        }
    }

    /// Bloquea la conexión. Un mutex envenenado se recupera: SQLite garantiza la consistencia.
    pub fn conn(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        Ok(self
            .conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()))
    }
}
