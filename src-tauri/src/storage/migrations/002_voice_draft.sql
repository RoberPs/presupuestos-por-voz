-- Migración 2: dictado y borrador de presupuesto por voz (specs/002-voice-quote-draft/data-model.md).
-- Importes en céntimos, IVA en puntos básicos y cantidades en centésimas enteras.
-- Las implicaciones "A ⇒ B" de data-model.md se escriben como (NOT A OR B).

CREATE TABLE dictations (
    slot               TEXT    PRIMARY KEY CHECK (slot IN ('current', 'incoming')),
    audio_file         TEXT    NOT NULL,
    duration_ms        INTEGER NOT NULL CHECK (duration_ms BETWEEN 1 AND 600000),
    origin             TEXT    NOT NULL CHECK (origin IN ('recorded', 'imported')),
    original_file_name TEXT,
    status             TEXT    NOT NULL
                       CHECK (status IN ('recorded', 'processing', 'transcribed', 'failed', 'no_speech')),
    low_quality        INTEGER NOT NULL DEFAULT 0 CHECK (low_quality IN (0, 1)),
    error_code         TEXT    CHECK (error_code IS NULL
                                      OR error_code IN ('MODEL_UNAVAILABLE', 'TRANSCRIPTION_FAILED')),
    created_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE visit_text_draft (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    text       TEXT    NOT NULL CHECK (length(text) <= 20000),
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE quote_draft (
    id                 INTEGER PRIMARY KEY CHECK (id = 1),
    status             TEXT    NOT NULL CHECK (status IN ('draft', 'reviewed')),
    source_text        TEXT    NOT NULL CHECK (length(source_text) <= 20000),
    text_origin        TEXT    NOT NULL CHECK (text_origin IN ('transcribed', 'typed')),
    text_corrected     INTEGER NOT NULL DEFAULT 0 CHECK (text_corrected IN (0, 1)),
    vat_rate_bp        INTEGER NOT NULL CHECK (vat_rate_bp BETWEEN 0 AND 10000),
    has_manual_changes INTEGER NOT NULL DEFAULT 0 CHECK (has_manual_changes IN (0, 1)),
    created_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE draft_lines (
    id              INTEGER PRIMARY KEY,
    draft_id        INTEGER NOT NULL REFERENCES quote_draft (id) ON DELETE CASCADE,
    position        INTEGER NOT NULL,
    concept         TEXT    NOT NULL CHECK (length(concept) BETWEEN 1 AND 160),
    quantity_centi  INTEGER CHECK (quantity_centi IS NULL OR quantity_centi BETWEEN 1 AND 99999999),
    quantity_issue  TEXT    CHECK (quantity_issue IS NULL
                                   OR quantity_issue IN ('missing', 'ambiguous', 'rectified')),
    unit_label      TEXT    CHECK (unit_label IS NULL OR length(unit_label) BETWEEN 1 AND 30),
    price_cents     INTEGER CHECK (price_cents IS NULL OR price_cents > 0),
    price_source    TEXT    NOT NULL CHECK (price_source IN ('rate', 'manual', 'pending')),
    rate_id         INTEGER REFERENCES rates (id) ON DELETE SET NULL,
    rate_concept    TEXT,
    pending_reason  TEXT    CHECK (pending_reason IS NULL
                                   OR pending_reason IN ('no_rate', 'ambiguous_rate', 'similar_rate', 'unit_mismatch')),
    source_fragment TEXT,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (draft_id, position),
    CHECK (price_source <> 'rate' OR (price_cents IS NOT NULL AND rate_concept IS NOT NULL)),
    CHECK (price_source <> 'manual' OR (price_cents IS NOT NULL AND rate_id IS NULL)),
    CHECK (price_source <> 'pending'
           OR (price_cents IS NULL AND rate_id IS NULL AND pending_reason IS NOT NULL)),
    CHECK (price_cents IS NULL OR unit_label IS NOT NULL),
    CHECK (quantity_centi IS NULL OR quantity_issue IS NULL)
);

CREATE TABLE draft_line_suggestions (
    line_id    INTEGER NOT NULL REFERENCES draft_lines (id) ON DELETE CASCADE,
    rate_id    INTEGER NOT NULL REFERENCES rates (id) ON DELETE CASCADE,
    rank       INTEGER NOT NULL CHECK (rank BETWEEN 0 AND 4),
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (line_id, rate_id)
);

CREATE TABLE draft_unparsed_fragments (
    id         INTEGER PRIMARY KEY,
    draft_id   INTEGER NOT NULL REFERENCES quote_draft (id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    text       TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_draft_lines_rate ON draft_lines (rate_id);
CREATE INDEX idx_draft_line_suggestions_rate ON draft_line_suggestions (rate_id);
