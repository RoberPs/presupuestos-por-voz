use app_lib::storage::db;
use rusqlite::{params, Connection};

fn assert_schema(conn: &Connection) {
    let predefined: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM units WHERE is_predefined = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(predefined, 5);

    // price_cents = 0 viola el CHECK
    let zero = conn.execute(
        "INSERT INTO rates (concept, concept_norm, unit_id, price_cents) VALUES ('a', 'a', 1, 0)",
        [],
    );
    assert!(zero.is_err());

    // duplicado (concept_norm, unit_id)
    conn.execute(
        "INSERT INTO rates (concept, concept_norm, unit_id, price_cents) VALUES (?1, ?2, 1, 100)",
        params!["Grifo", "grifo"],
    )
    .unwrap();
    let dup = conn.execute(
        "INSERT INTO rates (concept, concept_norm, unit_id, price_cents) VALUES (?1, ?2, 1, 200)",
        params!["GRIFO", "grifo"],
    );
    assert!(dup.is_err());

    // FK a una unidad inexistente
    let fk = conn.execute(
        "INSERT INTO rates (concept, concept_norm, unit_id, price_cents) VALUES ('b', 'b', 999, 100)",
        [],
    );
    assert!(fk.is_err());

    // una sola fila de perfil
    conn.execute("INSERT INTO business_profile (id) VALUES (1)", [])
        .unwrap();
    assert!(conn
        .execute("INSERT INTO business_profile (id) VALUES (2)", [])
        .is_err());

    let vat: i64 = conn
        .query_row("SELECT vat_rate_bp FROM business_profile", [], |r| r.get(0))
        .unwrap();
    assert_eq!(vat, 2100);
}

#[test]
fn migrations_apply_in_memory() {
    let conn = db::open_in_memory().unwrap();
    assert_schema(&conn);
}

#[test]
fn migrations_apply_on_file_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let conn = db::open(dir.path()).unwrap();
        assert_schema(&conn);
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }
    // Reabrir no vuelve a aplicar migraciones ni pierde datos
    let conn = db::open(dir.path()).unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM rates", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
}
