---

description: "Lista de tareas para implementar el PDF del presupuesto"
---

# Tasks: PDF del presupuesto

**Input**: Documentos de diseño en `/specs/003-quote-pdf-export/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/ipc-commands.md](contracts/ipc-commands.md), [contracts/pdf-document.md](contracts/pdf-document.md), [quickstart.md](quickstart.md)

**Tests**: se incluyen porque CLAUDE.md exige TDD. En cada historia, las pruebas se escriben primero y deben fallar antes de implementar.

**Organization**: tareas agrupadas por historia de usuario, para implementar y probar cada una por separado.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: se puede ejecutar en paralelo (otro archivo, sin dependencias pendientes).
- **[Story]**: historia a la que pertenece (US1–US4, según spec.md).
- Rutas relativas a la raíz del repositorio: `src/` es la interfaz React y `src-tauri/` el núcleo Rust.
- Las pruebas de integración usan el crate `app_lib`, así que el módulo nuevo `pdf` debe ser `pub`.
- **Registro de comandos**: todos se registran en `src-tauri/src/lib.rs`, así que las tareas que lo tocan no son paralelas entre sí.
- **Contrato IPC**: cualquier cambio se hace primero en [contracts/ipc-commands.md](contracts/ipc-commands.md), después en `src/shared/ipc/types.ts` y por último en Rust.
- **Compilación**: usar siempre `$env:CARGO_BUILD_JOBS = 2`. Con más procesos falla por falta de memoria.
- **Dinero**: el documento copia los importes ya calculados por `domain/draft/totals.rs` y `domain/money.rs`. Las coordenadas de maquetación pueden usar `f32`; los importes, nunca.
- **Registros técnicos**: `log::error!` solo recibe el tipo de fallo y el código del sistema. Nunca datos del cliente, contenido del presupuesto ni la ruta del archivo (FR-028b).
- Marcar cada tarea completada con `- [X]`.

---

## Phase 1: Setup (infraestructura compartida)

**Purpose**: añadir las dependencias y los recursos de la tipografía.

- [X] T001 Añadir a `src-tauri/Cargo.toml` las dependencias `krilla` 0.8.2 (`default-features = false`, features `raster-images` y `simple-text`), `rustybuzz` 0.20 y `time` 0.3 (feature `local-offset`), y `lopdf` 0.47 en `[dev-dependencies]`.
- [X] T002 [P] Descargar `https://github.com/liberationfonts/liberation-fonts/files/7261482/liberation-fonts-ttf-2.1.5.tar.gz`, copiar `LiberationSans-Regular.ttf` y `LiberationSans-Bold.ttf` a `src-tauri/assets/fonts/` y comprobar que sus SHA-256 coinciden con research.md (R3). Si no coinciden, detenerse y avisar al responsable.
- [X] T003 [P] Copiar el archivo `LICENSE` de ese paquete a `src-tauri/resources/licenses/LiberationFonts-OFL.txt` y añadirlo a `bundle.resources` en `src-tauri/tauri.conf.json`, sin modificar la CSP.
- [X] T004 Ejecutar `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features` y confirmar que compila con las dependencias nuevas.

**Checkpoint**: el proyecto compila con `krilla` y las tipografías están verificadas.

---

## Phase 2: Foundational (requisitos que bloquean todo)

**Purpose**: esquema de datos, errores, módulos vacíos, cliente IPC y textos.

**⚠️ CRITICAL**: ninguna historia puede empezar hasta terminar esta fase.

### Pruebas (escribir primero)

- [X] T005 [P] Ampliar `src-tauri/tests/db_migrations.rs`: la migración 3 se aplica; una segunda fila en `quote_details` falla; `validity_days` fuera de 1–365 falla; borrar `quote_draft` borra `quote_details` en cascada; `quote_number_sequences` no cambia al borrar el borrador.
- [X] T006 Añadir a `src-tauri/src/error.rs` una prueba de serialización de `quoteBlocked`, `quoteChanged`, `pdfSave` (con cada `PdfSaveCode`), `pdfRender` y del código `VALIDITY_INVALID`, según [contracts/ipc-commands.md](contracts/ipc-commands.md).

### Implementación

- [X] T007 Ampliar `src-tauri/src/error.rs` con `QuoteBlocked`, `QuoteChanged`, `PdfSave`, `PdfRender`, los tipos `QuoteBlocker` y `PdfSaveCode`, y el código `ValidityInvalid`, con mensajes en español sencillo (FR-029).
- [X] T008 Crear `src-tauri/src/storage/migrations/003_quote_pdf.sql` con `quote_details` y `quote_number_sequences` según [data-model.md](data-model.md) y registrarla en `src-tauri/src/storage/db.rs`.
- [X] T009 Crear `src-tauri/src/domain/quote_doc/mod.rs` con los tipos `QuoteDetails`, `QuoteDocument`, `DocumentLine`, `DocumentTotals` y los submódulos `details`, `numbering`, `dates`, `blockers` y `layout` vacíos; declarar `quote_doc` en `src-tauri/src/domain/mod.rs`.
- [X] T010 Crear `src-tauri/src/storage/quote_repo.rs` con lectura y guardado de `quote_details`, y lectura y actualización de `quote_number_sequences` con `MAX(last_number, usado)`; declararlo en `src-tauri/src/storage/mod.rs`.
- [X] T011 Crear los módulos vacíos `src-tauri/src/pdf/mod.rs`, `src-tauri/src/pdf/fonts.rs`, `src-tauri/src/pdf/logo.rs`, `src-tauri/src/pdf/renderer.rs`, `src-tauri/src/storage/pdf_store.rs`, `src-tauri/src/services/quote_service.rs` y `src-tauri/src/commands/quote.rs`; declararlos en sus `mod.rs` y `pub mod pdf` en `src-tauri/src/lib.rs`.
- [X] T012 Ampliar `src-tauri/src/state.rs` con la señal de generación activa y el último destino elegido, que se guarda solo en memoria.
- [X] T013 Añadir a `src/shared/ipc/types.ts` los tipos del contrato y a `src/shared/ipc/client.ts` las funciones `quotePrepare`, `quoteDetailsSave` y `quoteGenerate`.
- [X] T014 [P] Añadir a `src/shared/i18n/es.ts` todos los textos de la generación del PDF: formulario, confirmación, bloqueos, avisos, errores de guardado y resultado (FR-029).

**Checkpoint**: la migración se aplica, los errores siguen el contrato y el proyecto compila.

---

## Phase 3: User Story 1 - Generar el PDF del presupuesto revisado (Priority: P1) 🎯 MVP

**Goal**: desde un borrador revisado, el profesional completa los datos del cliente, confirma y obtiene un PDF con su logo, sus datos, las líneas y los totales.

**Independent Test**: con un perfil completo, un logo y un borrador revisado, se genera el PDF, se abre en un visor distinto de la app y se compara línea a línea con el borrador.

### Tests for User Story 1 ⚠️

> **NOTE: escribir estas pruebas primero y comprobar que fallan antes de implementar.**

- [X] T015 [P] [US1] Crear `src-tauri/tests/quote_numbering.rs`: primer número del año `2026-001`; `2026-007` propone `2026-008`; `2026-999` propone `2026-1000`; un número con formato año-número actualiza la serie con el mayor valor; `2026-07` y `OBRA-7` no la alteran; orden de la propuesta de data-model.md: número escrito a mano, número emitido si la huella coincide y siguiente de la serie (FR-013, FR-013b y FR-013c).
- [X] T016 [P] [US1] Crear `src-tauri/tests/quote_dates.rs`: 18/11/2026 + 30 días = 18/12/2026; cambio de mes y de año; 29 de febrero; validez de 1 y 365 días; formato `dd/mm/aaaa` (FR-014 y FR-014b).
- [X] T017 [P] [US1] Crear `src-tauri/tests/quote_details.rs`: limpieza de espacios; opcionales vacíos pasan a `None`; longitudes máximas de data-model.md con `TOO_LONG`; validez 0, 366 y negativa con `VALIDITY_INVALID`.
- [X] T018 [P] [US1] Crear `src-tauri/tests/quote_layout.rs` con un medidor simulado: los datos obligatorios y opcionales del negocio aparecen en la cabecera (FR-007); saltos de renglón por espacios y por caracteres; ninguna fila partida entre páginas; cabecera de tabla repetida; totales una sola vez y sin separar; `Página X de Y` en todas las páginas; sin logo no queda hueco; datos opcionales vacíos sin renglones; importe ancho reducido hasta 7 pt y después en varios renglones; 200 líneas de 160 caracteres sin perder texto (FR-012, FR-017 a FR-019 y SC-005).
- [X] T019 [P] [US1] Crear `src-tauri/tests/quote_pdf_render.rs` con `lopdf`: páginas A4, tipografías incrustadas, marca PDF/A-2b, título y número de páginas correctos; un PNG con transparencia conserva su máscara; un JPG se incrusta; un documento sin logo se genera (FR-006, FR-015 y FR-016).
- [X] T020 [P] [US1] Crear `src-tauri/tests/quote_service.rs`: con el ejemplo de la spec, el documento contiene 75,00 €, 60,00 €, 135,00 €, 28,35 € y 163,35 € sin información interna (FR-008 a FR-011); guardar los datos no cambia el estado de revisión; se propone el siguiente número; el número se gasta al guardar; un segundo PDF sin cambios, o con cambios solo en el cliente o la validez, propone el mismo; tras cambiar una línea o el IVA propone el siguiente (FR-013b); un token caducado devuelve `quoteChanged`, también al cambiar la fecha; una generación simultánea devuelve `invalidState` (FR-026); sin nombre del cliente o sin número no hay token y `generate_to_path` devuelve `quoteBlocked` (FR-001 y FR-012).
- [X] T021 [P] [US1] Ampliar `src-tauri/tests/draft_regenerate.rs`: regenerar conserva `quote_details`; un borrador nuevo, un dictado nuevo y descartar los vacían (FR-012b).
- [X] T022 [P] [US1] Crear `src/features/quote-pdf/QuotePdfDialog.test.tsx` con IPC simulado: muestra número, fechas, líneas y total; cancelar no llama a `quote_generate`; el nombre del cliente vacío impide generar y muestra el motivo; corregir un dato llama a `quote_details_save`; al escribir un dato y pulsar confirmar directamente, un solo clic guarda los datos y llama a `quote_generate` con el token devuelto; el botón se desactiva solo durante la generación.

### Implementation for User Story 1

- [X] T023 [P] [US1] Implementar `src-tauri/src/domain/quote_doc/numbering.rs` según research.md (R9), incluidas la huella de contenido y el orden de la propuesta de data-model.md.
- [X] T024 [P] [US1] Implementar `src-tauri/src/domain/quote_doc/dates.rs` con `time`, recibiendo la fecha como parámetro (R12).
- [X] T025 [P] [US1] Implementar `src-tauri/src/domain/quote_doc/details.rs` según data-model.md.
- [X] T026 [US1] Implementar `src-tauri/src/domain/quote_doc/layout.rs` con el trait `TextMeasurer` y las reglas de [contracts/pdf-document.md](contracts/pdf-document.md).
- [X] T027 [P] [US1] Implementar `src-tauri/src/pdf/fonts.rs`: tipografías con `include_bytes!` y medida con `rustybuzz`, con cada texto medido en su propia tipografía (R3 y R4).
- [X] T028 [P] [US1] Implementar `src-tauri/src/pdf/logo.rs`: decodificar, reducir a 640 px como máximo sin ampliar y reescribir como PNG RGBA (R6).
- [X] T029 [US1] Implementar `src-tauri/src/pdf/renderer.rs`: dibujar el resultado de `layout.rs` con `krilla` en PDF/A-2b, con título, idioma `es-ES`, fecha del documento e interpolación de imagen desactivada (R1 y R2).
- [X] T030 [US1] Implementar en `src-tauri/src/storage/pdf_store.rs` la escritura atómica: temporal `.presupuesto.tmp` en la misma carpeta, volcado a disco, renombrado y borrado del temporal ante un fallo (R8).
- [X] T031 [US1] Implementar en `src-tauri/src/services/quote_service.rs` `prepare`, `save_details` y `generate_to_path`, con el token SHA-256 (R11), el orden de la transacción de R9 y los bloqueos `clientNameRequired` y `quoteNumberRequired` en `src-tauri/src/domain/quote_doc/blockers.rs`.
- [X] T032 [US1] Modificar `regenerate` en `src-tauri/src/services/draft_service.rs` para conservar `quote_details` en la misma transacción (R10).
- [X] T033 [US1] Implementar en `src-tauri/src/commands/quote.rs` `quote_prepare`, `quote_details_save` y `quote_generate`, con el diálogo nativo abierto desde Rust en un hilo de bloqueo y filtro PDF, y registrarlos en `src-tauri/src/lib.rs`.
- [X] T034 [P] [US1] Crear `src/features/quote-pdf/useQuotePdf.ts` con el estado de preparación, guardado de datos y generación.
- [X] T035 [P] [US1] Crear `src/features/quote-pdf/QuoteDetailsForm.tsx` con React Hook Form: cliente, número y validez, con objetivos de 44 px y foco visible.
- [X] T036 [US1] Crear `src/features/quote-pdf/QuotePdfDialog.tsx` con el resumen, el formulario y la confirmación explícita (FR-004). Al confirmar, espera a `quote_details_save` y usa el token que devuelve, según [contracts/ipc-commands.md](contracts/ipc-commands.md).
- [X] T037 [US1] Añadir el botón "Generar PDF" y el diálogo a `src/features/voice-draft/DraftPage.tsx`. El botón se muestra siempre que exista un borrador, aunque no esté revisado o tenga líneas pendientes.

**Checkpoint**: un borrador revisado produce un PDF correcto en la carpeta elegida.

---

## Phase 4: User Story 2 - Impedir el PDF mientras falten datos (Priority: P1)

**Goal**: la app explica qué falta, lleva a completarlo y no crea ningún archivo hasta resolverlo.

**Independent Test**: con un borrador que tiene una línea sin precio, se intenta generar el PDF y se comprueba que no se crea ningún archivo y que la app señala esa línea; al completarla, el PDF se genera.

### Tests for User Story 2 ⚠️

- [X] T038 [P] [US2] Crear `src-tauri/tests/quote_blockers.rs`: cada bloqueo de data-model.md por separado y combinados; las líneas pendientes se enumeran por concepto, tanto de precio como de cantidad; un emoji en un concepto, en el cliente o en el perfil devuelve el dato y el carácter (FR-001 a FR-003 y FR-018b).
- [X] T039 [P] [US2] Ampliar `src-tauri/tests/quote_service.rs`: `generate_to_path` con cualquier bloqueo devuelve `quoteBlocked`, no crea archivo y no gasta número (SC-004); un logo ilegible bloquea salvo con `allowMissingLogo`; los avisos `noLogo`, `vatChanged` y `unparsedFragments` no bloquean; actualizar el IVA devuelve el borrador a revisión (FR-005 y FR-005b).
- [X] T040 [P] [US2] Crear `src/features/quote-pdf/QuoteBlockers.test.tsx` con IPC simulado: enumera las líneas pendientes, ofrece ir a completarlas y a "Mi negocio", muestra el carácter afectado, muestra los avisos y permite seguir sin logo.

### Implementation for User Story 2

- [X] T041 [US2] Completar `src-tauri/src/domain/quote_doc/blockers.rs` con el resto de bloqueos y los avisos de data-model.md.
- [X] T042 [US2] Añadir a `src-tauri/src/pdf/fonts.rs` la comprobación de caracteres disponibles en la tipografía (FR-018b).
- [X] T043 [US2] Integrar bloqueos y avisos en `prepare` y `generate_to_path` de `src-tauri/src/services/quote_service.rs`, incluido `allowMissingLogo`.
- [X] T044 [US2] Crear `src/features/quote-pdf/QuoteBlockers.tsx` con los motivos, los avisos y los accesos directos a las líneas y a "Mi negocio".
- [X] T045 [US2] Integrar `QuoteBlockers` en `src/features/quote-pdf/QuotePdfDialog.tsx`, incluida la opción de actualizar el IVA con `draft_apply_current_vat`.

**Checkpoint**: ningún presupuesto incompleto llega a crear un archivo.

---

## Phase 5: User Story 3 - Elegir dónde se guarda el PDF (Priority: P1)

**Goal**: el profesional elige carpeta y nombre, sabe dónde ha quedado el archivo y puede resolver cualquier fallo de guardado.

**Independent Test**: se genera un PDF eligiendo una carpeta concreta, se comprueba que el archivo está en ella y que se abre desde allí.

### Tests for User Story 3 ⚠️

- [X] T046 [P] [US3] Crear `src-tauri/tests/pdf_store.rs`: tabla de códigos de Windows de research.md (R8); carpeta inexistente; archivo de destino bloqueado con `share_mode(0)` devuelve `PDF_FILE_IN_USE` y lo conserva intacto; sustituir un archivo funciona; tras un fallo no queda ningún temporal (FR-023 y FR-024).
- [X] T047 [P] [US3] Ampliar `src-tauri/tests/quote_service.rs`: nombre propuesto `Presupuesto 2026-001 2026-11-18.pdf`; el número `OBRA/7` se convierte en `OBRA-7` dentro del nombre; se añade `.pdf` si falta y no se duplica; un fallo de guardado no gasta el número ni modifica el borrador (FR-013b, FR-020 y FR-024).
- [X] T048 [P] [US3] Crear `src/features/quote-pdf/QuotePdfResult.test.tsx` con IPC simulado: muestra el nombre y la ruta completa; cancelar el diálogo vuelve a la confirmación sin error; cada `PdfSaveCode` muestra su mensaje y permite reintentar sin repetir la confirmación (FR-021, FR-023 y FR-025).

### Implementation for User Story 3

- [X] T049 [US3] Añadir a `src-tauri/src/storage/pdf_store.rs` la clasificación de errores en `PdfSaveCode` según R8, registrando solo el código del sistema.
- [X] T050 [US3] Añadir a `src-tauri/src/services/quote_service.rs` el nombre propuesto con el número y la fecha, la sustitución de caracteres no admitidos y la comprobación de la extensión `.pdf`.
- [X] T051 [US3] Ampliar `quote_generate` en `src-tauri/src/commands/quote.rs` para proponer el nombre, asociar el diálogo a la ventana principal y reabrirlo con el último destino tras un fallo.
- [X] T052 [US3] Crear `src/features/quote-pdf/QuotePdfResult.tsx` con la confirmación del guardado y los errores con reintento.
- [X] T053 [US3] Integrar `QuotePdfResult` en `src/features/quote-pdf/QuotePdfDialog.tsx`.

**Checkpoint**: el archivo queda donde se eligió y ningún fallo de guardado pierde trabajo.

---

## Phase 6: User Story 4 - Generar el PDF sin conexión (Priority: P2)

**Goal**: la generación funciona igual sin conexión a Internet.

**Independent Test**: con la red del equipo desactivada, se repite la generación de la Historia 1 y se obtiene el mismo documento.

### Tests for User Story 4 ⚠️

- [X] T054 [US4] Ampliar `src-tauri/tests/offline_guarantees.rs`: los archivos de `src/pdf/`, `src/services/quote_service.rs`, `src/storage/pdf_store.rs` y `src/commands/quote.rs` no contienen direcciones de red (FR-027 y FR-028). La CSP, los permisos y las dependencias de red ya están cubiertos por las pruebas existentes.

### Implementation for User Story 4

- [X] T055 [US4] Validar a mano el escenario 8 de `specs/003-quote-pdf-export/quickstart.md` con la red desactivada, cubriendo generación, bloqueo y guardado (SC-008), y anotar el resultado en esta tarea. **Resultado (2026-11-19)**: validado por el responsable; generación, bloqueo y guardado funcionan sin red y sin avisos de conexión.

**Checkpoint**: todas las historias funcionan con la red desactivada.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: comprobaciones finales y validación manual.

- [X] T056 [P] Añadir a `e2e/specs/4-quote-pdf.e2e.ts` el recorrido del texto escrito hasta la confirmación del PDF, incluido el bloqueo por falta de nombre del cliente. El diálogo nativo no se automatiza.
- [X] T057 Revisar `src-tauri/src/services/quote_service.rs`, `src-tauri/src/storage/pdf_store.rs` y `src-tauri/src/commands/quote.rs` para confirmar que ningún registro incluye datos del cliente, contenido del presupuesto ni rutas (FR-028b).
- [X] T058 Ejecutar `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features`, `npm run test`, `npm run lint`, `npm run typecheck` y `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, y corregir lo que falle.
- [X] T059 Medir la generación de un presupuesto de 50 líneas en la app compilada en modo release y anotar el resultado en `specs/003-quote-pdf-export/research.md` (SC-002). **Resultado (2026-11-19)**: mediana de 172 ms y peor caso de 750 ms con `examples/quote_pdf_benchmark.rs`; cumple.
- [X] T060 Validar a mano los escenarios 1 a 7 de `specs/003-quote-pdf-export/quickstart.md`, incluidos Acrobat, Edge y Chrome (SC-006), la sustitución de archivos (FR-022) y la falta de espacio, y cronometrar el paso de un borrador revisado al PDF guardado, que debe durar menos de 1 minuto (SC-001).
- [X] T061 Validar un documento generado con veraPDF, perfil PDF/A-2b, y anotar el resultado en `specs/003-quote-pdf-export/research.md`.
- [X] T062 [P] Actualizar `CLAUDE.md`: feature en curso, dependencias nuevas, carpeta `src/features/quote-pdf/`, módulo `src-tauri/src/pdf/` y la descarga única de la tipografía al preparar el repositorio (T002) como excepción a la regla de red.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sin dependencias.
- **Foundational (Phase 2)**: depende de Setup y bloquea todas las historias.
- **US1 (Phase 3)**: depende de Foundational.
- **US2 (Phase 4) y US3 (Phase 5)**: dependen de US1, porque amplían su servicio y su diálogo.
- **US4 (Phase 6)**: depende de US1. Conviene hacerla al final para validar el flujo completo.
- **Polish (Phase 7)**: depende de todas las historias.

### User Story Dependencies

- **US1 (P1)**: es la base del flujo y no depende de otras historias.
- **US2 (P1)**: añade los bloqueos sobre US1. Se puede hacer antes o después de US3.
- **US3 (P1)**: completa el guardado de US1. Se puede hacer antes o después de US2.
- **US4 (P2)**: verifica las demás sin añadir funciones.

### Within Each User Story

- Las pruebas se escriben primero y deben fallar.
- Dominio antes que `pdf/` y `storage/`, estos antes que `services/`, y los servicios antes que `commands/` y la interfaz.
- T026 va antes de T029, y T029 y T030 antes de T031.
- T031 va antes de T033, y T034 y T035 antes de T036.

### Parallel Opportunities

- T002 y T003 en Setup.
- T005 y T014 en Foundational.
- T015 a T022: todas las pruebas de US1.
- T023 a T025, T027 y T028: dominio y recursos de US1.
- T034 y T035: interfaz de US1.
- T038 a T040: pruebas de US2.
- T046 a T048: pruebas de US3.
- US2 y US3 pueden avanzar a la vez solo si se coordinan los cambios en `src-tauri/src/services/quote_service.rs`, `src-tauri/tests/quote_service.rs` y `src/features/quote-pdf/QuotePdfDialog.tsx`.

---

## Parallel Example: User Story 1

```text
# Pruebas de dominio de US1:
T015 src-tauri/tests/quote_numbering.rs
T016 src-tauri/tests/quote_dates.rs
T017 src-tauri/tests/quote_details.rs
T018 src-tauri/tests/quote_layout.rs

# Implementación independiente de US1:
T023 src-tauri/src/domain/quote_doc/numbering.rs
T024 src-tauri/src/domain/quote_doc/dates.rs
T025 src-tauri/src/domain/quote_doc/details.rs
T027 src-tauri/src/pdf/fonts.rs
T028 src-tauri/src/pdf/logo.rs
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Completar Setup y Foundational.
2. Completar US1.
3. **Parar y validar**: generar un PDF real y compararlo con el borrador.

US1 sirve para probar el documento, pero **no debe entregarse sola**: el bloqueo de US2 es obligatorio según el Principio V de la constitución.

### Incremental Delivery

1. Setup y Foundational: base lista.
2. US1: documento correcto.
3. US2: emisión segura. **Primer incremento entregable.**
4. US3: guardado completo con todos sus fallos.
5. US4 y Polish: validación final.

---

## Notes

- Las tareas [P] tocan archivos distintos y no tienen dependencias pendientes.
- La etiqueta [Story] vincula cada tarea con su historia.
- Comprobar que las pruebas fallan antes de implementar.
- Las tareas T055, T060 y T061 son manuales y requieren al responsable del proyecto.
