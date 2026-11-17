# Implementation Plan: Perfil del negocio y tarifas

**Branch**: `001-business-profile-rates` | **Date**: 2026-11-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-business-profile-rates/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

El profesional configura una vez los datos de su negocio (identidad fiscal, contacto, dirección, IVA general y logo) y mantiene su lista de tarifas (concepto, unidad, precio sin IVA), con búsqueda que no distingue mayúsculas ni tildes. Es la primera feature del proyecto, así que también fija el stack: app de escritorio **Tauri 2** con núcleo **Rust** (dominio, validaciones, SQLite local y gestión del logo) e interfaz **React + TypeScript**. Se comunican solo mediante comandos IPC tipados ([contracts/ipc-commands.md](contracts/ipc-commands.md)). Todo funciona sin red y los datos residen solo en el equipo del usuario.

## Technical Context

**Language/Version**: Rust estable (edición 2021) para el núcleo; TypeScript 5.x + React 18 para la interfaz

**Primary Dependencies**:
- Núcleo: Tauri 2.x, `tauri-plugin-dialog`, `rusqlite` (`bundled`), `rusqlite_migration`, `image` (PNG/JPEG), `unicode-normalization`, `sha2`, `serde`, `thiserror`.
- Interfaz: Vite, React Router 6, React Hook Form, `@tauri-apps/api`.

**Storage**: SQLite local en `app_data_dir` (modo WAL) y archivo de logo en `app_data_dir/branding/`

**Testing**: `cargo test` (con `tempfile`), Vitest + React Testing Library + `mockIPC`, WebdriverIO + `tauri-driver` para e2e

**Target Platform**: Windows 10/11 x64 (WebView2) como objetivo principal; macOS y Linux posibles con el mismo código sin compromiso en esta feature

**Project Type**: desktop-app (Tauri: frontend web embebido + núcleo nativo)

**Performance Goals**: búsqueda y filtrado < 50 ms con 500 tarifas (SC-005 exige < 1 s percibido); arranque en frío < 2 s; guardar el perfil o una tarifa < 100 ms

**Constraints**: 100 % offline (sin plugins de red, CSP sin orígenes remotos); logo PNG/JPG ≤ 5 MB; importes en céntimos enteros; interfaz en español sencillo; instalador < 15 MB

**Scale/Scope**: 1 usuario, 1 perfil, ~500 tarifas como máximo previsto, 2 pantallas (Mi negocio, Tarifas) y 1 diálogo de tarifa

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principio / restricción | Evaluación | Estado |
|---|---|---|
| **I. Offline-First** | Ninguna función usa la red; la CSP bloquea los orígenes remotos (R9); la prueba manual se hace con la red desactivada (quickstart) | ✅ PASS |
| **II. Sin servidor propio** | SQLite y archivos en el equipo del usuario; sin backend, cuentas, telemetría ni sincronización | ✅ PASS |
| **III. IA con degradación elegante** | La feature no usa IA. El stack elegido admite whisper.cpp local (`whisper-rs`) para la feature de voz | ✅ N/A (stack compatible) |
| **IV. Precios y marca del usuario** | Esta feature crea la fuente de verdad de los precios (tarifas del usuario) y de la marca (logo y datos). Importes exactos en céntimos; precio > 0 obligatorio | ✅ PASS |
| **V. Revisión humana antes de emitir** | Sin generación de PDF en esta feature | ✅ N/A |
| **Datos locales y exportables** | Datos en un único directorio local; la exportación queda para una feature posterior (spec, Assumptions), sin bloquear este principio | ✅ PASS (exportación pendiente) |
| **Credenciales de terceros** | No hay | ✅ N/A |
| **Stack compatible con I–V** | Tauri + Rust: ejecución local, transcripción local viable, PDF local viable | ✅ PASS |

**Re-check post-design (Phase 1)**: ✅ sin violaciones. El contrato IPC no contiene ninguna operación de red; el modelo de datos guarda los precios como enteros exactos; el logo se guarda como copia local.

## Project Structure

### Documentation (this feature)

```text
specs/001-business-profile-rates/
├── plan.md              # Este archivo
├── research.md          # Fase 0: decisiones R1–R10
├── data-model.md        # Fase 1: tablas, validaciones, estado derivado
├── quickstart.md        # Fase 1: guía de validación de principio a fin
├── contracts/
│   └── ipc-commands.md  # Fase 1: contrato de comandos Tauri
├── checklists/
│   └── requirements.md
└── tasks.md             # Fase 2 (/speckit-tasks; no lo crea este comando)
```

### Source Code (repository root)

```text
src/                                  # Interfaz React + TypeScript
├── main.tsx
├── app/
│   ├── App.tsx                       # rutas, layout y aviso de perfil incompleto
│   └── routes.tsx
├── features/
│   ├── business-profile/
│   │   ├── BusinessProfilePage.tsx   # formulario y modo guiado (FR-001c)
│   │   ├── LogoPicker.tsx
│   │   ├── useBusinessProfile.ts
│   │   └── *.test.tsx
│   └── rates/
│       ├── RatesPage.tsx             # lista, búsqueda y estados vacío / sin resultados
│       ├── RateFormDialog.tsx
│       ├── UnitSelect.tsx
│       ├── useRates.ts
│       └── *.test.tsx
└── shared/
    ├── ipc/                          # envoltorio de invoke() y types.ts (espejo del contrato)
    ├── components/                   # Button, Field, ConfirmDialog, UnsavedChangesGuard…
    ├── i18n/es.ts                    # todos los textos de la interfaz
    └── styles/tokens.css

src-tauri/                            # Núcleo Rust
├── Cargo.toml
├── tauri.conf.json                   # CSP sin orígenes remotos
├── capabilities/default.json         # solo dialog:allow-open y comandos propios
└── src/
    ├── main.rs
    ├── lib.rs                        # construcción de la app y registro de comandos
    ├── error.rs                      # AppError → JSON del contrato
    ├── commands/                     # capa fina: profile.rs, logo.rs, units.rs, rates.rs
    ├── domain/
    │   ├── normalize.rs              # R5
    │   ├── money.rs                  # parseo y formato de céntimos y puntos básicos (R4)
    │   ├── validation/               # nif.rs, postal_code.rs, phone.rs, email.rs (R6)
    │   ├── profile.rs                # BusinessProfile y missing_fields (R7)
    │   └── rate.rs
    ├── services/                     # profile_service.rs, logo_service.rs, rate_service.rs
    └── storage/
        ├── db.rs                     # apertura, WAL y migraciones
        ├── migrations/001_init.sql
        ├── profile_repo.rs
        ├── unit_repo.rs
        ├── rate_repo.rs
        └── logo_store.rs             # escritura atómica en branding/

src-tauri/tests/                      # integración Rust (repositorios y servicios con DB temporal)
e2e/                                  # WebdriverIO + tauri-driver (recorridos del quickstart)
```

**Structure Decision**: estructura estándar de Tauri 2: `src/` para la interfaz y `src-tauri/` para el núcleo. La lógica de negocio (validaciones, normalización, dinero) vive solo en `src-tauri/src/domain`, sin dependencias de Tauri, para probarla de forma aislada. `commands/` es una capa fina que traduce entre IPC y `services/`. La interfaz se organiza por features (`business-profile`, `rates`), preparada para añadir `quotes` y `voice` en features posteriores.

## Complexity Tracking

Sin violaciones de la constitución que justificar.
