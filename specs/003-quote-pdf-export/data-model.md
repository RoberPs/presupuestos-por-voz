# Data Model: PDF del presupuesto

**Feature**: `003-quote-pdf-export` | **Fecha**: 2026-11-18

Amplía la base de datos de la feature 002 con la migración 3. El PDF generado no se registra en la base de datos, porque el historial está fuera de alcance.

## Diagrama

```text
quote_draft (0..1, feature 002)
    └──< quote_details (0..1; CASCADE al borrar el borrador)

quote_number_sequences (una fila por año; independiente del borrador)
```

---

## QuoteDetails — tabla `quote_details`

Datos del presupuesto en curso (FR-012b). No cambian el estado de revisión del borrador.

| Campo | Tipo | Reglas |
|---|---|---|
| `draft_id` | INTEGER PK FK | Siempre 1; `ON DELETE CASCADE` hacia `quote_draft` |
| `client_name` | TEXT NOT NULL | 0–120 caracteres; obligatorio para generar |
| `client_address` | TEXT NULL | Hasta 200 caracteres |
| `client_phone` | TEXT NULL | Hasta 30 caracteres |
| `client_email` | TEXT NULL | Hasta 254 caracteres |
| `client_tax_id` | TEXT NULL | Hasta 20 caracteres |
| `quote_number` | TEXT NULL | Hasta 30 caracteres. Número escrito a mano. `NULL` = usar el número propuesto por la app; vacío = el profesional lo ha borrado |
| `issued_number` | TEXT NULL | Número del último PDF guardado de este borrador |
| `issued_content_hash` | TEXT NULL | Huella de contenido de ese PDF. Se rellena junto con `issued_number` |
| `validity_days` | INTEGER NOT NULL | 1–365; 30 por defecto (FR-014b) |

Los textos se guardan sin espacios sobrantes y sin comprobar su formato. Un dato opcional vacío se guarda como `NULL`.

### Reglas

| Situación | Efecto |
|---|---|
| Se escribe el número que propone la app | `quote_number = NULL`: sigue siendo automático |
| Se escribe otro número | Se guarda tal cual |
| El PDF se guarda correctamente | `issued_number` e `issued_content_hash` guardan lo emitido y `quote_number` vuelve a `NULL` (FR-013b) |
| Cambian las líneas o el IVA tras emitir | La huella deja de coincidir y se propone el siguiente número |
| Cambian solo el cliente o la validez | La huella coincide y se propone el mismo número |
| Se regenera el mismo borrador | Los datos se conservan |
| Empieza un borrador nuevo o se descarta | La cascada borra la fila |

---

## QuoteNumberSequence — tabla `quote_number_sequences`

| Campo | Tipo | Reglas |
|---|---|---|
| `year` | INTEGER PK | 1–9999 |
| `last_number` | INTEGER NOT NULL | 0–999 999 999 |

```text
propuesto = quote_details.quote_number, si no es NULL
            o issued_number, si issued_content_hash = huella de contenido actual
            o "<año de emisión>-<last_number + 1 con al menos 3 cifras>"

al guardar el PDF, si el número usado cumple ^\d{4}-\d{3,9}$:
    last_number(año del número) = MAX(last_number, número usado)
```

---

## Valores derivados (no se guardan)

```text
issue_date   = fecha local del día de la generación
valid_until  = issue_date + validity_days
file_name    = "Presupuesto <número> <aaaa-mm-dd>.pdf"   (caracteres no admitidos del número → "-")
token        = SHA-256 del contenido completo del documento
```

### Huella de contenido

SHA-256 de las líneas en orden (concepto, cantidad, unidad y precio) y del tipo de IVA. No incluye el cliente, las fechas, la validez ni el número.

### Contenido del token

Número, fechas, datos del cliente, datos del negocio, huella del logo, líneas en orden, tipo de IVA, base, IVA y total.

---

## Condiciones para generar

| Bloqueo | Condición | Requisito |
|---|---|---|
| `noLines` | El borrador no tiene líneas | FR-001 |
| `pendingLines` | Alguna línea sin precio o sin cantidad | FR-002 |
| `notReviewed` | El estado no es `reviewed` | FR-001 |
| `profileIncomplete` | El perfil tiene datos obligatorios sin completar | FR-003 |
| `clientNameRequired` | `client_name` está vacío | FR-012 |
| `quoteNumberRequired` | El número está vacío | FR-001 |
| `unsupportedCharacters` | Algún texto no se puede representar | FR-018b |
| `logoUnreadable` | El logo no se puede leer y no se ha aceptado seguir sin él | Casos límite |

## Avisos en la confirmación

| Aviso | Condición |
|---|---|
| `noLogo` | El negocio no tiene logo |
| `logoUnreadable` | El profesional ha aceptado seguir sin el logo ilegible. Hasta entonces, actúa como bloqueo |
| `vatChanged` | El IVA del negocio difiere del aplicado al borrador. No bloquea; actualizarlo con `draft_apply_current_vat` devuelve el borrador a `draft` (FR-005b) |
| `unparsedFragments` | Hay fragmentos del dictado sin interpretar |

---

## Estados de la generación

```text
preparado ──confirmar──▶ eligiendo ubicación ──elegir──▶ generando ──▶ guardado
    ▲                          │                             │
    │                       cancelar                       error
    └──────────────────────────┴─────────────────────────────┘
       (sin archivo, sin gastar número y con el borrador intacto)
```

Solo puede haber una generación activa a la vez (FR-026).

---

## Migraciones

| Versión | Contenido |
|---|---|
| 3 | Creación de `quote_details` y `quote_number_sequences` |

## Archivos en disco

```text
<carpeta elegida>/
├── Presupuesto 2026-001 2026-11-18.pdf  # documento final
└── .presupuesto.tmp                     # solo mientras se escribe; la siguiente generación sustituye el que quede tras un cierre inesperado
```
