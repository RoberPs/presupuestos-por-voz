# Contrato IPC: PDF del presupuesto

**Feature**: `003-quote-pdf-export` | **Fecha**: 2026-11-18

Amplía el [contrato de la feature 002](../../002-voice-quote-draft/contracts/ipc-commands.md), cuyas convenciones se mantienen: `invoke(nombre, args)`, campos en `camelCase` y rechazo de la promesa con un `AppError`.

## Errores nuevos

```ts
type AppError =
  | /* errores de las features 001 y 002 */
  | { kind: "quoteBlocked"; blockers: QuoteBlocker[]; message: string }
  | { kind: "quoteChanged"; message: string }
  | { kind: "pdfSave"; code: PdfSaveCode; message: string }
  | { kind: "pdfRender"; message: string };

type PdfSaveCode =
  | "PDF_ACCESS_DENIED"
  | "PDF_DISK_FULL"
  | "PDF_LOCATION_UNAVAILABLE"
  | "PDF_FILE_IN_USE"
  | "PDF_WRITE_FAILED";

type ErrorCode =
  | /* códigos de las features 001 y 002 */
  | "VALIDITY_INVALID";
```

## Vistas

```ts
type QuoteDetailsInput = {
  clientName: string;
  clientAddress: string | null;
  clientPhone: string | null;
  clientEmail: string | null;
  clientTaxId: string | null;
  quoteNumber: string;
  validityDays: number;
};

type QuoteBlocker =
  | { kind: "noLines" }
  | { kind: "pendingLines"; lines: { id: number; concept: string }[] }
  | { kind: "notReviewed" }
  | { kind: "profileIncomplete"; missingFields: FieldError[] }
  | { kind: "clientNameRequired" }
  | { kind: "quoteNumberRequired" }
  | { kind: "unsupportedCharacters"; fields: { field: string; label: string; characters: string[] }[] }
  | { kind: "logoUnreadable" };

type QuoteWarning =
  | { kind: "noLogo" }
  | { kind: "logoUnreadable" }
  | { kind: "vatChanged"; appliedRatePercent: string; currentRatePercent: string }
  | { kind: "unparsedFragments"; count: number };

type QuotePreparationView = {
  details: QuoteDetailsInput;
  issueDateDisplay: string;          // "18/11/2026"
  validUntilDisplay: string;         // "18/12/2026"
  lineCount: number;
  totalDisplay: string;              // "163,35 €"
  blockers: QuoteBlocker[];
  warnings: QuoteWarning[];
  confirmationToken: string | null;  // null mientras haya bloqueos
};

type QuoteGenerateResult =
  | { outcome: "saved"; fileName: string; fullPath: string; pageCount: number }
  | { outcome: "cancelled" };
```

---

## Comandos

### `quote_prepare(args: { allowMissingLogo: boolean })` → `QuotePreparationView`

Calcula lo que se va a emitir: datos guardados, número propuesto, fechas, bloqueos y avisos (FR-001 a FR-005). No crea archivos ni gasta números.

Con `allowMissingLogo: true`, un logo ilegible deja de bloquear y el documento sale sin logo.

Errores: `invalidState` si no hay borrador.

### `quote_details_save(args: { details: QuoteDetailsInput; allowMissingLogo: boolean })` → `QuotePreparationView`

Guarda los datos del cliente, el número y la validez (FR-012b). No cambia el estado de revisión del borrador. La interfaz lo llama al salir de cada campo y siempre al pulsar confirmar: espera su respuesta y llama a `quote_generate` con el `confirmationToken` que devuelve. Si la respuesta trae bloqueos, los muestra y no genera. El botón de confirmar no se desactiva por tener datos sin guardar.

Un nombre o un número vacíos se guardan y aparecen como bloqueo. Errores: `validation` con `TOO_LONG` o `VALIDITY_INVALID`; `invalidState` si no hay borrador.

### `quote_generate(args: { confirmationToken: string; allowMissingLogo: boolean })` → `QuoteGenerateResult`

Es la confirmación explícita del profesional (FR-004). En orden:

1. Comprueba que no haya otra generación activa y que el token siga vigente.
2. Abre el diálogo nativo de guardado, con el nombre propuesto y filtro PDF (FR-020). Si se cancela, devuelve `cancelled`.
3. Vuelve a comprobar el token y añade `.pdf` al nombre si falta.
4. Genera el documento.
5. Guarda el archivo de forma atómica y gasta el número en la misma operación (FR-013b y FR-024).

Tras un error de guardado, el diálogo se reabre con la última carpeta y el último nombre elegidos, sin repetir la confirmación (FR-023).

| Error | Cuándo |
|---|---|
| `quoteBlocked` | Hay bloqueos |
| `quoteChanged` | El contenido ha cambiado desde la confirmación |
| `invalidState` | Ya hay una generación activa o no hay borrador |
| `pdfSave` | El archivo no se ha podido guardar |
| `pdfRender` | El documento no se ha podido componer |

---

## Cambios en comandos existentes

| Comando | Cambio |
|---|---|
| `draft_generate_from_text` | Con `mode: "regenerate"` conserva `quote_details` |

---

## Garantías transversales

- Ningún comando hace peticiones de red (FR-027 y FR-028).
- Cancelar o fallar no crea archivos, no gasta números y no modifica el borrador (FR-024 y FR-025).
- La interfaz no calcula importes, fechas ni números: los recibe del núcleo.
- Los `message` están en español sencillo. El detalle técnico solo va al log local (FR-029), sin datos del cliente ni contenido del presupuesto (FR-028b).
- No se añaden plugins ni permisos: el diálogo se abre desde Rust y `capabilities/default.json` no cambia.
