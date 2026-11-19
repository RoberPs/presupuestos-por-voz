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

// --- Migración 2: borrador de presupuesto por voz (feature 002) ---

const DRAFT_TABLES: [&str; 6] = [
    "dictations",
    "visit_text_draft",
    "quote_draft",
    "draft_lines",
    "draft_line_suggestions",
    "draft_unparsed_fragments",
];

fn insert_draft(conn: &Connection) {
    conn.execute(
        "INSERT INTO quote_draft (id, status, source_text, text_origin, vat_rate_bp)
         VALUES (1, 'draft', 'texto', 'typed', 2100)",
        [],
    )
    .unwrap();
}

struct LineRow<'a> {
    position: i64,
    unit_label: Option<&'a str>,
    price_cents: Option<i64>,
    price_source: &'a str,
    rate_id: Option<i64>,
    rate_concept: Option<&'a str>,
    pending_reason: Option<&'a str>,
}

impl Default for LineRow<'_> {
    /// Línea válida con precio de entrada manual.
    fn default() -> Self {
        Self {
            position: 0,
            unit_label: Some("metro"),
            price_cents: Some(2500),
            price_source: "manual",
            rate_id: None,
            rate_concept: None,
            pending_reason: None,
        }
    }
}

fn insert_line(conn: &Connection, line: &LineRow) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO draft_lines (draft_id, position, concept, quantity_centi, unit_label,
                                  price_cents, price_source, rate_id, rate_concept, pending_reason)
         VALUES (1, ?1, 'Bajante', 300, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            line.position,
            line.unit_label,
            line.price_cents,
            line.price_source,
            line.rate_id,
            line.rate_concept,
            line.pending_reason,
        ],
    )
}

#[test]
fn migration_2_creates_the_draft_tables() {
    let conn = db::open_in_memory().unwrap();
    for table in DRAFT_TABLES {
        let found: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(found, 1, "falta la tabla {table}");
    }
}

#[test]
fn only_one_quote_draft_can_exist() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    let second = conn.execute(
        "INSERT INTO quote_draft (id, status, source_text, text_origin, vat_rate_bp)
         VALUES (2, 'draft', 'otro', 'typed', 2100)",
        [],
    );
    assert!(second.is_err());
}

#[test]
fn valid_lines_are_accepted() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    insert_line(&conn, &LineRow::default()).unwrap();
    insert_line(
        &conn,
        &LineRow {
            position: 1,
            unit_label: None,
            price_cents: None,
            price_source: "pending",
            pending_reason: Some("no_rate"),
            ..LineRow::default()
        },
    )
    .unwrap();
}

#[test]
fn rate_line_without_price_is_rejected() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    let result = insert_line(
        &conn,
        &LineRow {
            price_cents: None,
            price_source: "rate",
            rate_concept: Some("Bajante"),
            ..LineRow::default()
        },
    );
    assert!(result.is_err());
}

#[test]
fn pending_line_with_price_is_rejected() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    let result = insert_line(
        &conn,
        &LineRow {
            price_source: "pending",
            pending_reason: Some("no_rate"),
            ..LineRow::default()
        },
    );
    assert!(result.is_err());
}

#[test]
fn line_with_price_and_no_unit_is_rejected() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    let result = insert_line(
        &conn,
        &LineRow {
            unit_label: None,
            ..LineRow::default()
        },
    );
    assert!(result.is_err());
}

#[test]
fn deleting_a_rate_keeps_the_line_price() {
    let conn = db::open_in_memory().unwrap();
    conn.execute(
        "INSERT INTO rates (concept, concept_norm, unit_id, price_cents)
         VALUES ('Bajante', 'bajante', 3, 2500)",
        [],
    )
    .unwrap();
    let rate_id = conn.last_insert_rowid();
    insert_draft(&conn);
    insert_line(
        &conn,
        &LineRow {
            price_source: "rate",
            rate_id: Some(rate_id),
            rate_concept: Some("Bajante"),
            ..LineRow::default()
        },
    )
    .unwrap();

    conn.execute("DELETE FROM rates WHERE id = ?1", [rate_id])
        .unwrap();

    let (kept_rate, price, source, concept): (Option<i64>, i64, String, String) = conn
        .query_row(
            "SELECT rate_id, price_cents, price_source, rate_concept FROM draft_lines",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(kept_rate, None);
    assert_eq!(price, 2500);
    assert_eq!(source, "rate");
    assert_eq!(concept, "Bajante");
}

#[test]
fn draft_tables_store_no_end_customer_data() {
    const FORBIDDEN: [&str; 9] = [
        "client", "customer", "cliente", "tax_id", "nif", "address", "phone", "email", "postal",
    ];
    let conn = db::open_in_memory().unwrap();
    for table in DRAFT_TABLES {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |r| r.get(1))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(!columns.is_empty(), "la tabla {table} no tiene columnas");
        for column in columns {
            let name = column.to_lowercase();
            assert!(
                !FORBIDDEN.iter().any(|f| name.contains(f)),
                "{table}.{column} parece un dato del cliente final"
            );
        }
    }
}

// --- Migración 3: datos del presupuesto y numeración (feature 003) ---

fn insert_details(conn: &Connection, draft_id: i64, validity_days: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO quote_details (draft_id, client_name, validity_days) VALUES (?1, 'Ana', ?2)",
        params![draft_id, validity_days],
    )
}

#[test]
fn migration_3_is_the_latest_and_creates_its_tables() {
    let conn = db::open_in_memory().unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 3);
    for table in ["quote_details", "quote_number_sequences"] {
        let found: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(found, 1, "falta la tabla {table}");
    }
}

#[test]
fn only_one_quote_details_row_can_exist() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    insert_details(&conn, 1, 30).unwrap();
    assert!(insert_details(&conn, 1, 30).is_err());
    assert!(insert_details(&conn, 2, 30).is_err());
}

#[test]
fn validity_days_must_be_between_1_and_365() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    for invalid in [0, 366, -1] {
        assert!(
            insert_details(&conn, 1, invalid).is_err(),
            "{invalid} días no debería admitirse"
        );
    }
    insert_details(&conn, 1, 1).unwrap();
    conn.execute("UPDATE quote_details SET validity_days = 365", [])
        .unwrap();
}

#[test]
fn validity_days_defaults_to_30() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    conn.execute(
        "INSERT INTO quote_details (draft_id, client_name) VALUES (1, '')",
        [],
    )
    .unwrap();
    let days: i64 = conn
        .query_row("SELECT validity_days FROM quote_details", [], |r| r.get(0))
        .unwrap();
    assert_eq!(days, 30);
}

#[test]
fn deleting_the_draft_deletes_its_details_but_not_the_numbering() {
    let conn = db::open_in_memory().unwrap();
    insert_draft(&conn);
    insert_details(&conn, 1, 30).unwrap();
    conn.execute(
        "INSERT INTO quote_number_sequences (year, last_number) VALUES (2026, 7)",
        [],
    )
    .unwrap();

    conn.execute("DELETE FROM quote_draft WHERE id = 1", [])
        .unwrap();

    let details: i64 = conn
        .query_row("SELECT COUNT(*) FROM quote_details", [], |r| r.get(0))
        .unwrap();
    assert_eq!(details, 0);
    let last: i64 = conn
        .query_row(
            "SELECT last_number FROM quote_number_sequences WHERE year = 2026",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(last, 7);
}

#[test]
fn number_sequences_reject_values_out_of_range() {
    let conn = db::open_in_memory().unwrap();
    for (year, last) in [(0, 1), (10_000, 1), (2026, -1), (2026, 1_000_000_000)] {
        let result = conn.execute(
            "INSERT INTO quote_number_sequences (year, last_number) VALUES (?1, ?2)",
            params![year, last],
        );
        assert!(result.is_err(), "({year}, {last}) no debería admitirse");
    }
}
