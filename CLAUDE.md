# CLAUDE.md

Guía para Claude Code al trabajar en este repositorio.

## Qué es este proyecto

**Presupuestos por Voz** (nombre de trabajo): aplicación de escritorio para profesionales de oficios
(fontanería, electricidad, pintura…) que genera presupuestos en PDF a partir de la visita al cliente
dictada por voz. Un solo usuario por instalación, sin conocimientos técnicos.

## Estado actual

- El proyecto se desarrolla con **Spec Kit** (Spec-Driven Development, v0.16.5, scripts PowerShell).
- Feature en curso: `003-quote-pdf-export` (PDF del presupuesto), en la rama del mismo nombre.
  Implementadas y validadas US1–US4; todas las tareas de `tasks.md` completadas.
- Features anteriores: `002-voice-quote-draft` (borrador de presupuesto por voz) y
  `001-business-profile-rates` (perfil del negocio, logo y tarifas).
- Equipo de desarrollo con poca memoria: compila Rust con `-j 2` (o `$env:CARGO_BUILD_JOBS = 2`).
  Con más procesos falla con el error 1455 (archivo de paginación).
- Compilación: `.cargo/config.toml` fija la salida en `C:/vp-target` (la ruta del repositorio supera el
  límite de 260 caracteres de Windows) y compila whisper.cpp con `clang-cl`, Ninja y AVX2 fijo. Con MSVC
  la transcripción es unas 8 veces más lenta; el modo nativo podría romper el instalador en otros equipos.
- Fuera del alcance de la feature 003: edición del PDF dentro de la app, historial de presupuestos,
  agenda de clientes, firma, plantillas alternativas, envío por email o WhatsApp, y otros idiomas
  distintos del español.

## Fuente de verdad

La **única fuente de verdad** es [.specify/memory/constitution.md](.specify/memory/constitution.md).
Prevalece sobre cualquier spec, plan, tarea o decisión de código.

Los documentos de cada feature no son fuente de verdad: son artefactos de trabajo que se revisan y se
corrigen en cada ciclo con los comandos de Spec Kit pensados para ello:

- `/speckit-clarify` resuelve ambigüedades de la spec y las incorpora a ella.
- `/speckit-analyze` comprueba la coherencia entre spec, plan y tareas, y con la constitución.

Si detectas una incoherencia entre artefactos, no decidas por tu cuenta cuál prevalece: señálala y propón
pasar por `/speckit-clarify` o `/speckit-analyze`. Si un artefacto contradice la constitución, manda la constitución.

### Artefactos de la feature en curso (consulta, no fuente de verdad)

| Documento | Contenido |
|---|---|
| [specs/003-quote-pdf-export/spec.md](specs/003-quote-pdf-export/spec.md) | Requisitos (FR-xxx), casos límite y criterios de éxito |
| [specs/003-quote-pdf-export/plan.md](specs/003-quote-pdf-export/plan.md) | Stack, estructura del código y Constitution Check |
| [specs/003-quote-pdf-export/research.md](specs/003-quote-pdf-export/research.md) | Decisiones técnicas R1–R13 y mediciones |
| [specs/003-quote-pdf-export/data-model.md](specs/003-quote-pdf-export/data-model.md) | Tablas SQLite, bloqueos, avisos y estados |
| [specs/003-quote-pdf-export/contracts/ipc-commands.md](specs/003-quote-pdf-export/contracts/ipc-commands.md) | Contrato de los comandos IPC de Tauri |
| [specs/003-quote-pdf-export/contracts/pdf-document.md](specs/003-quote-pdf-export/contracts/pdf-document.md) | Contenido y maquetación del documento |
| [specs/003-quote-pdf-export/tasks.md](specs/003-quote-pdf-export/tasks.md) | Tareas T001… ordenadas por fase e historia |
| [specs/003-quote-pdf-export/quickstart.md](specs/003-quote-pdf-export/quickstart.md) | Cómo probar y validar a mano |

## Principios de la constitución (resumen)

1. **Offline-First**: toda función principal funciona sin Internet. Verifica cada feature con la red desactivada.
2. **Sin servidor propio**: sin backend, cuentas, telemetría, sincronización ni nube del proyecto. Los datos
   viven solo en el equipo del usuario.
3. **IA con degradación elegante**: transcripción local siempre disponible (whisper.cpp vía `whisper-rs`);
   una API externa es opcional y, si falla, se recurre a la local sin perder el audio. Prueba siempre el camino de fallo.
4. **Precios y marca del usuario, nunca inventados**: la IA no inventa, estima ni infiere precios. Cada importe
   sale de una tarifa o de una entrada manual. Sin tarifa → "precio pendiente". Hay que conservar la trazabilidad del precio.
5. **Revisión humana antes de emitir**: ningún PDF se genera sin confirmación explícita del usuario, y queda
   bloqueado mientras haya precios pendientes.

Las claves de API de terceros las aporta el usuario y se guardan localmente de forma protegida, nunca en el código.

## Stack (definido en plan.md)

- **Tauri 2**: núcleo en **Rust** (edición 2021) e interfaz en **React 18 + TypeScript 5 + Vite**.
- Núcleo: `tauri-plugin-dialog`, `tauri-plugin-log`, `rusqlite` (`bundled`), `rusqlite_migration`, `image`
  (solo PNG y JPEG), `unicode-normalization`, `sha2`, `serde` y `thiserror`.
- Voz: `whisper-rs` 0.16 (feature `local-transcription`), `cpal` 0.18 (micrófono), `symphonia` 0.6 (M4A, MP3
  y WAV) y `rubato` 5 (conversión a 16 kHz). Modelo `ggml-small-q5_1.bin`, incluido en el instalador.
- PDF: `krilla` 0.8 (PDF/A-2b), `rustybuzz` 0.20 (medida del texto), `time` 0.3 (fechas) y `lopdf` 0.47 (solo
  pruebas). Tipografía Liberation Sans 2.1.5 (OFL) en `src-tauri/assets/fonts/`, dentro del ejecutable.
- Interfaz: React Router 6, React Hook Form y `@tauri-apps/api` 2.
- Datos: SQLite en modo WAL dentro de `app_data_dir` (`presupuestos.db`), logo en `app_data_dir/branding/`
  y audios del dictado en `app_data_dir/recordings/`.
- Plataforma principal: Windows 10/11 x64 (WebView2).

## Estructura

```text
src/                     # Interfaz React, organizada por features
  features/business-profile/, features/rates/, features/voice-draft/, features/quote-pdf/
  shared/ipc/            # client.ts + types.ts (espejo exacto del contrato IPC)
  shared/i18n/es.ts      # TODOS los textos de la interfaz
  shared/components/, shared/styles/
src-tauri/src/           # Núcleo Rust (crate de librería `app_lib`)
  commands/              # capa fina IPC → services
  domain/                # lógica pura sin dependencias de Tauri (normalize, money, validation, draft, quote_doc…)
  pdf/                   # tipografías, logo y dibujo del documento con krilla
  audio/                 # micrófono, importación, WAV, conversión a 16 kHz y detección de silencio
  transcription/         # trait `Transcriber` y motor real whisper.cpp
  services/              # casos de uso
  storage/               # db.rs, migraciones SQL y repositorios
  error.rs               # AppError serializado según el contrato
src-tauri/tests/         # pruebas de integración Rust (DB temporal)
e2e/                     # WebdriverIO + tauri-driver
```

## Comandos

```powershell
npm install
npm run model:fetch                                     # descarga y verifica el modelo de voz (solo al preparar el entorno)
npm run tauri dev                                       # arrancar la app en desarrollo
cargo test --manifest-path src-tauri/Cargo.toml         # pruebas Rust
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features  # sin compilar whisper.cpp
npm run test                                            # Vitest + Testing Library (IPC simulado)
npm run lint; npm run typecheck
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run test:e2e                                        # e2e (requiere tauri-driver y Edge WebDriver)
npm run seed:rates -- 500                               # datos de prueba de rendimiento
```

La shell principal es PowerShell 5.1: encadena comandos con `;` (no existe `&&`).

## Convenciones y reglas de implementación

- **Dinero**: importes en **céntimos enteros** (`i64`) y porcentajes en **puntos básicos** (21 % = 2100). Nunca uses `float` para dinero.
  Precios en euros y sin IVA; un IVA general por negocio (21 % por defecto).
- **Normalización** (duplicados y búsqueda): NFD, quitar tildes, minúsculas, `trim` y colapsar espacios. La unicidad de las tarifas es `(concepto normalizado, unidad)`.
- **Lógica de negocio solo en Rust** (`src-tauri/src/domain`), sin depender de Tauri, para poder probarla de forma aislada. `commands/` solo traduce.
- **Contrato IPC**: cualquier cambio en los comandos se hace primero en `contracts/ipc-commands.md` y después en
  `src/shared/ipc/types.ts` y en Rust. Los módulos `domain`, `services`, `storage` y `error` son `pub`, porque los usan las pruebas de integración.
- Todos los comandos se registran en `src-tauri/src/lib.rs`, así que las tareas que lo tocan no son paralelas.
- **Seguridad sin red**: CSP sin orígenes remotos; capabilities limitadas a `core:default`, `dialog:allow-open` y `log:default`.
  No añadas plugins HTTP, updater, telemetría, CDNs ni fuentes remotas. Únicas excepciones de red, solo al
  preparar el repositorio: `npm run model:fetch` descarga el modelo y la tipografía se descargó una vez
  (T002 de la feature 003) y está en git. La app nunca descarga nada.
- **Precios del borrador**: el intérprete de texto es determinista y solo copia precios de una tarifa. Cualquier
  duda deja la línea pendiente. Cantidades en **centésimas enteras** (1,5 → 150).
- **Textos**: todos en español sencillo y sin jerga técnica, centralizados en `src/shared/i18n/es.ts`. Los errores de almacenamiento
  muestran un mensaje amable al usuario y el detalle va a `log::error!`.
- **Accesibilidad**: letra base de 16 px, objetivos táctiles de al menos 44 px, contraste AA y foco visible.
- **TDD**: en cada historia, las pruebas se escriben primero y deben fallar antes de implementar (tasks.md).
- Marca cada tarea completada en `tasks.md` (`- [x]`).

## Flujo de trabajo con Spec Kit

Skills disponibles en `.claude/skills/`, en orden habitual:
`/speckit-constitution` → `/speckit-specify` → `/speckit-clarify` → `/speckit-plan` → `/speckit-tasks`
→ `/speckit-analyze` → `/speckit-implement` (y `/speckit-converge` para cerrar huecos).

- Las features se numeran de forma secuencial (`NNN-nombre-corto`) en `specs/` y en una rama con el mismo nombre.
- Cada `/speckit-plan` debe superar el Constitution Check. Las excepciones se justifican por escrito en el plan.
- Cambiar la constitución exige un Sync Impact Report, versionado semver y la aprobación del responsable del proyecto.
- Pendientes en la constitución: `TODO(QUALITY_STANDARDS)` y la confirmación del nombre del producto.
