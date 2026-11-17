# CLAUDE.md

Guía para Claude Code al trabajar en este repositorio.

## Qué es este proyecto

**Presupuestos por Voz** (nombre de trabajo): aplicación de escritorio para profesionales de oficios
(fontanería, electricidad, pintura…) que genera presupuestos en PDF a partir de la visita al cliente
dictada por voz. Un solo usuario por instalación, sin conocimientos técnicos.

## Estado actual

- El proyecto se desarrolla con **Spec Kit** (Spec-Driven Development, v0.16.5, scripts PowerShell).
- **Todavía no hay código fuente.** Solo existen la constitución y los artefactos de la primera feature.
- Feature en curso: `001-business-profile-rates` (perfil del negocio, logo y tarifas), en la rama del
  mismo nombre. Spec, plan y tareas ya están hechos; el siguiente paso es `/speckit-implement`.
- Fuera del alcance de la feature 001: dictado por voz, presupuestos y PDF, importar desde Excel,
  varios negocios o usuarios, exportación y copias de seguridad.

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
| [specs/001-business-profile-rates/spec.md](specs/001-business-profile-rates/spec.md) | Requisitos (FR-xxx), casos límite y criterios de éxito |
| [specs/001-business-profile-rates/plan.md](specs/001-business-profile-rates/plan.md) | Stack, estructura del código y Constitution Check |
| [specs/001-business-profile-rates/research.md](specs/001-business-profile-rates/research.md) | Decisiones técnicas R1–R10 |
| [specs/001-business-profile-rates/data-model.md](specs/001-business-profile-rates/data-model.md) | Tablas SQLite, validaciones y estado derivado |
| [specs/001-business-profile-rates/contracts/ipc-commands.md](specs/001-business-profile-rates/contracts/ipc-commands.md) | Contrato de los comandos IPC de Tauri |
| [specs/001-business-profile-rates/tasks.md](specs/001-business-profile-rates/tasks.md) | Tareas T001… ordenadas por fase e historia |
| [specs/001-business-profile-rates/quickstart.md](specs/001-business-profile-rates/quickstart.md) | Cómo arrancar, probar y validar a mano |

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
- Interfaz: React Router 6, React Hook Form y `@tauri-apps/api` 2.
- Datos: SQLite en modo WAL dentro de `app_data_dir` (`presupuestos.db`) y logo en `app_data_dir/branding/`.
- Plataforma principal: Windows 10/11 x64 (WebView2).

## Estructura prevista

```text
src/                     # Interfaz React, organizada por features
  features/business-profile/, features/rates/
  shared/ipc/            # client.ts + types.ts (espejo exacto del contrato IPC)
  shared/i18n/es.ts      # TODOS los textos de la interfaz
  shared/components/, shared/styles/
src-tauri/src/           # Núcleo Rust (crate de librería `app_lib`)
  commands/              # capa fina IPC → services
  domain/                # lógica pura sin dependencias de Tauri (normalize, money, validation…)
  services/              # casos de uso
  storage/               # db.rs, migraciones SQL y repositorios
  error.rs               # AppError serializado según el contrato
src-tauri/tests/         # pruebas de integración Rust (DB temporal)
e2e/                     # WebdriverIO + tauri-driver
```

## Comandos (previstos; disponibles cuando exista el esqueleto de T001–T003)

```powershell
npm install
npm run tauri dev                                       # arrancar la app en desarrollo
cargo test --manifest-path src-tauri/Cargo.toml         # pruebas Rust
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
  No añadas plugins HTTP, updater, telemetría, CDNs ni fuentes remotas.
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
