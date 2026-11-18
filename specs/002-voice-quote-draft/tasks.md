---

description: "Lista de tareas para implementar el borrador de presupuesto por voz"
---

# Tasks: Borrador de presupuesto por voz

**Input**: Documentos de diseño en `/specs/002-voice-quote-draft/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/ipc-commands.md](contracts/ipc-commands.md), [quickstart.md](quickstart.md)

**Tests**: se incluyen porque CLAUDE.md exige TDD. En cada historia, las pruebas se escriben primero y deben fallar antes de implementar.

**Organization**: tareas agrupadas por historia de usuario, para implementar y probar cada una por separado.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: se puede ejecutar en paralelo (otro archivo, sin dependencias pendientes).
- **[Story]**: historia a la que pertenece (US1–US5, según spec.md).
- Rutas relativas a la raíz del repositorio: `src/` es la interfaz React y `src-tauri/` el núcleo Rust.
- Las pruebas de integración usan el crate `app_lib`, así que los módulos nuevos `audio` y `transcription` deben ser `pub`.
- **Registro de comandos**: todos se registran en `src-tauri/src/lib.rs`, así que las tareas que lo tocan no son paralelas entre sí.
- **Contrato IPC**: cualquier cambio se hace primero en [contracts/ipc-commands.md](contracts/ipc-commands.md), después en `src/shared/ipc/types.ts` y por último en Rust.
- **Compilación**: usar siempre `$env:CARGO_BUILD_JOBS = 2`. Con más procesos falla por falta de memoria.
- Marcar cada tarea completada con `- [X]`.

---

## Phase 1: Setup (infraestructura compartida)

**Purpose**: preparar el entorno, las dependencias y el modelo, y **medir la velocidad del modelo antes de construir nada más**.

- [ ] T001 Comprobar que `cmake --version` y `clang --version` funcionan y que `LIBCLANG_PATH` está definido, siguiendo los prerrequisitos de `specs/002-voice-quote-draft/quickstart.md`. Si falta algo, detenerse y pedir al responsable que lo instale.
- [ ] T002 Añadir a `src-tauri/Cargo.toml` las dependencias `cpal` 0.18, `symphonia` 0.6 (features `isomp4`, `aac`, `alac`, `mp3`, `wav`, `pcm`), `rubato` 5 y `whisper-rs` 0.16 como dependencia opcional, con `[features] default = ["local-transcription"]` y `local-transcription = ["dep:whisper-rs"]`.
- [ ] T003 [P] Crear `scripts/fetch-model.mjs`, que descargue `ggml-small-q5_1.bin` y `ggml-base-q5_1.bin` del repositorio público de whisper.cpp a `src-tauri/resources/models/` y verifique los SHA-256 de research.md (R2); añadir el script `model:fetch` a `package.json`.
- [ ] T004 [P] Añadir `src-tauri/resources/models/*.bin` a `.gitignore`.
- [ ] T005 Crear `src-tauri/examples/transcribe_benchmark.rs` (con `required-features = ["local-transcription"]` en `src-tauri/Cargo.toml`), que transcriba un WAV de 16 kHz con el modelo indicado e imprima el tiempo, el pico de memoria y el texto.
- [ ] T006 **Puerta de decisión (SC-002)**: ejecutar `transcribe_benchmark` con un dictado real de 1 minuto y anotar los resultados en `specs/002-voice-quote-draft/research.md` (R2). Si `small-q5_1` supera los 60 s en un equipo de 4 núcleos y 8 GB, cambiar a `base-q5_1` y actualizar plan.md y research.md antes de continuar.
- [ ] T007 Incluir el modelo elegido en `bundle.resources` de `src-tauri/tauri.conf.json`, sin modificar la CSP.

**Checkpoint**: el modelo está elegido con datos medidos y `cargo build -j 2` compila.

---

## Phase 2: Foundational (requisitos que bloquean todo)

**Purpose**: errores, dinero, esquema de datos, repositorios, cliente IPC y armazón de la pantalla.

**⚠️ CRITICAL**: ninguna historia puede empezar hasta terminar esta fase.

### Pruebas (escribir primero)

- [ ] T008 [P] Ampliar `src-tauri/tests/db_migrations.rs`: la migración 2 se aplica; una segunda fila en `quote_draft` falla; una línea `price_source = 'rate'` sin precio falla; una línea `pending` con precio falla; una línea con precio y sin unidad falla; borrar una tarifa deja `rate_id` en `NULL` y conserva el precio; ninguna tabla nueva tiene columnas de datos del cliente final (FR-041).
- [ ] T009 [P] Añadir a `src-tauri/src/domain/money.rs` pruebas de la tabla de cantidades de data-model.md, y de importe e IVA: 300 × 2500 → 7500; 150 × 3333 → 5000 (redondeo al céntimo); base 13500 al 21 % → 2835; base 16000 al 21 % → 3360; desbordamiento → `AMOUNT_TOO_LARGE`.
- [ ] T010 [P] Crear `src-tauri/tests/draft_totals.rs`: base, IVA y total de los ejemplos de la spec (135,00 / 28,35 / 163,35 y 160,00 / 33,60 / 193,60); las líneas pendientes quedan fuera y `is_provisional` es verdadero (FR-029, FR-031).

### Implementación

- [ ] T011 Ampliar `src-tauri/src/error.rs` con `Microphone`, `AudioRejected`, `Transcription`, `ConfirmationRequired`, `InvalidState` y `DraftHasPending`, y con los códigos `QUANTITY_*`, `AMOUNT_TOO_LARGE` y `TEXT_REQUIRED`, con mensajes en español sencillo y una prueba de serialización según el contrato.
- [ ] T012 Implementar en `src-tauri/src/domain/money.rs` `parse_quantity_centi`, `format_quantity`, `line_amount_cents` y `vat_amount_cents` con cálculo intermedio en `i128`, según research.md (R9).
- [ ] T013 Crear `src-tauri/src/storage/migrations/002_voice_draft.sql` con las tablas y los `CHECK` de data-model.md y registrarla en `src-tauri/src/storage/db.rs`.
- [ ] T014 Crear `src-tauri/src/domain/draft/mod.rs` con los tipos `DraftLine`, `PriceSource`, `PendingReason` y `QuantityIssue`, y `src-tauri/src/domain/draft/totals.rs`; declarar `draft` en `src-tauri/src/domain/mod.rs`.
- [ ] T015 Crear `src-tauri/src/storage/draft_repo.rs`: leer el borrador completo, sustituirlo en una transacción, y operaciones sobre líneas, sugerencias y fragmentos sin interpretar; declararlo en `src-tauri/src/storage/mod.rs`.
- [ ] T016 Crear `src-tauri/src/storage/dictation_repo.rs` (ranuras `current` e `incoming` y cambios de estado) y `src-tauri/src/storage/recording_store.rs` (rutas en `recordings/`, promoción atómica y limpieza de huérfanos); declararlos en `src-tauri/src/storage/mod.rs`.
- [ ] T017 Crear `src-tauri/src/transcription/mod.rs` con el trait `Transcriber`, `TranscribeOptions`, `Transcript` y un `UnavailableTranscriber` para compilar sin la feature `local-transcription`; declarar `pub mod transcription` y `pub mod audio` en `src-tauri/src/lib.rs` y crear `src-tauri/src/audio/mod.rs`.
- [ ] T018 Ampliar `src-tauri/src/state.rs` con la grabación activa, la señal de cancelación y el `Transcriber`, y en `src-tauri/src/lib.rs` limpiar los audios huérfanos y devolver a `recorded` los dictados en `processing` al arrancar.
- [ ] T019 Añadir a `src/shared/ipc/types.ts` los tipos del contrato y a `src/shared/ipc/client.ts` una función tipada por comando, usando `Channel` de `@tauri-apps/api/core`.
- [ ] T020 [P] Añadir a `src/shared/i18n/es.ts` todos los textos de la pantalla "Presupuesto": dictado, importación, progreso, errores, pendientes, totales y confirmaciones (FR-040).
- [ ] T021 Crear `src/features/voice-draft/DraftPage.tsx` como armazón, añadir la ruta `presupuesto` en `src/app/routes.tsx` y el enlace "Presupuesto" en la navegación de `src/app/App.tsx`.

**Checkpoint**: `cargo test -j 2 --no-default-features` y `npm run test` pasan, y la app muestra la pantalla vacía.

---

## Phase 3: User Story 1 - Dictar la visita y obtener un borrador (Priority: P1) 🎯 MVP

**Goal**: dictar o importar un audio y obtener un borrador con líneas, importes y total calculados con las tarifas.

**Independent Test**: con las tarifas "Bajante" (25,00 €/metro) y "Mano de obra" (30,00 €/hora), dictar "cambiar bajante, 3 metros, mano de obra 2 horas" da 75,00 € + 60,00 €, base 135,00 €, IVA 28,35 € y total 163,35 €.

### Pruebas de User Story 1 (escribir primero y comprobar que fallan) ⚠️

- [ ] T022 [P] [US1] Crear `src-tauri/tests/draft_numbers.rs`: cifras, decimales con coma, palabras hasta miles, "metro y medio" → 1,5, "dos horas y media" → 2,5, "media hora" → 0,5, "cuarto de hora" → 0,25 y "un par de" → 2 (FR-016).
- [ ] T023 [P] [US1] Crear `src-tauri/tests/draft_parser.rs` con la tabla de patrones de research.md (R6): el ejemplo de la spec da dos trabajos; varios trabajos mezclados conservan el orden y un concepto repetido da dos líneas; "unos cuantos metros" es ambiguo; "3 metros, no, mejor 4" es rectificación; lo no interpretable queda como fragmento (FR-015, FR-020, FR-022).
- [ ] T024 [P] [US1] Crear `src-tauri/tests/draft_matcher.rs` con cada fila de research.md (R7): coincidencia con singular y plural, varias tarifas, desempate por unidad, unidad distinta sin convertir, parecidos sin precio, hasta 5 sugerencias en orden estable, sin tarifas y el caso "Mano de obra" / "Mano de obra urgente" (FR-017 a FR-021, FR-023).
- [ ] T025 [P] [US1] Crear `src-tauri/tests/draft_builder.rs`: el borrador del ejemplo coincide con la spec; 100 repeticiones dan un resultado idéntico (FR-014); ninguna línea tiene un precio distinto al de su tarifa (SC-004); con 500 tarifas tarda menos de 100 ms.
- [ ] T026 [P] [US1] Crear `src-tauri/tests/audio_pipeline.rs`: la reparación de un `.wav.part` truncado conserva las muestras; el silencio se detecta; WAV, MP3 y M4A de `src-tauri/tests/fixtures/audio/` se decodifican a 16 kHz mono; un texto renombrado a `.mp3` da `AUDIO_UNSUPPORTED_FORMAT`; un archivo truncado da `AUDIO_CORRUPT`; más de 10 minutos da `AUDIO_TOO_LONG`.
- [ ] T027 [P] [US1] Crear `src-tauri/tests/dictation_service.rs` con un transcriptor simulado, que cubra los caminos de fallo del Principio III: éxito; sin voz; modelo no disponible; fallo y reintento; cancelación; y que en todos los fallos se conservan el audio y el borrador anterior (FR-004, FR-006, FR-009, SC-008). Incluir la recuperación de una grabación interrumpida.
- [ ] T028 [P] [US1] Crear `src/features/voice-draft/DictationPanel.test.tsx` con `mockIPC`: grabar y parar con el botón y con F9, tiempo visible, aviso del límite, progreso y cancelación, errores de micrófono con las alternativas y confirmación antes de sustituir un borrador.
- [ ] T029 [P] [US1] Crear `src/features/voice-draft/DraftLinesTable.test.tsx`: líneas completas, "precio pendiente", "cantidad pendiente" con su fragmento, sugerencias, fragmentos sin interpretar, total provisional y acceso a crear tarifas.

### Implementación de User Story 1

- [ ] T030 [P] [US1] Implementar `src-tauri/src/domain/draft/numbers.rs` según research.md (R6).
- [ ] T031 [P] [US1] Implementar `src-tauri/src/domain/draft/units.rs` con los alias de research.md (R8) y las unidades personalizadas.
- [ ] T032 [US1] Implementar `src-tauri/src/domain/draft/parser.rs` según research.md (R6) (depende de T030 y T031).
- [ ] T033 [US1] Implementar `src-tauri/src/domain/draft/matcher.rs` según research.md (R7), incluida la distancia de edición propia para los parecidos.
- [ ] T034 [US1] Implementar `src-tauri/src/domain/draft/builder.rs` como función pura `build_draft(text, rates, units, vat_rate_bp)`.
- [ ] T035 [P] [US1] Crear las muestras cortas `tone-3s.wav`, `tone-3s.mp3` y `tone-3s.m4a` en `src-tauri/tests/fixtures/audio/`.
- [ ] T036 [P] [US1] Implementar `src-tauri/src/audio/wav.rs`: escritura incremental con volcado cada segundo y reparación de la cabecera (research.md, R5).
- [ ] T037 [P] [US1] Implementar `src-tauri/src/audio/resample.rs` con `rubato`, a 16 kHz y mono.
- [ ] T038 [P] [US1] Implementar `src-tauri/src/audio/quality.rs`: detección de silencio y criterio de calidad baja (research.md, R10).
- [ ] T039 [US1] Implementar `src-tauri/src/audio/decode.rs` con `symphonia`, con decodificación en flujo y corte a los 10 minutos (FR-002b, FR-002c).
- [ ] T040 [US1] Implementar `src-tauri/src/audio/capture.rs`: hilo propio con `cpal`, eventos de nivel y tiempo, aviso a los 9 minutos, parada a los 10, desconexión y errores `MIC_*` (FR-001, FR-005, FR-008; research.md, R3).
- [ ] T041 [US1] Implementar `src-tauri/src/transcription/whisper.rs`: carga bajo demanda, idioma `es`, progreso, cancelación, vocabulario de tarifas y descarte de segmentos sin voz (research.md, R1, R10 y R11).
- [ ] T042 [US1] Implementar `src-tauri/src/services/dictation_service.rs`: iniciar, parar, importar, procesar, reintentar, cancelar, recuperar y descartar, sin mantener bloqueada la conexión SQLite durante la transcripción (FR-004, FR-009, FR-010, FR-035).
- [ ] T043 [US1] Implementar `src-tauri/src/services/draft_service.rs` con la generación del borrador y su vista completa, incluidos totales y `vatChange`, y declarar ambos servicios en `src-tauri/src/services/mod.rs`.
- [ ] T044 [US1] Crear `src-tauri/src/commands/session.rs` (`session_get`) y `src-tauri/src/commands/dictation.rs` con los comandos de dictado del contrato; declararlos en `src-tauri/src/commands/mod.rs` y registrarlos en `src-tauri/src/lib.rs`.
- [ ] T045 [US1] Crear `src/features/voice-draft/useDraftSession.ts` con la carga de la sesión, los canales de eventos y las acciones de dictado.
- [ ] T046 [US1] Crear `src/features/voice-draft/DictationPanel.tsx`: botón de al menos 96 px, tecla F9, importar, tiempo, nivel, progreso, cancelar, reintentar, recuperar y mensajes de error (FR-001, FR-002, FR-006 a FR-009, R12).
- [ ] T047 [P] [US1] Crear `src/features/voice-draft/DraftLineRow.tsx` en modo lectura, con las marcas de pendiente, el fragmento dictado y las sugerencias.
- [ ] T048 [US1] Crear `src/features/voice-draft/DraftLinesTable.tsx`, con los fragmentos sin interpretar y el acceso a crear tarifas (FR-022, FR-023).
- [ ] T049 [P] [US1] Crear `src/features/voice-draft/DraftTotals.tsx` con base, IVA, total y aviso de total provisional (FR-029, FR-031).
- [ ] T050 [US1] Completar `src/features/voice-draft/DraftPage.tsx`, con el texto transcrito visible junto al borrador (FR-010).

**Checkpoint**: el escenario 1 de quickstart.md y los escenarios 7–10 y 12–16 funcionan.

---

## Phase 4: User Story 2 - Revisar y corregir las líneas del borrador (Priority: P1)

**Goal**: editar, añadir y borrar líneas con recálculo inmediato, y dar el borrador por bueno.

**Independent Test**: sobre un borrador generado, cambiar la cantidad de bajante a 4 da importe 100,00 €, base 160,00 €, IVA 33,60 € y total 193,60 €, y los cambios se conservan al reabrir la app.

### Pruebas de User Story 2 (escribir primero y comprobar que fallan) ⚠️

- [ ] T051 [P] [US2] Crear `src-tauri/tests/draft_editing.rs` con cada fila de "Reglas de edición" de data-model.md: cantidad, concepto, precio → manual, unidad → pendiente, asignar tarifa, añadir desde tarifa y a mano, borrar y validaciones (FR-024 a FR-027, FR-032).
- [ ] T052 [P] [US2] Crear `src-tauri/tests/draft_lifecycle.rs`: una tarifa modificada o borrada no cambia la línea (FR-033); dar por bueno falla con pendientes o sin líneas; editar un borrador revisado lo devuelve a borrador (FR-034); el cambio de IVA conserva el total hasta aceptarlo (FR-029b); el borrador se conserva al reabrir la base de datos (FR-035); descartar borra el borrador y su audio (FR-036).
- [ ] T053 [P] [US2] Crear `src/features/voice-draft/DraftEditing.test.tsx` con `mockIPC`: editar una cantidad actualiza los importes mostrados, errores junto al campo, añadir, borrar con confirmación, elegir una sugerencia, dar por bueno y aviso de IVA.

### Implementación de User Story 2

- [ ] T054 [US2] Añadir a `src-tauri/src/services/draft_service.rs` las operaciones de edición de líneas, según las reglas de data-model.md.
- [ ] T055 [US2] Añadir a `src-tauri/src/services/draft_service.rs` `mark_reviewed`, `apply_current_vat` y `discard`.
- [ ] T056 [US2] Crear `src-tauri/src/commands/draft.rs` con `draft_line_add`, `draft_line_update`, `draft_line_assign_rate`, `draft_line_delete`, `draft_apply_current_vat`, `draft_mark_reviewed` y `draft_discard`; declararlo en `src-tauri/src/commands/mod.rs` y registrarlos en `src-tauri/src/lib.rs`.
- [ ] T057 [US2] Añadir a `src/features/voice-draft/useDraftSession.ts` las acciones de edición, aplicando siempre la `DraftView` que devuelve el núcleo, sin calcular dinero en la interfaz.
- [ ] T058 [US2] Hacer editable `src/features/voice-draft/DraftLineRow.tsx`: concepto, cantidad, unidad y precio, con errores junto al campo, elección de sugerencia y borrado con `ConfirmDialog`.
- [ ] T059 [P] [US2] Crear `src/features/voice-draft/AddLineDialog.tsx`, para añadir una línea desde una tarifa o a mano (FR-025).
- [ ] T060 [US2] Añadir a `src/features/voice-draft/DraftPage.tsx` "Dar por bueno", el señalado de las líneas pendientes, el estado revisado, el aviso de IVA y "Nuevo presupuesto" con confirmación (FR-029b, FR-034, FR-036).

**Checkpoint**: los escenarios 2, 3, 11 y 17 de quickstart.md funcionan.

---

## Phase 5: User Story 3 - Dictar sin conexión a Internet (Priority: P1)

**Goal**: garantizar que todo el flujo funciona con la red desactivada y que la app informa de que la transcripción es local.

**Independent Test**: con la red desactivada, repetir el dictado de la Historia 1 da el mismo borrador y no aparece ningún aviso de red.

### Pruebas de User Story 3 (escribir primero y comprobar que fallan) ⚠️

- [ ] T061 [P] [US3] Crear `src-tauri/tests/offline_guarantees.rs`: la CSP de `tauri.conf.json` no tiene orígenes remotos; `capabilities/default.json` solo concede `core:default`, `dialog:allow-open` y `log:default`; `Cargo.toml` no declara clientes HTTP ni plugins de red; el modelo elegido figura en `bundle.resources` (FR-037, FR-038).
- [ ] T062 [P] [US3] Añadir a `src/features/voice-draft/DictationPanel.test.tsx` la comprobación del aviso no bloqueante de transcripción local (FR-039).

### Implementación de User Story 3

- [ ] T063 [US3] Resolver la ruta del modelo con `BaseDirectory::Resource` en `src-tauri/src/lib.rs` y comprobar su existencia antes de transcribir, devolviendo `MODEL_UNAVAILABLE` con un mensaje claro y sin perder el audio.
- [ ] T064 [US3] Mostrar en `src/features/voice-draft/DictationPanel.tsx` el aviso de `transcribedLocally` (FR-039).
- [ ] T065 [US3] Ejecutar con la red desactivada los escenarios 4 y 15 de `specs/002-voice-quote-draft/quickstart.md` en una versión instalada con `npm run tauri build`, y anotar el resultado en ese archivo.

**Checkpoint**: el flujo completo funciona sin red en la app instalada.

---

## Phase 6: User Story 4 - Ver y corregir el texto transcrito (Priority: P2)

**Goal**: corregir el texto y regenerar el borrador sin volver a dictar.

**Independent Test**: tras un dictado, cambiar "3 metros" por "5 metros" y regenerar deja la línea en 5 metros con importe 125,00 €.

### Pruebas de User Story 4 (escribir primero y comprobar que fallan) ⚠️

- [ ] T066 [P] [US4] Crear `src-tauri/tests/draft_regenerate.rs`: regenerar aplica el texto nuevo; con cambios manuales y sin confirmar devuelve `confirmationRequired` y no cambia nada; al confirmar, sustituye las líneas; el texto guardado sobrevive al reabrir; regenerar un borrador revisado lo devuelve a borrador; se usa el precio vigente de las tarifas (FR-011, FR-012, FR-035).
- [ ] T067 [P] [US4] Crear `src/features/voice-draft/VisitTextEditor.test.tsx` con `mockIPC`: editar guarda con *debounce*, regenerar, aviso de cambios manuales y aviso de calidad baja (FR-007).

### Implementación de User Story 4

- [ ] T068 [US4] Añadir a `src-tauri/src/services/draft_service.rs` `regenerate` y `save_text`.
- [ ] T069 [US4] Añadir `draft_generate_from_text` (modo `regenerate`) y `draft_text_save` a `src-tauri/src/commands/draft.rs` y registrarlos en `src-tauri/src/lib.rs`.
- [ ] T070 [US4] Crear `src/features/voice-draft/VisitTextEditor.tsx`, con guardado cada 500 ms, botón de regenerar y `ConfirmDialog`.
- [ ] T071 [US4] Sustituir en `src/features/voice-draft/DraftPage.tsx` el texto de solo lectura por `VisitTextEditor`.

**Checkpoint**: el escenario 5 de quickstart.md funciona.

---

## Phase 7: User Story 5 - Escribir la visita a mano (Priority: P2)

**Goal**: obtener el mismo borrador escribiendo o pegando el texto, sin usar el micrófono.

**Independent Test**: pegar "cambiar bajante, 3 metros, mano de obra 2 horas" da las mismas líneas y totales que la Historia 1.

### Pruebas de User Story 5 (escribir primero y comprobar que fallan) ⚠️

- [ ] T072 [P] [US5] Crear `src-tauri/tests/draft_from_text.rs`: el texto escrito y el transcrito dan el mismo borrador (FR-014, SC-007); el texto vacío da `TEXT_REQUIRED`; con un borrador existente exige confirmación; al confirmar borra el audio anterior.
- [ ] T073 [P] [US5] Añadir a `src/features/voice-draft/VisitTextEditor.test.tsx` los casos de escribir y generar sin dictado, y de texto vacío.
- [ ] T074 [P] [US5] Crear `e2e/specs/3-voice-draft.e2e.ts`: crear tarifas, pegar el texto del ejemplo, comprobar los totales, cambiar una cantidad, completar una línea pendiente y dar por bueno.

### Implementación de User Story 5

- [ ] T075 [US5] Añadir a `src-tauri/src/services/draft_service.rs` y a `src-tauri/src/commands/draft.rs` el modo `new` de `draft_generate_from_text`.
- [ ] T076 [US5] Añadir a `src/features/voice-draft/VisitTextEditor.tsx` el modo "Escribir la visita" con el botón de generar, y enlazarlo desde las alternativas de `src/features/voice-draft/DictationPanel.tsx` (FR-008, FR-013).

**Checkpoint**: el escenario 6 de quickstart.md y la prueba e2e pasan.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: calidad, precisión real y documentación.

- [ ] T077 Con al menos 20 dictados reales con ruido de obra y de coche, medir la precisión (SC-003), ajustar los umbrales de `src-tauri/src/audio/quality.rs` y anotar el resultado en `specs/002-voice-quote-draft/research.md` (R10).
- [ ] T078 Validar notas de voz reales de iPhone y Android, y anotar en `specs/002-voice-quote-draft/research.md` (R4) los formatos que no se puedan abrir.
- [ ] T079 [P] Revisar la accesibilidad de `src/features/voice-draft/`: letra de 16 px, objetivos de al menos 44 px, contraste AA, foco visible y anuncios `aria-live` del estado de grabación y de los totales.
- [ ] T080 [P] Actualizar `README.md` con los prerrequisitos, `npm run model:fetch` y el uso del dictado.
- [ ] T081 [P] Actualizar `CLAUDE.md`: feature en curso, dependencias nuevas, módulos `audio` y `transcription`, y la excepción de descarga del modelo solo al preparar el entorno.
- [ ] T082 Ejecutar `cargo test -j 2`, `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, `npm run test`, `npm run lint`, `npm run typecheck` y `npm run test:e2e`, y corregir lo que falle.
- [ ] T083 Ejecutar los 18 escenarios y las mediciones de `specs/002-voice-quote-draft/quickstart.md` y anotar el resultado en ese archivo.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sin dependencias. **T006 decide el modelo y bloquea el resto.**
- **Foundational (Phase 2)**: depende de Setup y bloquea todas las historias.
- **US1 (Phase 3)**: depende de Foundational.
- **US2, US3, US4 y US5 (Phases 4–7)**: dependen de US1, porque reutilizan su intérprete y su pantalla.
- **Polish (Phase 8)**: depende de las historias que se quieran entregar.

### User Story Dependencies

- **US1 (P1)**: solo depende de Foundational.
- **US2 (P1)**: necesita el borrador de US1. Sus pruebas de núcleo crean los datos directamente.
- **US3 (P1)**: verifica el flujo de US1 sin red.
- **US4 (P2)**: necesita el texto y el generador de US1. Su aviso de cambios manuales se prueba mejor con US2 terminada.
- **US5 (P2)**: necesita el generador de US1 y `VisitTextEditor` de US4. Su prueba e2e usa también la edición de US2.

### Within Each User Story

- Las pruebas se escriben primero y deben fallar.
- Dominio → audio y transcripción → servicios → comandos → interfaz.
- Una historia se valida en su checkpoint antes de pasar a la siguiente.

### Parallel Opportunities

- Setup: T003 y T004.
- Foundational: pruebas T008–T010, y T020.
- US1: pruebas T022–T029; dominio T030 y T031; audio T035–T038; interfaz T047 y T049.
- US2: pruebas T051–T053, y T059.
- US3: T061 y T062.
- US4: T066 y T067.
- US5: T072–T074.
- Polish: T079–T081.

---

## Parallel Example: User Story 1

```text
# Pruebas de dominio y audio, a la vez:
T022 src-tauri/tests/draft_numbers.rs
T023 src-tauri/tests/draft_parser.rs
T024 src-tauri/tests/draft_matcher.rs
T025 src-tauri/tests/draft_builder.rs
T026 src-tauri/tests/audio_pipeline.rs

# Componentes de audio independientes, a la vez:
T036 src-tauri/src/audio/wav.rs
T037 src-tauri/src/audio/resample.rs
T038 src-tauri/src/audio/quality.rs
```

Con `CARGO_BUILD_JOBS = 2`, escribir en paralelo es seguro, pero conviene compilar y ejecutar las pruebas de una en una.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Completar Setup, incluida la medición de T006.
2. Completar Foundational.
3. Completar US1.
4. **PARAR y VALIDAR** con el escenario 1 de quickstart.md.

### Incremental Delivery

1. Setup + Foundational → base lista.
2. US1 → dictar y ver el borrador.
3. US2 → corregirlo y darlo por bueno.
4. US3 → garantía sin conexión. **Aquí quedan completas las tres historias P1.**
5. US4 → corregir el texto y regenerar.
6. US5 → escribir sin dictar.
7. Polish → precisión real y cierre.

---

## Notes

- Las tareas marcadas con [P] tocan archivos distintos y no tienen dependencias pendientes.
- Hacer commit tras cada tarea o grupo lógico.
- Nunca calcular dinero en la interfaz ni usar `float` para importes o cantidades.
- Nunca asignar un precio que no proceda de una tarifa o de una entrada manual (Principio IV).
- T001, T006, T035, T065, T077, T078 y T083 pueden requerir herramientas, audios o pruebas manuales del responsable.
