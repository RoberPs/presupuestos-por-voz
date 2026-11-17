//! Fila única `business_profile` (id = 1).

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::domain::profile::{Address, BusinessProfile, LogoMeta};

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

fn from_row(r: &Row<'_>) -> rusqlite::Result<BusinessProfile> {
    let logo_file: Option<String> = r.get("logo_file")?;
    let logo = match logo_file {
        Some(file) => Some(LogoMeta {
            file,
            mime: r.get("logo_mime")?,
            width_px: r.get("logo_width_px")?,
            height_px: r.get("logo_height_px")?,
            bytes: r.get::<_, i64>("logo_bytes")? as u64,
            sha256: r.get("logo_sha256")?,
        }),
        None => None,
    };
    Ok(BusinessProfile {
        trade_name: r.get("trade_name")?,
        legal_name: r.get("legal_name")?,
        tax_id: r.get("tax_id")?,
        phone: r.get("phone")?,
        email: r.get("email")?,
        website: r.get("website")?,
        address: Address {
            street: r.get("address_street")?,
            postal_code: r.get("address_postal_code")?,
            city: r.get("address_city")?,
            province: r.get("address_province")?,
        },
        vat_rate_bp: r.get("vat_rate_bp")?,
        logo,
    })
}

pub fn get(conn: &Connection) -> rusqlite::Result<Option<BusinessProfile>> {
    conn.query_row("SELECT * FROM business_profile WHERE id = 1", [], from_row).optional()
}

/// Crea la fila con valores vacíos si aún no existe (borrador).
pub fn ensure_row(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute("INSERT OR IGNORE INTO business_profile (id) VALUES (1)", [])?;
    Ok(())
}

/// Crea o actualiza los datos del perfil. No toca las columnas del logo.
pub fn upsert(tx: &Transaction<'_>, p: &BusinessProfile) -> rusqlite::Result<()> {
    let sql = format!(
        "INSERT INTO business_profile (
            id, trade_name, legal_name, tax_id, phone, email, website,
            address_street, address_postal_code, address_city, address_province, vat_rate_bp
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
            trade_name = excluded.trade_name,
            legal_name = excluded.legal_name,
            tax_id = excluded.tax_id,
            phone = excluded.phone,
            email = excluded.email,
            website = excluded.website,
            address_street = excluded.address_street,
            address_postal_code = excluded.address_postal_code,
            address_city = excluded.address_city,
            address_province = excluded.address_province,
            vat_rate_bp = excluded.vat_rate_bp,
            updated_at = {NOW}"
    );
    tx.execute(
        &sql,
        params![
            p.trade_name,
            p.legal_name,
            p.tax_id,
            p.phone,
            p.email,
            p.website,
            p.address.street,
            p.address.postal_code,
            p.address.city,
            p.address.province,
            p.vat_rate_bp,
        ],
    )?;
    Ok(())
}

/// Asocia o quita (`None`) el logo. La fila debe existir (ver `ensure_row`).
pub fn set_logo(tx: &Transaction<'_>, logo: Option<&LogoMeta>) -> rusqlite::Result<()> {
    let sql = format!(
        "UPDATE business_profile SET
            logo_file = ?1, logo_mime = ?2, logo_width_px = ?3, logo_height_px = ?4,
            logo_bytes = ?5, logo_sha256 = ?6, updated_at = {NOW}
         WHERE id = 1"
    );
    tx.execute(
        &sql,
        params![
            logo.map(|l| &l.file),
            logo.map(|l| &l.mime),
            logo.map(|l| l.width_px),
            logo.map(|l| l.height_px),
            logo.map(|l| l.bytes as i64),
            logo.map(|l| &l.sha256),
        ],
    )?;
    Ok(())
}
