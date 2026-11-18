# Implementation Plan: Borrador de presupuesto por voz

**Branch**: `002-voice-quote-draft` | **Date**: 2026-11-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-voice-quote-draft/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

El profesional dicta la visita en el ordenador, importa un audio grabado con el móvil o escribe el texto, y la app le propone un borrador de presupuesto calculado con sus tarifas, que revisa y corrige. Todo ocurre en el equipo: el micrófono se captura desde el núcleo Rust, el audio se normaliza a WAV, whisper.cpp lo transcribe con un modelo incluido en el instalador y un **intérprete determinista basado en reglas** (sin IA generativa) convierte el texto en líneas. Ese intérprete solo asigna precios que salen de una tarifa; cualquier duda queda como pendiente. El borrador, su texto y su audio se conservan en SQLite y en disco ante cierres inesperados.

## Technical Context

**Language/Version**: Rust estable (edición 2021) para el núcleo; TypeScript 5.x + React 18 para la interfaz (sin cambios respecto a la feature 001)

**Primary Dependencies**:
- Existentes: Tauri 2, `tauri-plugin-dialog`, `tauri-plugin-log`, `rusqlite`, `rusqlite_migration`, `unicode-normalization`, `serde`, `thiserror`; React Router 6, `@tauri-apps/api`.
- Nuevas en el núcleo: `whisper-rs` 0.16 (transcripción local), `cpal` 0.18 (micrófono), `symphonia` 0.6 (lectura de M4A, MP3 y WAV), `rubato` 5 (cambio de frecuencia a 16 kHz). Versiones comprobadas el 2026-11-18 (ver [research.md](research.md), R1–R4).
- Modelo: `ggml-small-q5_1.bin` (181 MiB), incluido como recurso del instalador.

**Storage**: SQLite existente (migración 002) y audio en `app_data_dir/recordings/` como WAV de 16 kHz, mono y 16 bits (≈ 1,9 MB por minuto)

**Testing**: `cargo test` (dominio, servicios con transcriptor simulado), Vitest + Testing Library + `mockIPC`, WebdriverIO + `tauri-driver` para el camino de texto escrito; validación manual del micrófono y de la precisión con un conjunto de audios reales ([quickstart.md](quickstart.md))

**Target Platform**: Windows 10/11 x64 (WebView2)

**Project Type**: desktop-app (Tauri)

**Performance Goals**: borrador de un dictado de 1 minuto en < 60 s en un portátil de 4 núcleos y 8 GB (SC-002); interpretar el texto y calcular el borrador con 500 tarifas en < 100 ms; actualizar importes y total en < 100 ms (SC-005 exige < 1 s percibido)

**Constraints**: 100 % sin red en ejecución; sin plugins ni permisos nuevos; dictados de 10 minutos como máximo; importes en céntimos enteros y cantidades en centésimas enteras; el modelo se carga solo mientras se transcribe, para liberar memoria; compilación con `-j 2`

**Scale/Scope**: 1 usuario, 1 borrador en curso, ~500 tarifas, 1 pantalla nueva ("Presupuesto") con panel de dictado, texto, líneas y totales. El instalador pasa a pesar unos 200 MB por el modelo

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principio / restricción | Evaluación | Estado |
|---|---|---|
| **I. Offline-First** | Captura, decodificación, transcripción, interpretación y edición son locales. El modelo viaja en el instalador. Se valida con la red desactivada ([quickstart.md](quickstart.md)) | ✅ PASS |
| **II. Sin servidor propio** | Audio, texto y borrador solo en `app_data_dir`. Sin backend, cuentas ni telemetría. La CSP y las capabilities no cambian | ✅ PASS |
| **III. IA con degradación elegante** | El motor local es el único y siempre está disponible; la API externa queda fuera de esta feature. Si el motor falla o falta el modelo, se conserva el audio, se puede reintentar y siempre se puede escribir el texto. Los caminos de fallo se prueban con un transcriptor simulado. La app informa de que la transcripción es local (FR-039) | ✅ PASS |
| **IV. Precios del usuario, nunca inventados** | Whisper solo produce texto. El precio lo asigna código determinista desde la tabla de tarifas o una entrada manual. Cada línea guarda origen, tarifa y precio. Sin tarifa aplicable, "precio pendiente" fuera del total | ✅ PASS |
| **V. Revisión humana antes de emitir** | No se genera ningún PDF. Todas las líneas son editables y "dar por bueno" exige una acción explícita sin pendientes. Los datos del cliente se revisarán en la feature del PDF, antes de emitir | ✅ PASS |
| **Datos locales y exportables** | Todo reside en un único directorio local; la exportación sigue pendiente de su propia feature, como en la 001 | ✅ PASS (exportación pendiente) |
| **Credenciales de terceros** | No hay | ✅ N/A |

**Re-check post-design (Phase 1)**: ✅ sin violaciones. El contrato IPC no contiene operaciones de red, el modelo de datos impide por `CHECK` una línea con precio sin origen, y el borrador anterior no se sustituye hasta que el nuevo está listo.

## Project Structure

### Documentation (this feature)

```text
specs/002-voice-quote-draft/
├── plan.md              # Este archivo
├── research.md          # Fase 0: decisiones R1–R12
├── data-model.md        # Fase 1: tablas, validaciones y estados
├── quickstart.md        # Fase 1: guía de validación de principio a fin
├── contracts/
│   └── ipc-commands.md  # Fase 1: comandos Tauri nuevos
├── checklists/
│   └── requirements.md
└── tasks.md             # Fase 2 (/speckit-tasks; no lo crea este comando)
```

### Source Code (repository root)

```text
src/
├── app/routes.tsx                     # añade la ruta "Presupuesto"
├── features/voice-draft/
│   ├── DraftPage.tsx                  # composición de la pantalla y estados
│   ├── DictationPanel.tsx             # grabar (botón y tecla F9), importar, progreso y errores
│   ├── VisitTextEditor.tsx            # texto de la visita y regenerar
│   ├── DraftLinesTable.tsx            # líneas, pendientes y sugerencias
│   ├── DraftLineRow.tsx
│   ├── AddLineDialog.tsx
│   ├── DraftTotals.tsx                # base, IVA, total y aviso de provisional
│   ├── useDraftSession.ts
│   └── *.test.tsx
└── shared/
    ├── ipc/{client.ts,types.ts}       # comandos y tipos nuevos
    └── i18n/es.ts                     # textos nuevos

src-tauri/
├── Cargo.toml                         # dependencias nuevas y feature `local-transcription`
├── tauri.conf.json                    # recurso del modelo
├── resources/models/                  # modelo, fuera de git
└── src/
    ├── lib.rs                         # registro de comandos y recuperación al arrancar
    ├── state.rs                       # grabación y proceso activos
    ├── error.rs                       # errores nuevos del contrato
    ├── commands/{session.rs,dictation.rs,draft.rs}
    ├── domain/
    │   ├── money.rs                   # cantidades, importes e IVA
    │   └── draft/
    │       ├── numbers.rs             # cantidades en cifras y palabras
    │       ├── units.rs               # alias de unidades
    │       ├── parser.rs              # texto → trabajos
    │       ├── matcher.rs             # trabajos → tarifas (FR-017 a FR-021)
    │       ├── builder.rs             # borrador completo y determinista
    │       └── totals.rs
    ├── audio/
    │   ├── capture.rs                 # micrófono en un hilo propio
    │   ├── wav.rs                     # escritura incremental y reparación
    │   ├── decode.rs                  # importación M4A, MP3 y WAV
    │   ├── resample.rs
    │   └── quality.rs                 # silencio y calidad baja
    ├── transcription/
    │   ├── mod.rs                     # trait `Transcriber`
    │   └── whisper.rs                 # implementación real
    ├── services/{dictation_service.rs,draft_service.rs}
    └── storage/
        ├── migrations/002_voice_draft.sql
        ├── draft_repo.rs
        ├── dictation_repo.rs
        └── recording_store.rs

src-tauri/tests/                       # integración con DB temporal y transcriptor simulado
├── fixtures/audio/                    # audios cortos sintéticos para decodificación
└── draft_*.rs, dictation_*.rs
scripts/fetch-model.mjs                # descarga y verifica el modelo al preparar el entorno
e2e/                                   # recorrido del texto escrito hasta dar por bueno
```

**Structure Decision**: se mantiene la estructura de la feature 001. La interpretación y el cálculo viven en `domain/draft`, sin Tauri ni audio, para probarlos de forma aislada. `audio/` y `transcription/` son módulos nuevos porque envuelven dispositivos y librerías nativas. El transcriptor real queda detrás de un trait y de la feature de Cargo `local-transcription`, de modo que las pruebas de dominio se pueden compilar sin whisper.cpp en un equipo con poca memoria.

## Complexity Tracking

Sin violaciones de la constitución que justificar.
