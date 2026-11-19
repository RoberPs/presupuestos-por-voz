-- Migración 3: datos del presupuesto y numeración (specs/003-quote-pdf-export/data-model.md).

CREATE TABLE quote_details (
    draft_id            INTEGER PRIMARY KEY CHECK (draft_id = 1)
                        REFERENCES quote_draft (id) ON DELETE CASCADE,
    client_name         TEXT    NOT NULL CHECK (length(client_name) <= 120),
    client_address      TEXT    CHECK (client_address IS NULL OR length(client_address) <= 200),
    client_phone        TEXT    CHECK (client_phone IS NULL OR length(client_phone) <= 30),
    client_email        TEXT    CHECK (client_email IS NULL OR length(client_email) <= 254),
    client_tax_id       TEXT    CHECK (client_tax_id IS NULL OR length(client_tax_id) <= 20),
    quote_number        TEXT    CHECK (quote_number IS NULL OR length(quote_number) <= 30),
    issued_number       TEXT    CHECK (issued_number IS NULL OR length(issued_number) <= 30),
    issued_content_hash TEXT,
    validity_days       INTEGER NOT NULL DEFAULT 30 CHECK (validity_days BETWEEN 1 AND 365),
    created_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK ((issued_number IS NULL) = (issued_content_hash IS NULL))
);

CREATE TABLE quote_number_sequences (
    year        INTEGER PRIMARY KEY CHECK (year BETWEEN 1 AND 9999),
    last_number INTEGER NOT NULL CHECK (last_number BETWEEN 0 AND 999999999),
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
