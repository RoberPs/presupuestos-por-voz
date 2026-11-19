# Data Model: Perfil del negocio y tarifas

**Feature**: `001-business-profile-rates` | **Fecha**: 2026-11-17

Base de datos SQLite local (ver [research.md](research.md), R2). Importes en céntimos de euro e IVA en puntos básicos (R4). Todas las tablas incluyen `created_at` y `updated_at` (texto ISO-8601 UTC).

## Diagrama

```text
business_profile (1 fila, id = 1)
    └── logo (0..1, columnas embebidas + archivo en app_data_dir/branding/)

units (predefinidas + personalizadas)
    └──< rates (N tarifas por unidad)
```

---

## BusinessProfile — tabla `business_profile`

Fila única (`CHECK (id = 1)`). Si no existe la fila, es el primer arranque y se activa el modo guiado (FR-001c).

| Campo | Tipo | Obligatorio para "completo" | Validación (FR-001, FR-001b) |
|---|---|---|---|
| `id` | INTEGER PK | — | Siempre 1 |
| `trade_name` | TEXT | Sí | 1–120 caracteres tras `trim` |
| `legal_name` | TEXT NULL | No | ≤ 160 caracteres |
| `tax_id` | TEXT | Sí | DNI, NIE o CIF válido con carácter de control; se guarda normalizado (mayúsculas, sin espacios ni guiones) |
| `phone` | TEXT | Sí | 9 dígitos españoles (6/7/8/9) con `+34` opcional, o internacional de 6 a 15 dígitos |
| `email` | TEXT NULL | No | Sintaxis válida si se rellena |
| `website` | TEXT NULL | No | ≤ 200 caracteres; sin validar que exista |
| `address_street` | TEXT | Sí | 1–160 caracteres (calle y número) |
| `address_postal_code` | TEXT | Sí | 5 dígitos, prefijo 01–52 |
| `address_city` | TEXT | Sí | 1–80 caracteres |
| `address_province` | TEXT | Sí | 1–80 caracteres |
| `vat_rate_bp` | INTEGER | Sí (por defecto 2100) | 0–10000 (0,00 %–100,00 %) (FR-019b) |
| `currency` | TEXT | — | Fijo `'EUR'` (`CHECK`) |
| `logo_file` | TEXT NULL | No | Ruta relativa a `app_data_dir`, p. ej. `branding/logo-<hash8>.png` |
| `logo_mime` | TEXT NULL | — | `image/png` o `image/jpeg` |
| `logo_width_px` / `logo_height_px` | INTEGER NULL | — | > 0 |
| `logo_bytes` | INTEGER NULL | — | ≤ 5 242 880 |
| `logo_sha256` | TEXT NULL | — | Hash del archivo guardado |

En la base de datos los campos de texto obligatorios admiten cadena vacía, porque los borradores incompletos se pueden guardar (FR-008).

### Estado derivado: `missing_fields`

No se guarda (R7). Se calcula en cada lectura y escritura:

```text
missing_fields = [campo obligatorio vacío o con formato inválido]
status = "complete" si missing_fields está vacío, si no "incomplete"
```

Transiciones:

```text
(sin fila) ──guardar borrador──▶ incomplete ──completar y corregir──▶ complete
                                     ▲                                  │
                                     └──────── vaciar o invalidar ──────┘
```

El logo no influye en el estado: es opcional.

---

## Unit — tabla `units`

| Campo | Tipo | Reglas |
|---|---|---|
| `id` | INTEGER PK | |
| `label` | TEXT | 1–30 caracteres tras `trim`, p. ej. "hora", "m²" |
| `label_norm` | TEXT UNIQUE | Normalización R5 de `label` |
| `is_predefined` | INTEGER (0/1) | 1 para las semillas |

**Semillas** (migración inicial, FR-011): `unidad`, `hora`, `metro`, `m²`, `metro lineal`.

- Una unidad personalizada se crea al guardar una tarifa con un texto de unidad que no existe (comparando por `label_norm`). Desde entonces aparece en la lista (FR-011).
- No se borran unidades en esta feature. Una unidad personalizada que se queda sin tarifas sigue en la lista, lo que es aceptable a esta escala.

---

## Rate — tabla `rates`

| Campo | Tipo | Reglas |
|---|---|---|
| `id` | INTEGER PK | |
| `concept` | TEXT NOT NULL | 1–160 caracteres tras `trim`; se guarda tal como lo escribe el usuario (sin espacios sobrantes) |
| `concept_norm` | TEXT NOT NULL | Normalización R5 de `concept` |
| `unit_id` | INTEGER NOT NULL FK → `units.id` | Obligatoria |
| `price_cents` | INTEGER NOT NULL | `CHECK (price_cents > 0)`; precio sin IVA en céntimos (FR-012, FR-019) |

- **Unicidad (FR-013)**: `UNIQUE (concept_norm, unit_id)`. Si se intenta un duplicado se devuelve el error `DUPLICATE_RATE` con el `id` de la tarifa existente.
- **Índice**: el propio `UNIQUE` sirve para ordenar por `concept_norm` y para el `LIKE` por subcadena.
- **Borrado (FR-015)**: borrado definitivo (`DELETE`) tras confirmar en la interfaz. El efecto sobre presupuestos es asunto de la feature de presupuestos (spec, Assumptions).

### Reglas de entrada del precio (interfaz y Rust)

| Entrada | Resultado |
|---|---|
| vacío | `PRICE_REQUIRED` |
| `abc`, `12,3,4` | `PRICE_INVALID_FORMAT` |
| `12,345` | `PRICE_TOO_MANY_DECIMALS` |
| `0`, `0,00`, `-5` | `PRICE_NOT_POSITIVE` |
| `12`, `12,5`, `12.50`, `1.234,56` | válido → 1200, 1250, 1250, 123456 céntimos |

Reglas de interpretación: la coma es el separador decimal. El punto también lo es si aparece una sola vez y le siguen 1 o 2 dígitos al final. En cualquier otra posición se trata como separador de miles.

---

## Migraciones

| Versión | Contenido |
|---|---|
| 1 | Creación de `business_profile`, `units` (con semillas) y `rates`; `PRAGMA journal_mode=WAL`, `foreign_keys=ON` |

## Ficheros en disco

```text
<app_data_dir>/
├── presupuestos.db          # SQLite (+ -wal, -shm)
└── branding/
    └── logo-<hash8>.<png|jpg>  # solo el logo vigente
```
