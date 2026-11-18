//! Apertura de la base de datos SQLite local y migraciones (R2).

use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::error::AppError;

pub const DB_FILE_NAME: &str = "presupuestos.db";

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("migrations/001_init.sql")),
        M::up(include_str!("migrations/002_voice_draft.sql")),
    ])
}

fn configure(conn: &mut Connection) -> Result<(), AppError> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations().to_latest(conn).map_err(|e| {
        log::error!("Error al aplicar las migraciones: {e}");
        AppError::storage()
    })?;
    Ok(())
}

/// Abre (o crea) `presupuestos.db` en `data_dir`, en modo WAL y con las migraciones aplicadas.
pub fn open(data_dir: &Path) -> Result<Connection, AppError> {
    std::fs::create_dir_all(data_dir)?;
    let mut conn = Connection::open(data_dir.join(DB_FILE_NAME))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    configure(&mut conn)?;
    Ok(conn)
}

/// Base de datos en memoria con el mismo esquema, para pruebas.
pub fn open_in_memory() -> Result<Connection, AppError> {
    let mut conn = Connection::open_in_memory()?;
    configure(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert!(migrations().validate().is_ok());
    }
}
