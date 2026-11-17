---

description: "Lista de tareas para implementar Perfil del negocio y tarifas"
---

# Tasks: Perfil del negocio y tarifas

**Input**: Documentos de diseño en `/specs/001-business-profile-rates/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/ipc-commands.md](contracts/ipc-commands.md), [quickstart.md](quickstart.md)

**Tests**: se incluyen porque la estrategia de pruebas está definida en research.md (R10) y en quickstart.md. En cada historia, las pruebas se escriben primero y deben fallar antes de implementar.

**Organization**: tareas agrupadas por historia de usuario, para implementar y probar cada una por separado.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: se puede ejecutar en paralelo (otro archivo, sin dependencias pendientes).
- **[Story]**: historia a la que pertenece (US1–US4, según spec.md).
- Rutas relativas a la raíz del repositorio: `src/` es la interfaz React y `src-tauri/` el núcleo Rust (ver plan.md, Project Structure).
- El crate de la librería Rust se llama `app_lib`; las pruebas de integración en `src-tauri/tests/` usan `app_lib::…`, así que los módulos `domain`, `services`, `storage` y `error` deben ser `pub`.
- **Registro de comandos**: todos los comandos se registran en `src-tauri/src/lib.rs`. Las tareas que lo modifican no son paralelas entre sí.

---

## Phase 1: Setup (infraestructura compartida)

**Purpose**: crear el proyecto Tauri 2 + React + TypeScript y su configuración base.

- [ ] T001 Crear el esqueleto de Tauri 2 con plantilla React + TypeScript + Vite en la raíz del repositorio: `package.json`, `index.html`, `vite.config.ts`, `tsconfig.json`, `src/main.tsx`, `src-tauri/Cargo.toml` (`[lib] name = "app_lib"`), `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` y `src-tauri/tauri.conf.json` (`productName` "Presupuestos por Voz", `identifier` "es.presupuestosvoz.app", ventana de 1100×760, mínimo 900×640).
- [ ] T002 Añadir dependencias Rust en `src-tauri/Cargo.toml`: `tauri` 2, `tauri-plugin-dialog` 2, `tauri-plugin-log` 2, `rusqlite` (feature `bundled`), `rusqlite_migration`, `image` (`default-features = false`, features `png` y `jpeg`), `unicode-normalization`, `sha2`, `hex`, `base64`, `serde` (derive), `serde_json`, `thiserror` y `log`. En dev-dependencies, `tempfile`.
- [ ] T003 [P] Añadir dependencias npm en `package.json`: `react-router-dom@6`, `react-hook-form`, `@tauri-apps/api@2` y `@tauri-apps/plugin-dialog@2`. En dev: `vitest`, `jsdom`, `@testing-library/react`, `@testing-library/user-event`, `@testing-library/jest-dom`, `eslint`, `typescript-eslint`, `eslint-plugin-react-hooks` y `prettier`. Scripts: `dev`, `build`, `tauri`, `test` (vitest run), `lint`, `typecheck` (tsc --noEmit), `test:e2e` y `seed:rates`.
- [ ] T004 [P] Configurar Vitest con entorno `jsdom` y `setupFiles` en `vite.config.ts`, y crear `src/test/setup.ts` que importe `@testing-library/jest-dom` y limpie `mockIPC` (`clearMocks` de `@tauri-apps/api/mocks`) después de cada prueba.
- [ ] T005 [P] Configurar el lint y el formato: `eslint.config.js` (typescript-eslint + react-hooks), `.prettierrc` y `src-tauri/rustfmt.toml` (edición 2021).
- [ ] T006 [P] Crear `.gitignore` con `node_modules/`, `dist/`, `src-tauri/target/` y `src-tauri/gen/`.
- [ ] T007 [P] Endurecer la seguridad (R9). En `src-tauri/tauri.conf.json`, fijar la CSP `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self'`. En `src-tauri/capabilities/default.json`, conceder solo `core:default`, `dialog:allow-open` y `log:default`. No instalar plugins HTTP, updater ni de telemetría.

---

## Phase 2: Foundational (requisitos que bloquean todo)

**Purpose**: base de datos, errores, normalización, dinero, cliente IPC y armazón de la interfaz, que usan todas las historias.

**⚠️ CRITICAL**: ninguna historia puede empezar hasta terminar esta fase.

- [ ] T008 Crear `src-tauri/src/storage/migrations/001_init.sql` con las tablas `business_profile` (`CHECK (id = 1)`, `CHECK (currency = 'EUR')`, `vat_rate_bp` con `DEFAULT 2100` y `CHECK` entre 0 y 10000, columnas de logo nulables), `units` (`label_norm UNIQUE`, `is_predefined`) y `rates` (`price_cents CHECK > 0`, FK `unit_id`, `UNIQUE(concept_norm, unit_id)`), más `created_at` y `updated_at`. Sembrar las unidades `unidad`, `hora`, `metro`, `m²` y `metro lineal` con `is_predefined = 1` y su `label_norm`. Todo según data-model.md.
- [ ] T009 Implementar `src-tauri/src/storage/db.rs` y `src-tauri/src/storage/mod.rs`: `open(data_dir) -> Connection` (archivo `presupuestos.db`, `PRAGMA journal_mode=WAL`, `foreign_keys=ON`, migraciones con `rusqlite_migration` e `include_str!` del SQL) y `open_in_memory()` para pruebas. Crear `src-tauri/src/state.rs` con `AppState { conn: Mutex<Connection>, data_dir: PathBuf }`.
- [ ] T010 [P] Implementar `src-tauri/src/error.rs`: enum `AppError` (`Validation { errors: Vec<FieldError> }`, `DuplicateRate { existing_rate_id, message }`, `NotFound { message }`, `LogoRejected { code, message }`, `Storage { message }`), `FieldError { field, code, message }` y enum `ErrorCode` con todos los códigos del contrato. Se serializa con `#[serde(tag = "kind", rename_all = "camelCase")]` para coincidir con `AppError` de [contracts/ipc-commands.md](contracts/ipc-commands.md). `From<rusqlite::Error>` y `From<std::io::Error>` producen `Storage` con mensaje en español sencillo ("No se ha podido guardar. Inténtalo de nuevo.") y registran el detalle con `log::error!`.
- [ ] T011 [P] Implementar `src-tauri/src/domain/normalize.rs` (R5): `normalize(s) -> String` (NFD, quitar marcas `Mn`, minúsculas, `trim`, colapsar espacios) y `escape_like(s)` (escapa `\`, `%` y `_` con `\`). Incluir `#[cfg(test)]` con casos "  Instalación  de GRIFO " → "instalacion de grifo", "Pintura" == "pintura" y "m²" estable.
- [ ] T012 [P] Implementar `src-tauri/src/domain/money.rs` (R4):
  - `parse_price_cents(&str) -> Result<i64, ErrorCode>` con las reglas y la tabla de data-model.md (vacío → `PRICE_REQUIRED`, formato → `PRICE_INVALID_FORMAT`, más de 2 decimales → `PRICE_TOO_MANY_DECIMALS`, ≤ 0 → `PRICE_NOT_POSITIVE`; "1.234,56" → 123456; "12.50" → 1250).
  - `format_eur(cents) -> "1.234,56 €"`.
  - `parse_percent_bp(&str) -> Result<i64, ErrorCode>` (0–100 con hasta 2 decimales; fuera de rango → `VAT_OUT_OF_RANGE`).
  - `format_percent_bp(2100) -> "21,00"`.
  - `#[cfg(test)]` que cubra cada fila de la tabla de data-model.md.
- [ ] T013 Crear `src-tauri/src/domain/mod.rs` (declara `normalize`, `money`, `validation`, `profile` y `rate`; los dos últimos se crean vacíos si aún no existen). En `src-tauri/src/lib.rs`, declarar los módulos `pub` y, en `setup`, crear `app_data_dir`, abrir la base de datos, registrar `AppState` con `manage`, inicializar `tauri-plugin-dialog` y `tauri-plugin-log` (solo destinos de archivo local y stdout) y dejar un `invoke_handler` vacío listo para añadir comandos.
- [ ] T014 [P] Crear la prueba de integración `src-tauri/tests/db_migrations.rs`: las migraciones se aplican en memoria y en un archivo de `tempfile::tempdir()`; hay 5 unidades predefinidas; un `INSERT` en `rates` con `price_cents = 0` falla; un duplicado `(concept_norm, unit_id)` falla; una segunda fila en `business_profile` falla.
- [ ] T015 [P] Crear `src/shared/ipc/types.ts`, espejo exacto de los tipos del contrato (`FieldError`, `AppError`, `ErrorCode`, `BusinessProfileView`, `BusinessProfileInput`, `LogoView`, `Unit`, `Rate` y `RateInput`), y `src/shared/ipc/client.ts` con una función tipada por comando (`profileGet`, `profileSave`, `profileValidate`, `logoPickAndSet`, `logoRemove`, `unitsList`, `ratesList`, `ratesCreate`, `ratesUpdate` y `ratesDelete`) sobre `invoke`, más un *type guard* `isAppError(e)`.
- [ ] T016 [P] Crear `src/shared/i18n/es.ts` con todos los textos de la interfaz en español sencillo (FR-025): títulos, etiquetas de campos, botones, estados vacío y sin resultados, avisos de perfil incompleto, de precios sin IVA y de logo de baja resolución, y diálogos de confirmación y de cambios sin guardar.
- [ ] T017 [P] Crear `src/shared/styles/tokens.css` y `src/shared/styles/global.css`: letra base de 16 px, objetivos táctiles de al menos 44 px, paleta con contraste AA, foco visible con `:focus-visible` y fuentes del sistema (sin CDN). Importarlos en `src/main.tsx`.
- [ ] T018 [P] Crear los componentes compartidos en `src/shared/components/` (cada uno con su `.module.css`):
  - `Button.tsx` (variantes `primary`, `secondary` y `danger`).
  - `Field.tsx` (label, input o slot, mensaje de error enlazado con `aria-describedby`, `aria-invalid`).
  - `ConfirmDialog.tsx` (modal accesible con foco atrapado, título, mensaje y botones configurables).
  - `EmptyState.tsx`.
  - `Banner.tsx` (variantes `info` y `warning`, con acción opcional).
  - `Toast.tsx` (confirmación no bloqueante con `aria-live="polite"`).
- [ ] T019 Crear `src/app/routes.tsx` con `createMemoryRouter` (rutas `/negocio` y `/tarifas`, inicial `/negocio`) y `src/app/App.tsx` con el layout (navegación lateral "Mi negocio" y "Tarifas", `<Outlet/>` y zona de banners y toasts). Montarlo en `src/main.tsx` con `RouterProvider`. Se usa un *data router* porque `useBlocker` lo exige.

**Checkpoint**: `npm run tauri dev` abre la app con las dos secciones vacías, y `cargo test` y `npm run test` pasan.

---

## Phase 3: User Story 1 - Guardar los datos del negocio y el logo (Priority: P1) 🎯 MVP

**Goal**: el profesional rellena los datos de su negocio, comprobados en formato, y carga un logo válido. Todo se conserva entre sesiones. En el primer arranque se abre la configuración guiada y, si el perfil está incompleto, se muestra un aviso permanente.

**Independent Test**: escenarios 1–7 y 10 de [quickstart.md](quickstart.md). Rellenar los datos, cargar el logo, cerrar y reabrir: todo debe aparecer idéntico, también sin red.

### Tests for User Story 1 ⚠️

> Escribir primero y comprobar que fallan.

- [ ] T020 [P] [US1] Crear `src-tauri/tests/validation.rs`:
  - DNI válido `12345678Z` e inválido `12345678A`.
  - NIE válido `X1234567L` e inválido.
  - CIF válidos `A58818501` y `B65410011`, y CIF con control erróneo.
  - Normalización de `12.345.678-z` → `12345678Z`.
  - Código postal `28001` válido; `5300`, `53001` y `00123` inválidos.
  - Teléfonos `600 123 456`, `+34 912345678` y `+44 20 7946 0958` válidos; `12345` y `500123456` inválidos.
  - Email `a@b.es` válido; `a@b` y `@b.es` inválidos.
- [ ] T021 [P] [US1] Crear `src-tauri/tests/profile_service.rs` con una base de datos de archivo en `tempdir`:
  - `get_profile` devuelve `None` en una base de datos nueva.
  - Guardar un borrador con campos vacíos devuelve `status = incomplete` con la lista exacta de `missing_fields`.
  - Guardar datos completos y válidos devuelve `complete`.
  - Un NIF inválido se guarda, pero deja el perfil `incomplete` con `TAX_ID_INVALID`.
  - IVA "150" → `Validation` con `VAT_OUT_OF_RANGE`; un nombre de 121 caracteres → `TOO_LONG`.
  - Cerrar y reabrir la conexión devuelve los mismos datos.
  - `vatRatePercent` es "21,00" por defecto.
- [ ] T022 [P] [US1] Crear `src-tauri/tests/logo_service.rs`, con imágenes de prueba generadas en el propio test con el crate `image` en `tempdir`:
  - Archivo de 6 MB → `LOGO_TOO_LARGE`.
  - Texto o PDF renombrado a `.png` → `LOGO_UNSUPPORTED_FORMAT`.
  - PNG truncado → `LOGO_CORRUPT`.
  - PNG RGBA de 150×150 → se acepta con `low_resolution = true` y un aviso.
  - JPG de 800×400 → se acepta sin aviso.
  - El archivo guardado en `branding/` tiene el mismo SHA-256 que el original (sin recodificar).
  - Tras un rechazo, el logo anterior y su archivo siguen intactos.
  - Sin fila de perfil previa, `set_logo` crea un borrador.
- [ ] T023 [P] [US1] Crear `src/features/business-profile/BusinessProfilePage.test.tsx` con `mockIPC`:
  - `profile_get` devuelve `null` → se ve el modo guiado y el botón "Saltar".
  - Pulsar "Saltar" muestra el banner de perfil incompleto.
  - Los errores de `profile_validate` aparecen junto a cada campo.
  - Guardar llama a `profile_save` con la dirección en cuatro campos y muestra un toast de confirmación.
  - Una respuesta `logoRejected` muestra el mensaje y conserva la vista previa anterior.

### Implementation for User Story 1

- [ ] T024 [P] [US1] Implementar `src-tauri/src/domain/validation/{mod.rs,nif.rs,postal_code.rs,phone.rs,email.rs}` según R6:
  - `normalize_tax_id`, `validate_tax_id` (DNI, NIE y CIF con algoritmo de control oficial).
  - `validate_postal_code` (5 dígitos, prefijo 01–52).
  - `validate_phone` (9 dígitos 6/7/8/9 con `+34`/`0034` opcional, o internacional de 6 a 15 dígitos).
  - `validate_email` (sintaxis básica).

  Cada función devuelve `Result<String, ErrorCode>` con el valor normalizado.
- [ ] T025 [US1] Implementar `src-tauri/src/domain/profile.rs`:
  - *Structs* `BusinessProfile`, `Address`, `LogoMeta` y `BusinessProfileInput` (serde camelCase).
  - Límites de longitud de data-model.md.
  - `hard_errors(&input) -> Vec<FieldError>` (`TOO_LONG`, `VAT_OUT_OF_RANGE`; bloquean el guardado).
  - `missing_fields(&profile) -> Vec<FieldError>` (obligatorios vacíos con `REQUIRED` y formatos inválidos con su código, usando los validadores de T024; los `message` salen de constantes en español).
  - `status()`.
- [ ] T026 [US1] Implementar `src-tauri/src/storage/profile_repo.rs`: `get(&Connection) -> Option<BusinessProfile>`, `upsert(&Transaction, &BusinessProfile)` (`INSERT … ON CONFLICT(id) DO UPDATE`, actualiza `updated_at`) y `set_logo(&Transaction, Option<LogoMeta>)`.
- [ ] T027 [P] [US1] Implementar `src-tauri/src/storage/logo_store.rs`:
  - `write_atomic(data_dir, bytes, ext) -> rel_path`: escribe `branding/.tmp-<rand>`, hace `fsync` y lo renombra a `branding/logo-<sha8>.<png|jpg>`.
  - `delete(data_dir, rel_path)` (ignora "no existe").
  - `read_data_url(data_dir, rel_path, mime) -> String` (base64).
- [ ] T028 [US1] Implementar `src-tauri/src/services/logo_service.rs` y `src-tauri/src/services/mod.rs`:
  - `validate_logo_file(path) -> Result<ValidatedLogo, AppError>`, en este orden: tamaño con `fs::metadata` (≤ 5 242 880), lectura, `image::guess_format` (solo PNG o JPEG), decodificación completa, dimensiones y `low_resolution` si el lado corto mide menos de 200, SHA-256.
  - `set_logo(state, path)`: valida; escribe con `logo_store`; en una transacción crea el borrador si no hay fila y llama a `profile_repo::set_logo`; tras el *commit* borra el archivo anterior. Si falla la transacción, borra el archivo nuevo.
  - `logo_view(...) -> LogoView`.
- [ ] T029 [US1] Implementar `src-tauri/src/services/profile_service.rs`:
  - `get_profile(state) -> Option<BusinessProfileView>`, que construye la vista con `missing_fields`, `status`, `vatRatePercent` formateado y el logo como `dataUrl`.
  - `save_profile(state, input) -> Result<BusinessProfileView, AppError>`, que normaliza (trim, tax_id), rechaza `hard_errors` y hace `upsert` en una transacción.
  - `validate_profile(input) -> Vec<FieldError>`, que devuelve `hard_errors` + `missing_fields` sin guardar.
- [ ] T030 [US1] Implementar `src-tauri/src/commands/{mod.rs,profile.rs,logo.rs}`:
  - `#[tauri::command] profile_get`, `profile_save` y `profile_validate`.
  - `logo_pick_and_set` (`async`): usa `app.dialog().file().add_filter("Imagen", &["png","jpg","jpeg"]).blocking_pick_file()` dentro de `tauri::async_runtime::spawn_blocking`; devuelve `None` si se cancela; si no, llama a `logo_service::set_logo` y devuelve `{ logo, warnings }`.

  Registrarlos en `invoke_handler` de `src-tauri/src/lib.rs`.
- [ ] T031 [P] [US1] Implementar `src/features/business-profile/useBusinessProfile.ts`: carga inicial (`profileGet`), `save(input)`, `validate(input)` con *debounce* de 300 ms, `pickLogo()`, y estado `profile`, `fieldErrors`, `saving` y `loading`. Los errores `AppError` se traducen a errores por campo o a un mensaje general.
- [ ] T032 [P] [US1] Implementar `src/features/business-profile/LogoPicker.tsx` (+ `.module.css`): vista previa del logo (`dataUrl`, `object-fit: contain`, fondo a cuadros para ver la transparencia), botón "Elegir logo", aviso de baja resolución con `Banner` `warning` y mensajes de rechazo con `role="alert"`.
- [ ] T033 [US1] Implementar `src/features/business-profile/BusinessProfilePage.tsx` (+ `.module.css`) con React Hook Form. Secciones:
  - Datos del negocio: nombre comercial*, razón social, NIF/CIF*, teléfono*, email, web.
  - Dirección: calle y número*, código postal*, localidad*, provincia*.
  - IVA general (%), con 21 por defecto.
  - Logo (`LogoPicker`).
  - Tarjeta de vista previa con los datos y el logo tal como aparecerán.

  Los errores combinan los del backend y los "obligatorio" del cliente. Al guardar se muestra un toast. Con `?guiado=1`, se ve una cabecera de bienvenida con pasos y el botón "Saltar por ahora" (FR-001c). Añadir la ruta en `src/app/routes.tsx`.
- [ ] T034 [US1] Crear `src/app/IncompleteProfileBanner.tsx` e integrarlo en `src/app/App.tsx`. Al arrancar se llama a `profileGet`. Si devuelve `null` y no se ha saltado la configuración en esta sesión, se navega a `/negocio?guiado=1`. Si el perfil no existe o está `incomplete`, se muestra en todas las pantallas un `Banner` `warning` "Tu perfil está incompleto: faltan N datos" con el botón "Completar ahora" (FR-001c, FR-008). El banner se actualiza después de cada guardado.

**Checkpoint**: US1 funciona por sí sola. Hay que validarla con los escenarios 1–7 y 10 de quickstart.md con la red desactivada.

---

## Phase 4: User Story 2 - Gestionar la lista de tarifas (Priority: P1)

**Goal**: crear, editar y borrar tarifas (concepto, unidad, precio sin IVA), con validación de precio, control de duplicados, unidades personalizadas que se recuerdan y estado vacío en el primer uso.

**Independent Test**: escenarios 11–18 y 21 de [quickstart.md](quickstart.md). Una tarifa nueva aparece en la lista, al editarla muestra el precio nuevo y al borrarla desaparece; todo se conserva al reabrir.

### Tests for User Story 2 ⚠️

- [ ] T035 [P] [US2] Crear `src-tauri/tests/rate_service.rs` con una base de datos en `tempdir`:
  - Crear una tarifa válida devuelve `priceDisplay` "45,50 €".
  - Cada código `PRICE_*` para "", "abc", "12,345", "0" y "-3".
  - `CONCEPT_REQUIRED` y `UNIT_REQUIRED`.
  - "instalacion  de GRIFO" con la misma unidad que "Instalación de grifo" → `DuplicateRate` con el `existing_rate_id` correcto.
  - "Pintura" por hora y por m² se permiten ambas.
  - La unidad nueva "punto de luz" se crea y aparece en `units_list` al final, en orden alfabético.
  - Una actualización de precio se refleja; actualizar al concepto de otra tarifa → `DuplicateRate`; actualizar sin cambios en sí misma no da duplicado.
  - Borrar elimina la tarifa; actualizar o borrar un `id` inexistente → `NotFound`.
  - La lista sale ordenada por `concept_norm` ("Ábaco" antes que "Brida").
  - Los datos persisten al reabrir la conexión.
- [ ] T036 [P] [US2] Crear `src/features/rates/RatesPage.test.tsx` con `mockIPC`:
  - Una lista vacía muestra `EmptyState` y "Crear primera tarifa".
  - Crear una tarifa la añade a la lista.
  - Editar el precio muestra el nuevo `priceDisplay`.
  - Borrar y cancelar la mantiene; borrar y confirmar la quita.
  - Un error `duplicateRate` muestra el mensaje y el botón "Editar la existente", que abre esa tarifa.
  - Se ve el aviso "Precios sin IVA".

### Implementation for User Story 2

- [ ] T037 [P] [US2] Implementar `src-tauri/src/domain/rate.rs`:
  - *Structs* `RateInput { concept, unit_label, price }` (serde camelCase), `ValidRate { concept, concept_norm, unit_label, unit_norm, price_cents }` y `RateView { id, concept, unit: UnitView, price_cents, price_display }`.
  - `validate(input) -> Result<ValidRate, Vec<FieldError>>`: concept con trim y espacios colapsados, de 1 a 160 caracteres; unit_label con trim, de 1 a 30 caracteres; precio con `money::parse_price_cents`.
- [ ] T038 [P] [US2] Implementar `src-tauri/src/storage/unit_repo.rs`: `list(&Connection) -> Vec<UnitView>` (predefinidas por `id` y después personalizadas por `label_norm`), `find_by_norm(&Connection, &str) -> Option<i64>` e `insert_custom(&Transaction, label, label_norm) -> i64`.
- [ ] T039 [US2] Implementar `src-tauri/src/storage/rate_repo.rs`: `list_all(&Connection) -> Vec<RateRow>` (JOIN `units`, `ORDER BY concept_norm, concept`), `get(&Connection, id)`, `find_duplicate(&Connection, concept_norm, unit_id, exclude_id: Option<i64>) -> Option<i64>`, `insert`, `update` y `delete` (devuelve las filas afectadas).
- [ ] T040 [US2] Implementar `src-tauri/src/services/rate_service.rs`: `list(state, query: Option<String>)` (por ahora ignora `query` y llama a `list_all`), `create(state, input)` y `update(state, id, input)`. Ambos, dentro de una transacción: validan, resuelven o crean la unidad, comprueban duplicados (excluyendo el propio `id` al actualizar) y escriben. Una violación de `UNIQUE` de SQLite también se traduce a `DuplicateRate`. `delete(state, id)` devuelve `NotFound` si no hay filas afectadas. Los `RateView` se construyen con `money::format_eur`.
- [ ] T041 [US2] Implementar `src-tauri/src/commands/units.rs` (`units_list`) y `src-tauri/src/commands/rates.rs` (`rates_list(query: Option<String>)`, `rates_create(input)`, `rates_update(id, input)` y `rates_delete(id)`) y registrarlos en `src-tauri/src/lib.rs`.
- [ ] T042 [P] [US2] Implementar `src/features/rates/useRates.ts`: `rates`, `units`, `reload()`, `create`, `update` y `remove`. Tras cada mutación recarga las tarifas y las unidades. Expone `lastError` como `AppError`.
- [ ] T043 [P] [US2] Implementar `src/features/rates/UnitSelect.tsx`: combobox accesible (`input` + `datalist`, o un listbox propio) con las unidades de `units_list` y la posibilidad de escribir una nueva. Muestra el texto de ayuda "Puedes escribir otra unidad".
- [ ] T044 [US2] Implementar `src/features/rates/RateFormDialog.tsx` (+ `.module.css`): modo crear o editar, campos concepto, unidad (`UnitSelect`) y precio (texto con `inputMode="decimal"` y sufijo "€ sin IVA"), y errores por campo desde `validation`. Ante `duplicateRate`, muestra el mensaje y el botón "Editar la existente" (`onEditExisting(id)`). Admite la *prop* `initialConcept` para precargar.
- [ ] T045 [US2] Implementar `src/features/rates/RatesPage.tsx` (+ `.module.css`): `Banner` `info` "Los precios se guardan sin IVA"; lista de filas con concepto, unidad y precio, cada una con los botones "Editar" y "Borrar"; botón "Nueva tarifa"; `EmptyState` "Aún no tienes tarifas" con "Crear primera tarifa"; borrado con `ConfirmDialog` ("¿Borrar la tarifa «X»?"); toasts de confirmación. Añadir la ruta `/tarifas` en `src/app/routes.tsx`.

**Checkpoint**: US1 y US2 funcionan cada una por separado.

---

## Phase 5: User Story 3 - Modificar o sustituir los datos y el logo (Priority: P2)

**Goal**: el profesional cambia sus datos, sustituye o quita el logo en cualquier momento y recibe un aviso si sale con cambios sin guardar.

**Independent Test**: escenarios 8 y 9 de [quickstart.md](quickstart.md). Con datos y logo guardados, cambiar un dato y el logo: tras reabrir aparecen los nuevos y no queda rastro del logo anterior en `branding/`.

**Depende de**: US1 (pantalla y servicios del perfil).

### Tests for User Story 3 ⚠️

- [ ] T046 [P] [US3] Crear `src-tauri/tests/logo_replace.rs`:
  - Sustituir el logo A por B deja en `branding/` solo el archivo de B.
  - `remove_logo` pone las columnas del logo a `NULL` y borra el archivo.
  - Intentar sustituir por un archivo inválido mantiene el archivo y la fila de A.
  - `cleanup_orphans` borra los archivos `logo-*` y `.tmp-*` no referenciados y conserva el vigente.
  - Una edición de datos ya guardados persiste al reabrir.
- [ ] T047 [P] [US3] Crear `src/shared/components/UnsavedChangesGuard.test.tsx` y ampliar `src/features/business-profile/BusinessProfilePage.test.tsx`:
  - Con el formulario modificado, navegar a `/tarifas` muestra el diálogo.
  - "Descartar" navega sin guardar.
  - "Guardar" llama a `profile_save` y después navega.
  - "Cancelar" se queda en la página.
  - "Quitar logo" pide confirmación y llama a `logo_remove`.

### Implementation for User Story 3

- [ ] T048 [US3] Ampliar `src-tauri/src/services/logo_service.rs` con `remove_logo(state)` (en una transacción pone el logo a `None` y, tras el *commit*, borra el archivo) y `cleanup_orphans(data_dir, conn)` (borra de `branding/` lo que no sea el `logo_file` vigente). Llamar a `cleanup_orphans` en el `setup` de `src-tauri/src/lib.rs`.
- [ ] T049 [US3] Añadir el comando `logo_remove` en `src-tauri/src/commands/logo.rs` (devuelve `BusinessProfileView`) y registrarlo en `src-tauri/src/lib.rs`.
- [ ] T050 [P] [US3] Implementar `src/shared/components/UnsavedChangesGuard.tsx`: recibe `when: boolean` y `onSave: () => Promise<boolean>`. Usa `useBlocker` de React Router para mostrar `ConfirmDialog` con "Guardar", "Descartar" y "Cancelar". Añade además un `beforeunload` para el cierre de la ventana.
- [ ] T051 [US3] Ampliar `src/features/business-profile/BusinessProfilePage.tsx` y `LogoPicker.tsx`: con logo existente, el botón pasa a "Cambiar logo" y aparece "Quitar logo" (con `ConfirmDialog`), que llama a `logoRemove` mediante `useBusinessProfile`. Envolver la página con `UnsavedChangesGuard` (`when = formState.isDirty`). Tras guardar, hacer `reset` con los valores guardados.

**Checkpoint**: US1, US2 y US3 funcionan cada una por separado.

---

## Phase 6: User Story 4 - Buscar una tarifa por nombre (Priority: P2)

**Goal**: filtrar la lista de tarifas escribiendo parte del concepto, sin distinguir mayúsculas ni tildes, con un estado propio para "sin resultados".

**Independent Test**: escenarios 19 y 20 de [quickstart.md](quickstart.md) y la comprobación de rendimiento con 500 tarifas.

**Depende de**: US2 (tarifas, repositorio y pantalla).

### Tests for User Story 4 ⚠️

- [ ] T052 [P] [US4] Crear `src-tauri/tests/rate_search.rs`:
  - Con "Instalación de grifo", "Cambio de grifo" y "Pintura de pared": `grifo` devuelve 2, `GRIFO` devuelve 2, `instalacion` devuelve 1 y `zzz` devuelve 0.
  - Una consulta vacía o de solo espacios devuelve todas.
  - `%` y `_` se tratan como literales.
  - `rates_search_500`: inserta 500 tarifas y comprueba que `list(Some("tubo"))` tarda menos de 50 ms con `--release` (con `cfg!(debug_assertions)` el umbral sube a 250 ms).
- [ ] T053 [P] [US4] Crear `src/features/rates/RatesSearch.test.tsx` con `mockIPC`:
  - Escribir llama a `rates_list` con `query` y muestra solo los resultados.
  - Sin resultados se ve "No hay tarifas que coincidan con «zzz»" y el botón "Crear tarifa «zzz»", que abre `RateFormDialog` con el concepto precargado.
  - Vaciar el buscador restaura la lista completa.
  - Una lista vacía sin consulta sigue mostrando el estado de primer uso, no el de "sin resultados".

### Implementation for User Story 4

- [ ] T054 [US4] Ampliar `src-tauri/src/storage/rate_repo.rs` con `search(&Connection, norm_query) -> Vec<RateRow>` (`WHERE r.concept_norm LIKE '%' || ?1 || '%' ESCAPE '\'`, mismo `ORDER BY`). En `src-tauri/src/services/rate_service.rs::list`, si `query` tiene texto tras `trim`, usar `search(escape_like(normalize(q)))`; si no, `list_all`.
- [ ] T055 [US4] Implementar `src/features/rates/RatesSearch.tsx` (input `type="search"` con etiqueta "Buscar tarifa", *debounce* de 150 ms y botón para limpiar) e integrarlo en `src/features/rates/RatesPage.tsx` y `useRates.ts` (parámetro `query`). Distinguir los estados vacío (sin consulta) y sin resultados (con consulta), y conectar "Crear tarifa «X»" con `RateFormDialog initialConcept`.

**Checkpoint**: las cuatro historias funcionan cada una por separado.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T056 [P] Crear `src-tauri/examples/seed_rates.rs`: abre la base de datos en el `app_data_dir` real (identificador de T001) e inserta N tarifas sintéticas ("Tubo cobre 15 mm 1"…) con unidades variadas. Conectar el script `seed:rates` de `package.json` a `cargo run --example seed_rates --manifest-path src-tauri/Cargo.toml --`.
- [ ] T057 [P] Configurar e2e en `e2e/wdio.conf.ts` (WebdriverIO + `tauri-driver` con el binario de `tauri build --debug`, carpeta de datos temporal mediante una variable de entorno que `lib.rs` respete solo en builds debug). Crear `e2e/specs/business-profile.e2e.ts` (escenarios 1–3 y 10) y `e2e/specs/rates.e2e.ts` (escenarios 11, 12, 17, 18 y 19).
- [ ] T058 [P] Revisar la accesibilidad en `src/shared/components/` y en las páginas: todos los inputs con `label`, errores con `aria-live`, orden de tabulación lógico, `Esc` cierra los diálogos, foco devuelto al disparador y contraste AA comprobado.
- [ ] T059 [P] Crear `README.md` en la raíz: qué es la app, requisitos (Node 20+, Rust estable, VS Build Tools, WebView2), arranque (`npm install`, `npm run tauri dev`), pruebas y dónde se guardan los datos.
- [ ] T060 Comprobar que no hay red (Principios I y II): revisar que `src-tauri/Cargo.toml` y `package.json` no incluyen clientes HTTP ni telemetría, que la CSP y `capabilities/default.json` coinciden con T007, y lanzar la app con la red desactivada.
- [ ] T061 Pasar todas las comprobaciones de calidad y corregir lo que falle: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` y `cargo test --release --test rate_search`, `npm run lint`, `npm run typecheck`, `npm run test` y `npm run test:e2e`.
- [ ] T062 Ejecutar los escenarios manuales 1–21 y la comprobación de robustez de [quickstart.md](quickstart.md) con la red desactivada. Registrar el resultado (OK o fallo, con nota) en `specs/001-business-profile-rates/checklists/quickstart-results.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sin dependencias.
- **Foundational (Phase 2)**: depende de Setup y bloquea todas las historias.
- **US1 (Phase 3)** y **US2 (Phase 4)**: dependen solo de Foundational y son independientes entre sí.
- **US3 (Phase 5)**: depende de US1.
- **US4 (Phase 6)**: depende de US2.
- **Polish (Phase 7)**: depende de las historias que se quieran entregar.

```text
Setup ─▶ Foundational ─┬─▶ US1 (P1) ─▶ US3 (P2) ─┐
                       └─▶ US2 (P1) ─▶ US4 (P2) ─┴─▶ Polish
```

### Within Each User Story

- Las pruebas se escriben primero y fallan; después se implementa.
- Orden: dominio → repositorio → servicio → comando (y registro en `lib.rs`) → hook React → componentes → página.
- Archivos compartidos que obligan a trabajar en secuencia: `src-tauri/src/lib.rs` (T013, T030, T041, T048, T049), `src/app/routes.tsx` (T019, T033, T045), `RatesPage.tsx` (T045 → T055) y `BusinessProfilePage.tsx` (T033 → T051).

### Parallel Opportunities

- **Setup**: T003–T007 en paralelo tras T001 y T002.
- **Foundational**: T010, T011, T012, T014, T015, T016, T017 y T018 en paralelo (T014 necesita T008 y T009).
- **US1 ∥ US2**: tras Foundational, dos personas pueden trabajar a la vez en US1 y US2. Solo chocan al registrar comandos en `lib.rs`.
- **Dentro de cada historia**: todas las pruebas [P] a la vez; en US1, T024 ∥ T027 y T031 ∥ T032; en US2, T037 ∥ T038 y T042 ∥ T043.

---

## Parallel Example: User Story 1

```text
# Pruebas de US1 en paralelo:
Task: "T020 Pruebas de validadores en src-tauri/tests/validation.rs"
Task: "T021 Pruebas de profile_service en src-tauri/tests/profile_service.rs"
Task: "T022 Pruebas de logo_service en src-tauri/tests/logo_service.rs"
Task: "T023 Pruebas de BusinessProfilePage en src/features/business-profile/BusinessProfilePage.test.tsx"

# Implementación independiente en paralelo:
Task: "T024 Validadores en src-tauri/src/domain/validation/"
Task: "T027 logo_store en src-tauri/src/storage/logo_store.rs"
```

## Parallel Example: User Story 2

```text
Task: "T035 Pruebas de rate_service en src-tauri/tests/rate_service.rs"
Task: "T036 Pruebas de RatesPage en src/features/rates/RatesPage.test.tsx"
Task: "T037 Dominio Rate en src-tauri/src/domain/rate.rs"
Task: "T038 unit_repo en src-tauri/src/storage/unit_repo.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1 (Setup) y Phase 2 (Foundational).
2. Phase 3 (US1): el profesional puede configurar su negocio y su logo.
3. **Parar y validar**: escenarios 1–7 y 10 del quickstart sin red.

### Incremental Delivery

1. Setup + Foundational → base lista.
2. US1 → validar → demo (MVP: identidad del negocio).
3. US2 → validar → demo (tarifas: la app ya tiene precios propios; con esto queda cubierto todo lo P1).
4. US3 → validar (mantenimiento de datos y logo).
5. US4 → validar (búsqueda y rendimiento con 500 tarifas).
6. Polish → e2e, accesibilidad, quickstart completo.

### Parallel Team Strategy

Con dos personas, tras Foundational: A hace US1 → US3 y B hace US2 → US4. Se coordinan solo al editar `src-tauri/src/lib.rs`.

---

## Notes

- [P] = otro archivo y sin dependencias pendientes.
- Cada historia se puede completar y probar por separado (respetando US1 → US3 y US2 → US4).
- Hacer un commit tras cada tarea o grupo lógico.
- Ninguna tarea puede introducir acceso a red ni servicios remotos (constitución, Principios I y II).
