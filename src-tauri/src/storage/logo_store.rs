//! Archivo del logo en `app_data_dir/branding/`, con escritura atómica.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;

pub const BRANDING_DIR: &str = "branding";

fn temp_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!(".tmp-{}-{nanos}", std::process::id())
}

/// Escribe `bytes` en un temporal, hace `fsync` y lo renombra a `branding/logo-<sha8>.<ext>`.
/// Devuelve la ruta relativa a `data_dir` (con `/`).
pub fn write_atomic(
    data_dir: &Path,
    bytes: &[u8],
    sha256_hex: &str,
    ext: &str,
) -> io::Result<String> {
    let dir = data_dir.join(BRANDING_DIR);
    fs::create_dir_all(&dir)?;
    let file_name = format!("logo-{}.{ext}", &sha256_hex[..8]);
    let tmp = dir.join(temp_name());
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, dir.join(&file_name))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result?;
    Ok(format!("{BRANDING_DIR}/{file_name}"))
}

/// Borra un archivo del logo. Que no exista no es un error.
pub fn delete(data_dir: &Path, rel_path: &str) -> io::Result<()> {
    match fs::remove_file(data_dir.join(rel_path)) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub fn read_data_url(data_dir: &Path, rel_path: &str, mime: &str) -> io::Result<String> {
    let bytes = fs::read(data_dir.join(rel_path))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}
