# Implementation Plan: PDF del presupuesto

**Branch**: `003-quote-pdf-export` | **Date**: 2026-11-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/003-quote-pdf-export/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

El profesional convierte el borrador revisado en un PDF con su logo y sus datos. Antes de emitirlo completa los datos del cliente, revisa el número y la validez, y confirma de forma explícita. El núcleo Rust maqueta el documento con código puro, lo dibuja con `krilla` en PDF/A-2b y lo guarda de forma atómica en la ubicación elegida. Todo ocurre en el equipo: la tipografía va dentro del ejecutable y no se añaden plugins ni permisos.

## Technical Context

**Language/Version**: Rust estable (edición 2021) para el núcleo; TypeScript 5.x + React 18 para la interfaz (sin cambios)

**Primary Dependencies**:
- Existentes: Tauri 2, `tauri-plugin-dialog`, `rusqlite`, `rusqlite_migration`, `image`, `sha2`, `serde`, `thiserror`; React Router 6, React Hook Form y `@tauri-apps/api`.
- Nuevas en el núcleo: `krilla` 0.8.2 (`raster-images` y `simple-text`), `rustybuzz` 0.20 (medida del texto) y `time` 0.3 (`local-offset`).
- Nueva solo para pruebas: `lopdf` 0.47.
- Recursos: Liberation Sans 2.1.5 Regular y Bold, con licencia SIL OFL 1.1 ([research.md](research.md), R3).

**Storage**: SQLite existente (migración 003). El PDF se guarda únicamente en la ubicación elegida por el profesional

**Testing**: `cargo test` (dominio, servicios y estructura del PDF), Vitest + Testing Library + `mockIPC`, WebdriverIO + `tauri-driver` hasta la confirmación; validación manual del diálogo nativo y de los visores ([quickstart.md](quickstart.md))

**Target Platform**: Windows 10/11 x64 (WebView2)

**Project Type**: desktop-app (Tauri)

**Performance Goals**: presupuesto de 50 líneas en < 5 s en un portátil de 4 núcleos y 8 GB (SC-002); medido en la prueba aislada: 192 ms para 50 líneas y 800 ms para 200

**Constraints**: 100 % sin red en ejecución; sin plugins ni permisos nuevos; PDF/A-2b; escritura atómica; importes en céntimos enteros; compilación con `-j 2`

**Scale/Scope**: 1 usuario, 1 borrador en curso, hasta 200 líneas de 160 caracteres (SC-005), 1 diálogo nuevo dentro de la pantalla "Presupuesto". El ejecutable crece alrededor de 1 MB por las tipografías

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principio / restricción | Evaluación | Estado |
|---|---|---|
| **I. Offline-First** | La maquetación, la generación y el guardado son locales. La tipografía va dentro del ejecutable. Se valida con la red desactivada ([quickstart.md](quickstart.md), escenario 8) | ✅ PASS |
| **II. Sin servidor propio** | Los datos del cliente y la numeración solo residen en SQLite local, y el PDF en la carpeta elegida. La CSP y las capabilities no cambian | ✅ PASS |
| **III. IA con degradación elegante** | Esta feature no usa IA | ✅ N/A |
| **IV. Precios y marca del usuario** | El documento copia los precios y totales del borrador sin recalcularlos con otro criterio. Usa el logo y los datos del perfil. Las líneas pendientes bloquean la emisión | ✅ PASS |
| **V. Revisión humana antes de emitir** | La generación exige borrador revisado, sin pendientes y una confirmación explícita. El token garantiza que lo confirmado es lo que se emite ([research.md](research.md), R11) | ✅ PASS |
| **Datos locales y exportables** | Todo reside en el directorio local; la exportación sigue pendiente de su propia feature, como en la 001 y la 002 | ✅ PASS (exportación pendiente) |
| **Credenciales de terceros** | No hay | ✅ N/A |

**Re-check post-design (Phase 1)**: ✅ sin violaciones. El contrato IPC no contiene operaciones de red, el diálogo se abre desde Rust sin ampliar permisos, y cancelar o fallar no crea archivos ni gasta números.

## Project Structure

### Documentation (this feature)

```text
specs/003-quote-pdf-export/
├── plan.md              # Este archivo
├── research.md          # Fase 0: decisiones R1–R13 y mediciones
├── data-model.md        # Fase 1: tablas, bloqueos y estados
├── quickstart.md        # Fase 1: guía de validación de principio a fin
├── contracts/
│   ├── ipc-commands.md  # Fase 1: comandos Tauri nuevos
│   └── pdf-document.md  # Fase 1: contenido y maquetación del documento
├── checklists/
│   └── requirements.md
└── tasks.md             # Fase 2 (/speckit-tasks; no lo crea este comando)
```

### Source Code (repository root)

```text
src/
├── features/
│   ├── voice-draft/DraftPage.tsx      # añade el botón "Generar PDF"
│   └── quote-pdf/
│       ├── QuotePdfDialog.tsx         # confirmación y composición del diálogo
│       ├── QuoteDetailsForm.tsx       # cliente, número y validez
│       ├── QuoteBlockers.tsx          # motivos de bloqueo, avisos y accesos directos
│       ├── QuotePdfResult.tsx         # archivo guardado y errores con reintento
│       ├── useQuotePdf.ts
│       └── *.test.tsx
└── shared/
    ├── ipc/{client.ts,types.ts}       # comandos y tipos nuevos
    └── i18n/es.ts                     # textos nuevos

src-tauri/
├── Cargo.toml                         # dependencias nuevas
├── tauri.conf.json                    # licencia de la tipografía como recurso
├── assets/fonts/                      # Liberation Sans Regular y Bold
├── resources/licenses/                # texto de la licencia OFL
└── src/
    ├── lib.rs                         # registro de comandos
    ├── state.rs                       # generación activa y último destino elegido
    ├── error.rs                       # errores nuevos del contrato
    ├── commands/quote.rs
    ├── domain/quote_doc/
    │   ├── mod.rs                     # modelo del documento
    │   ├── details.rs                 # validación de cliente, número y validez
    │   ├── numbering.rs               # formato y siguiente número
    │   ├── dates.rs                   # emisión y validez
    │   ├── blockers.rs                # bloqueos y avisos
    │   └── layout.rs                  # saltos de renglón y paginación
    ├── pdf/
    │   ├── fonts.rs                   # tipografías y medida del texto
    │   ├── logo.rs                    # normalización del logo
    │   └── renderer.rs                # dibujo con krilla en PDF/A-2b
    ├── services/
    │   ├── quote_service.rs           # preparar, confirmar y generar
    │   └── draft_service.rs           # regenerar conserva los datos
    └── storage/
        ├── migrations/003_quote_pdf.sql
        ├── quote_repo.rs
        └── pdf_store.rs               # escritura atómica y clasificación de errores

src-tauri/tests/{quote_*,pdf_store}.rs # integración con DB y carpeta temporales
e2e/                                   # recorrido hasta la confirmación
```

**Structure Decision**: se mantiene la estructura de las features anteriores. Las reglas y la maquetación viven en `domain/quote_doc`, sin Tauri ni librerías de PDF, para probarlas de forma aislada. `pdf/` es un módulo nuevo porque envuelve `krilla` y las tipografías, igual que `audio/` y `transcription/` envuelven sus librerías nativas.

## Complexity Tracking

Sin violaciones de la constitución que justificar.
