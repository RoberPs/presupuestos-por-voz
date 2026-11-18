# Contrato IPC: dictado y borrador

**Feature**: `002-voice-quote-draft` | **Fecha**: 2026-11-18

Amplía el [contrato de la feature 001](../../001-business-profile-rates/contracts/ipc-commands.md), cuyas convenciones se mantienen: `invoke(nombre, args)`, campos en `camelCase` y rechazo de la promesa con un `AppError`.

## Errores nuevos

```ts
type AppError =
  | /* errores de la feature 001 */
  | { kind: "microphone"; code: "MIC_NOT_FOUND" | "MIC_ACCESS_DENIED" | "MIC_UNAVAILABLE"; message: string }
  | { kind: "audioRejected"; code: "AUDIO_UNSUPPORTED_FORMAT" | "AUDIO_CORRUPT" | "AUDIO_TOO_LONG"; message: string }
  | { kind: "transcription"; code: "MODEL_UNAVAILABLE" | "TRANSCRIPTION_FAILED"; message: string }
  | { kind: "confirmationRequired"; reason: "replaceDraft" | "replacePendingAudio" | "discardManualChanges"; message: string }
  | { kind: "invalidState"; message: string }
  | { kind: "draftHasPending"; pendingLineIds: number[]; message: string };

type ErrorCode =
  | /* códigos de la feature 001 */
  | "QUANTITY_REQUIRED" | "QUANTITY_INVALID_FORMAT" | "QUANTITY_TOO_MANY_DECIMALS" | "QUANTITY_NOT_POSITIVE"
  | "AMOUNT_TOO_LARGE" | "TEXT_REQUIRED";
```

## Vistas

```ts
type SessionView = {
  activity:
    | { kind: "idle" }
    | { kind: "recording"; elapsedMs: number }
    | { kind: "processing"; percent: number | null };
  dictation: DictationView | null;   // la ranura incoming si existe; si no, current
  draft: DraftView | null;
  pendingText: string | null;        // texto escrito que aún no tiene borrador (FR-035)
  recovery: { durationMs: number } | null;   // grabación interrumpida que se puede recuperar
};

type DictationView = {
  origin: "recorded" | "imported";
  originalFileName: string | null;
  durationMs: number;
  status: "recorded" | "processing" | "transcribed" | "failed" | "noSpeech";
  lowQuality: boolean;
  errorCode: "MODEL_UNAVAILABLE" | "TRANSCRIPTION_FAILED" | null;
  transcribedLocally: boolean;       // FR-039
};

type DraftView = {
  status: "draft" | "reviewed";
  sourceText: string;
  textOrigin: "transcribed" | "typed";
  textCorrected: boolean;
  hasManualChanges: boolean;
  lines: DraftLineView[];
  unparsedFragments: string[];
  totals: {
    baseCents: number; baseDisplay: string;
    vatRatePercent: string;
    vatCents: number; vatDisplay: string;
    totalCents: number; totalDisplay: string;
    pendingCount: number;
    isProvisional: boolean;
  };
  vatChange: { currentRatePercent: string } | null;   // FR-029b
};

type DraftLineView = {
  id: number;
  concept: string;
  quantity: string | null;           // "1,5"
  quantityIssue: "missing" | "ambiguous" | "rectified" | null;
  unitLabel: string | null;
  priceCents: number | null;
  priceDisplay: string | null;
  amountCents: number | null;
  amountDisplay: string | null;
  priceSource: "rate" | "manual" | "pending";
  rateId: number | null;
  rateConcept: string | null;
  pendingReason: "noRate" | "ambiguousRate" | "similarRate" | "unitMismatch" | null;
  suggestions: Rate[];               // tipo Rate de la feature 001
  sourceFragment: string | null;
};
```

---

## Sesión

### `session_get()` → `SessionView`

Estado completo al abrir la pantalla, incluida una grabación recuperable (FR-035).

## Dictado

### `dictation_start(args: { confirmReplace: boolean; onEvent: Channel<RecordingEvent> })` → `void`

Empieza a grabar en un archivo temporal. El borrador actual no se toca hasta generar el nuevo.

```ts
type RecordingEvent =
  | { type: "level"; elapsedMs: number; level: number }          // 0–1, unas 10 veces por segundo
  | { type: "limitWarning"; remainingMs: number }                // a los 9 minutos (FR-005)
  | { type: "stopped"; reason: "limitReached" | "deviceLost"; durationMs: number };
```

Errores: `microphone`; `confirmationRequired` si `confirmReplace` es `false` y ya hay borrador (`replaceDraft`) o hay una ranura `incoming` o una grabación recuperable (`replacePendingAudio`); `invalidState` si ya se está grabando o procesando.

### `dictation_stop()` → `SessionView`

Cierra la grabación y crea la ranura `incoming` en estado `recorded`. La interfaz llama después a `dictation_process`. Si no se ha grabado audio, descarta el archivo, no crea la ranura `incoming` y la interfaz muestra el aviso de FR-006.

### `dictation_import(args: { confirmReplace: boolean })` → `SessionView | null`

Abre el diálogo nativo (M4A, MP3, WAV). Devuelve `null` si el usuario cancela. Valida y normaliza el archivo antes de crear la ranura `incoming`. Si lo rechaza, el error es `audioRejected` y el borrador en curso queda intacto (FR-002c). Exige la misma confirmación que `dictation_start` (`replaceDraft` o `replacePendingAudio`).

### `dictation_process(args: { onProgress: Channel<ProcessingEvent> })` → `ProcessResult`

Transcribe la ranura `incoming` y, si hay texto, genera el borrador en una única transacción que sustituye al anterior (FR-010). También sirve para reintentar tras un fallo (FR-004).

```ts
type ProcessingEvent =
  | { type: "stage"; stage: "preparing" | "transcribing" | "buildingDraft" }
  | { type: "progress"; percent: number }
  | { type: "finished"; outcome: "draftReady" | "noSpeech" | "cancelled" | "failed" };

type ProcessResult = { outcome: "draftReady" | "noSpeech" | "cancelled"; session: SessionView };
```

Errores: `transcription` (el audio se conserva y el estado pasa a `failed`); `invalidState` si no hay audio pendiente.

### `dictation_cancel_processing()` → `void`

Pide cancelar el proceso en curso. `dictation_process` termina con `cancelled` y el audio se conserva (FR-009).

### `dictation_subscribe(args: { onRecording: Channel<RecordingEvent>; onProgress: Channel<ProcessingEvent> })` → `void`

Reconecta los eventos de la grabación o del proceso en curso al volver a la pantalla. La interfaz lo llama al montarse si `activity` no es `idle`. Sustituye a los canales anteriores.

### `dictation_recover()` → `SessionView`

Convierte la grabación interrumpida en una ranura `incoming`.

### `dictation_discard()` → `SessionView`

Descarta la grabación recuperable o la ranura `incoming`. El borrador vigente no cambia.

## Borrador

### `draft_generate_from_text(args: { text: string; mode: "new" | "regenerate"; confirmed: boolean })` → `DraftView`

- `new`: crea un borrador desde texto escrito o pegado (FR-013). Si ya hay contenido, exige `confirmed`: `replaceDraft` si hay borrador y `replacePendingAudio` si hay un audio sin procesar o una grabación recuperable.
- `regenerate`: rehace el borrador con el texto editado (FR-011). Si hay cambios manuales, exige `confirmed` (`discardManualChanges`) (FR-012).
- Texto vacío: `validation` con `TEXT_REQUIRED`.

El resultado depende solo del texto, las tarifas y las unidades (FR-014).

### `draft_text_save(args: { text: string })` → `void`

Guarda el texto mientras se edita, sin regenerar (FR-035). La interfaz lo llama con *debounce* de 500 ms. Si aún no hay borrador, lo guarda como `pendingText`, que se vacía al generar el borrador.

### `draft_line_add(input: DraftLineInput)` → `DraftView`

```ts
type DraftLineInput =
  | { kind: "rate"; rateId: number; quantity: string }
  | { kind: "manual"; concept: string; quantity: string; unitLabel: string; price: string };
```

### `draft_line_update(args: { lineId: number; patch: DraftLinePatch })` → `DraftView`

```ts
type DraftLinePatch = { concept?: string; quantity?: string; unitLabel?: string; price?: string };
```

Aplica las reglas de edición de [data-model.md](../data-model.md). Errores: `validation` (`CONCEPT_REQUIRED`, `UNIT_REQUIRED`, `PRICE_*`, `QUANTITY_*`, `AMOUNT_TOO_LARGE`), `notFound`.

### `draft_line_assign_rate(args: { lineId: number; rateId: number })` → `DraftView`

Completa una línea pendiente con una tarifa (FR-026). Errores: `notFound`.

### `draft_line_delete(args: { lineId: number })` → `DraftView`

La confirmación la hace la interfaz (FR-025).

### `draft_apply_current_vat()` → `DraftView`

Aplica al borrador el IVA actual del negocio, a petición del profesional (FR-029b).

### `draft_mark_reviewed()` → `DraftView`

Errores: `draftHasPending` con las líneas por completar; `invalidState` si no hay líneas (FR-034).

### `draft_discard()` → `SessionView`

Borra el borrador, su texto y su audio. La confirmación la hace la interfaz (FR-036).

---

## Garantías transversales

- Ningún comando hace peticiones de red (FR-037, FR-038).
- Toda escritura en SQLite se hace en una transacción. Sustituir un borrador por otro es atómico.
- Solo puede haber una grabación o un proceso activos a la vez.
- Los comandos de borrador devuelven los importes ya calculados por el núcleo: la interfaz no hace cálculos de dinero.
- Los `message` están en español sencillo. El detalle técnico solo va al log local (FR-040).
- No se añaden plugins ni permisos: `Channel` forma parte de `core:default` y el diálogo usa `dialog:allow-open`.
