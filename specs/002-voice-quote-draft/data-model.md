# Data Model: Borrador de presupuesto por voz

**Feature**: `002-voice-quote-draft` | **Fecha**: 2026-11-18

Amplía la base de datos de la feature 001 con la migración 2. Importes en céntimos, IVA en puntos básicos y cantidades en centésimas enteras ([research.md](research.md), R9). Todas las tablas incluyen `created_at` y `updated_at` (texto ISO-8601 UTC).

## Diagrama

```text
dictations (0..2: ranuras current e incoming)

visit_text_draft (0..1, id = 1)

quote_draft (0..1, id = 1)
    ├──< draft_lines
    │       ├──> rates (opcional; SET NULL al borrar la tarifa)
    │       └──< draft_line_suggestions ──> rates (CASCADE)
    └──< draft_unparsed_fragments
```

---

## Dictation — tabla `dictations`

| Campo | Tipo | Reglas |
|---|---|---|
| `slot` | TEXT PK | `current` (audio del borrador vigente) o `incoming` (audio nuevo aún sin borrador) |
| `audio_file` | TEXT NOT NULL | Ruta relativa, p. ej. `recordings/dictation-<ms>.wav` |
| `duration_ms` | INTEGER NOT NULL | 1–600 000 (FR-005, FR-002c); la captura recorta al límite exacto y una grabación sin audio no crea fila |
| `origin` | TEXT NOT NULL | `recorded` o `imported` |
| `original_file_name` | TEXT NULL | Solo si es importado |
| `status` | TEXT NOT NULL | `recorded`, `processing`, `transcribed`, `failed`, `no_speech` |
| `low_quality` | INTEGER (0/1) | Aviso de FR-007 |
| `error_code` | TEXT NULL | `MODEL_UNAVAILABLE` o `TRANSCRIPTION_FAILED` si `failed` |

### Estados

```text
(grabando: solo archivo .part)
        │ parar / límite / recuperar / importar
        ▼
    recorded ──procesar──▶ processing ──con texto──▶ transcribed (pasa a current)
        ▲                      │
        ├────── cancelar ──────┤
        │                      ├── sin voz ──▶ no_speech
        └───── reintentar ── failed ◀── error
```

Al arrancar, una fila en `processing` vuelve a `recorded`, porque el proceso se interrumpió.

---

## VisitTextDraft — tabla `visit_text_draft`

Fila única (`CHECK (id = 1)`) con el texto escrito que aún no tiene borrador (FR-035). Se borra al generar el borrador.

| Campo | Tipo | Reglas |
|---|---|---|
| `id` | INTEGER PK | Siempre 1 |
| `text` | TEXT NOT NULL | Hasta 20 000 caracteres |

---

## QuoteDraft — tabla `quote_draft`

Fila única (`CHECK (id = 1)`), que existe solo si hay borrador (FR-036).

| Campo | Tipo | Reglas |
|---|---|---|
| `id` | INTEGER PK | Siempre 1 |
| `status` | TEXT NOT NULL | `draft` o `reviewed` |
| `source_text` | TEXT NOT NULL | Texto de la visita, hasta 20 000 caracteres |
| `text_origin` | TEXT NOT NULL | `transcribed` o `typed` |
| `text_corrected` | INTEGER (0/1) | El texto se ha cambiado tras generarlo |
| `vat_rate_bp` | INTEGER NOT NULL | 0–10000; copia del IVA del negocio al generar (FR-029) |
| `has_manual_changes` | INTEGER (0/1) | Activa el aviso antes de regenerar (FR-012) |

### Valores derivados (no se guardan)

```text
line_amount   = cantidad × precio, redondeado al céntimo (si la línea está completa)
base          = suma de line_amount
vat           = base × vat_rate_bp / 10000, redondeado al céntimo
total         = base + vat
pending_count = líneas sin precio o sin cantidad
is_provisional = pending_count > 0
vat_changed   = vat_rate_bp ≠ business_profile.vat_rate_bp (FR-029b)
```

### Estados

```text
(sin fila) ──generar──▶ draft ──dar por bueno (≥ 1 línea y 0 pendientes)──▶ reviewed
                          ▲                                                    │
                          └───────── cualquier edición o regeneración ─────────┘
```

---

## DraftLine — tabla `draft_lines`

| Campo | Tipo | Reglas |
|---|---|---|
| `id` | INTEGER PK | |
| `draft_id` | INTEGER NOT NULL FK | `ON DELETE CASCADE` |
| `position` | INTEGER NOT NULL | Orden del texto; `UNIQUE (draft_id, position)` |
| `concept` | TEXT NOT NULL | 1–160 caracteres. El de la tarifa si se aplica; si no, el dictado |
| `quantity_centi` | INTEGER NULL | `> 0` y ≤ 99 999 999; `NULL` = cantidad pendiente |
| `quantity_issue` | TEXT NULL | `missing`, `ambiguous` o `rectified` |
| `unit_label` | TEXT NULL | 1–30 caracteres; copia del texto de la unidad |
| `price_cents` | INTEGER NULL | `> 0`; `NULL` = precio pendiente |
| `price_source` | TEXT NOT NULL | `rate`, `manual` o `pending` (FR-032) |
| `rate_id` | INTEGER NULL FK → `rates.id` | `ON DELETE SET NULL` (FR-033) |
| `rate_concept` | TEXT NULL | Copia del concepto de la tarifa, para conservar la trazabilidad |
| `pending_reason` | TEXT NULL | `no_rate`, `ambiguous_rate`, `similar_rate` o `unit_mismatch` |
| `source_fragment` | TEXT NULL | Fragmento del que procede; `NULL` en líneas añadidas a mano |

### Restricciones (`CHECK`)

```text
price_source = 'rate'    ⇒ price_cents NOT NULL y rate_concept NOT NULL
price_source = 'manual'  ⇒ price_cents NOT NULL y rate_id NULL
price_source = 'pending' ⇒ price_cents NULL y rate_id NULL y pending_reason NOT NULL
price_cents NOT NULL     ⇒ unit_label NOT NULL
quantity_centi NOT NULL  ⇒ quantity_issue NULL
```

### Reglas de edición

| Acción | Efecto |
|---|---|
| Cambiar la cantidad | Recalcula el importe y borra `quantity_issue` |
| Cambiar el concepto | Conserva el origen del precio |
| Cambiar el precio | `price_source = manual`, `rate_id = NULL` (FR-032) |
| Cambiar la unidad de una línea con tarifa | El precio deja de valer: `pending`, `unit_mismatch`, con la tarifa como sugerencia (FR-021) |
| Asignar una tarifa | Copia precio, unidad y concepto; `price_source = rate` |
| Cualquiera de las anteriores | `has_manual_changes = 1`; si estaba `reviewed`, vuelve a `draft` (FR-034) |

### Validación de la cantidad

| Entrada | Resultado |
|---|---|
| vacío | `QUANTITY_REQUIRED` |
| `abc` | `QUANTITY_INVALID_FORMAT` |
| `1,234` | `QUANTITY_TOO_MANY_DECIMALS` |
| `0`, `-2` | `QUANTITY_NOT_POSITIVE` |
| `3`, `1,5`, `2.25` | válido → 300, 150, 225 |

---

## DraftLineSuggestion — tabla `draft_line_suggestions`

| Campo | Tipo | Reglas |
|---|---|---|
| `line_id` | INTEGER FK | `ON DELETE CASCADE` |
| `rate_id` | INTEGER FK | `ON DELETE CASCADE` |
| `rank` | INTEGER NOT NULL | 0–4; orden estable (R7) |

Clave primaria: `(line_id, rate_id)`. El precio de la sugerencia se lee de la tarifa vigente al mostrarla.

## DraftUnparsedFragment — tabla `draft_unparsed_fragments`

| Campo | Tipo | Reglas |
|---|---|---|
| `id` | INTEGER PK | |
| `draft_id` | INTEGER NOT NULL FK | `ON DELETE CASCADE` |
| `position` | INTEGER NOT NULL | Orden en el texto |
| `text` | TEXT NOT NULL | Fragmento sin interpretar (FR-022) |

---

## Migraciones

| Versión | Contenido |
|---|---|
| 2 | Creación de `dictations`, `visit_text_draft`, `quote_draft`, `draft_lines`, `draft_line_suggestions` y `draft_unparsed_fragments` |

## Ficheros en disco

```text
<app_data_dir>/
└── recordings/
    ├── dictation-<ms>.wav   # audio de las ranuras current e incoming (16 kHz, mono, 16 bits)
    └── incoming.wav.part    # solo mientras se graba o tras un cierre inesperado
```

Al arrancar se borran los `.wav` que ninguna fila de `dictations` referencia.
