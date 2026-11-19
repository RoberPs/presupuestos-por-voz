-- Migración 1: perfil del negocio, unidades y tarifas (data-model.md).
-- Importes en céntimos e IVA en puntos básicos (R4). Fechas en ISO-8601 UTC.

CREATE TABLE business_profile (
    id                  INTEGER PRIMARY KEY CHECK (id = 1),
    trade_name          TEXT    NOT NULL DEFAULT '',
    legal_name          TEXT,
    tax_id              TEXT    NOT NULL DEFAULT '',
    phone               TEXT    NOT NULL DEFAULT '',
    email               TEXT,
    website             TEXT,
    address_street      TEXT    NOT NULL DEFAULT '',
    address_postal_code TEXT    NOT NULL DEFAULT '',
    address_city        TEXT    NOT NULL DEFAULT '',
    address_province    TEXT    NOT NULL DEFAULT '',
    vat_rate_bp         INTEGER NOT NULL DEFAULT 2100 CHECK (vat_rate_bp BETWEEN 0 AND 10000),
    currency            TEXT    NOT NULL DEFAULT 'EUR' CHECK (currency = 'EUR'),
    logo_file           TEXT,
    logo_mime           TEXT    CHECK (logo_mime IS NULL OR logo_mime IN ('image/png', 'image/jpeg')),
    logo_width_px       INTEGER CHECK (logo_width_px IS NULL OR logo_width_px > 0),
    logo_height_px      INTEGER CHECK (logo_height_px IS NULL OR logo_height_px > 0),
    logo_bytes          INTEGER CHECK (logo_bytes IS NULL OR (logo_bytes > 0 AND logo_bytes <= 5242880)),
    logo_sha256         TEXT,
    created_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE units (
    id            INTEGER PRIMARY KEY,
    label         TEXT    NOT NULL CHECK (length(label) BETWEEN 1 AND 30),
    label_norm    TEXT    NOT NULL UNIQUE,
    is_predefined INTEGER NOT NULL DEFAULT 0 CHECK (is_predefined IN (0, 1)),
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO units (id, label, label_norm, is_predefined) VALUES
    (1, 'unidad',       'unidad',       1),
    (2, 'hora',         'hora',         1),
    (3, 'metro',        'metro',        1),
    (4, 'm²',           'm²',           1),
    (5, 'metro lineal', 'metro lineal', 1);

CREATE TABLE rates (
    id           INTEGER PRIMARY KEY,
    concept      TEXT    NOT NULL CHECK (length(concept) BETWEEN 1 AND 160),
    concept_norm TEXT    NOT NULL,
    unit_id      INTEGER NOT NULL REFERENCES units (id),
    price_cents  INTEGER NOT NULL CHECK (price_cents > 0),
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (concept_norm, unit_id)
);
