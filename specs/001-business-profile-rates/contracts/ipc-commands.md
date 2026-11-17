# Contrato IPC: comandos Tauri (Rust ⇄ React)

**Feature**: `001-business-profile-rates` | **Fecha**: 2026-11-17

Es la única interfaz entre la interfaz (WebView) y el núcleo. Se invoca con `invoke(nombre, args)` desde `src/shared/ipc/`. Los nombres de campo van en `camelCase` en JSON (`serde(rename_all = "camelCase")`). Los tipos TypeScript equivalentes viven en `src/shared/ipc/types.ts` y deben mantenerse sincronizados con las *structs* de Rust.

## Tipos comunes

```ts
type FieldError = { field: string; code: ErrorCode; message: string }; // message en español sencillo

type AppError =
  | { kind: "validation"; errors: FieldError[] }
  | { kind: "duplicateRate"; existingRateId: number; message: string }
  | { kind: "notFound"; message: string }
  | { kind: "logoRejected"; code: "LOGO_TOO_LARGE" | "LOGO_UNSUPPORTED_FORMAT" | "LOGO_CORRUPT"; message: string }
  | { kind: "storage"; message: string }; // fallo inesperado de disco o base de datos

type ErrorCode =
  | "REQUIRED" | "TOO_LONG"
  | "TAX_ID_INVALID" | "POSTAL_CODE_INVALID" | "PHONE_INVALID" | "EMAIL_INVALID"
  | "VAT_OUT_OF_RANGE"
  | "PRICE_REQUIRED" | "PRICE_INVALID_FORMAT" | "PRICE_TOO_MANY_DECIMALS" | "PRICE_NOT_POSITIVE"
  | "UNIT_REQUIRED" | "CONCEPT_REQUIRED";
```

Los comandos devuelven `Promise<T>`; en caso de error la promesa se rechaza con un `AppError`.

---

## Perfil del negocio

### `profile_get` → `BusinessProfileView | null`

Devuelve `null` en el primer arranque (no hay fila). Ese caso activa el modo guiado (FR-001c).

```ts
type BusinessProfileView = {
  tradeName: string; legalName: string | null; taxId: string;
  phone: string; email: string | null; website: string | null;
  address: { street: string; postalCode: string; city: string; province: string };
  vatRatePercent: string;          // "21,00" (formato es-ES, sin redondeo)
  currency: "EUR";
  logo: LogoView | null;
  status: "complete" | "incomplete";
  missingFields: FieldError[];     // vacío si está completo
};
type LogoView = { dataUrl: string; mime: "image/png" | "image/jpeg"; widthPx: number; heightPx: number; lowResolution: boolean };
```

### `profile_save(input: BusinessProfileInput)` → `BusinessProfileView`

Crea o actualiza la fila única. **Acepta borradores**: los campos vacíos o inválidos no impiden guardar (FR-008); se devuelven en `missingFields`. Solo rechaza con `validation` los errores que no se pueden guardar: `TOO_LONG` y `VAT_OUT_OF_RANGE`.

```ts
type BusinessProfileInput = {
  tradeName: string; legalName?: string; taxId: string; phone: string;
  email?: string; website?: string;
  address: { street: string; postalCode: string; city: string; province: string };
  vatRatePercent: string;          // admite "21", "21,5", "10.00"
};
```

### `profile_validate(input: BusinessProfileInput)` → `FieldError[]`

Valida sin guardar, para mostrar los errores mientras se escribe (con *debounce* de 300 ms en la interfaz).

### `logo_pick_and_set()` → `{ logo: LogoView; warnings: string[] } | null`

1. Abre el diálogo nativo (filtro PNG o JPG).
2. Devuelve `null` si el usuario cancela.
3. Valida el archivo: tamaño ≤ 5 MB, *magic bytes* y decodificación.
4. Si es válido, lo copia de forma atómica a `branding/`, actualiza el perfil y borra el logo anterior.
5. Si lo rechaza, el error es `logoRejected` y el logo anterior queda intacto (FR-003).

`warnings` incluye el aviso de baja resolución (< 200 px en el lado más corto) (FR-003b).

Si aún no hay fila de perfil, crea un borrador vacío para poder asociar el logo.

### `logo_remove()` → `BusinessProfileView`

Quita el logo del perfil y borra el archivo. La interfaz pide confirmación antes de llamarlo.

---

## Unidades

### `units_list()` → `Unit[]`

```ts
type Unit = { id: number; label: string; isPredefined: boolean };
```

Primero las predefinidas, en el orden de las semillas; después las personalizadas, en orden alfabético.

---

## Tarifas

```ts
type Rate = {
  id: number; concept: string;
  unit: Unit;
  priceCents: number;
  priceDisplay: string;   // "1.234,56 €"
};
type RateInput = {
  concept: string;
  unitLabel: string;      // etiqueta existente o nueva unidad personalizada
  price: string;          // texto tal cual lo escribe el usuario (es-ES)
};
```

### `rates_list(query?: string)` → `Rate[]`

- Sin `query` o con `query` vacío: todas las tarifas, en orden alfabético (FR-017).
- Con `query`: solo las tarifas cuyo `concept_norm` contiene `normalize(query)` (FR-020, FR-021).
- Una lista vacía es un resultado válido: la interfaz distingue "no hay tarifas" de "sin resultados" según haya `query` o no (FR-018, FR-022).

### `rates_create(input: RateInput)` → `Rate`

Errores posibles:
- `validation`: `CONCEPT_REQUIRED`, `UNIT_REQUIRED`, `PRICE_*`, `TOO_LONG`.
- `duplicateRate`: ya existe una tarifa con el mismo concepto normalizado y la misma unidad (FR-013).

Si `unitLabel` no existe, crea la unidad personalizada en la misma transacción.

### `rates_update(id: number, input: RateInput)` → `Rate`

Mismas validaciones que la creación (FR-014). El duplicado se comprueba excluyendo la propia tarifa. Devuelve `notFound` si el `id` no existe.

### `rates_delete(id: number)` → `void`

Borrado definitivo; la confirmación la hace la interfaz (FR-015). Devuelve `notFound` si el `id` no existe.

---

## Garantías transversales

- Ningún comando hace peticiones de red (FR-023, FR-024).
- Toda escritura se hace dentro de una transacción SQLite. Si un comando falla, no deja cambios a medias.
- Los `message` están en español, sin jerga técnica ni trazas internas (FR-025). El detalle técnico solo va al log local de la app.
