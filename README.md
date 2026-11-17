# Presupuestos por Voz

Aplicación de escritorio para profesionales de oficios (fontaneros, electricistas, pintores…) que
convierte la visita al cliente, dictada por voz, en un presupuesto en PDF con **su logo, sus datos
y sus tarifas**.

> **Estado:** primera feature implementada (*Perfil del negocio y tarifas*): datos del negocio,
> logo, IVA general y lista de tarifas con búsqueda. El dictado por voz y los presupuestos en PDF
> llegarán en features posteriores.

## Por qué

- **Funciona sin Internet.** Pensada para obras, sótanos y domicilios con mala cobertura.
- **Tus datos son tuyos.** No hay servidor, cuentas, nube ni telemetría: todo se guarda en tu equipo.
- **Los precios los pones tú.** La IA nunca inventa importes. Lo que no tiene tarifa queda marcado como "precio pendiente".
- **Tú tienes la última palabra.** Ningún presupuesto se genera sin que lo revises y lo confirmes.
- **Voz con respaldo local.** La transcripción funciona siempre con un motor local; un servicio externo es opcional.

Estos principios están recogidos en la [constitución del proyecto](.specify/memory/constitution.md).

## Hoja de ruta

| # | Feature | Estado |
|---|---|---|
| 001 | [Perfil del negocio y tarifas](specs/001-business-profile-rates/spec.md): datos fiscales, logo, IVA general, lista de tarifas con búsqueda | Implementada |
| — | Dictado por voz y transcripción local | Pendiente |
| — | Presupuestos y generación de PDF | Pendiente |
| — | Exportación y copia de seguridad de los datos | Pendiente |

## Tecnología

- [Tauri 2](https://tauri.app/): núcleo en **Rust** e interfaz en **React + TypeScript** (Vite).
- **SQLite** local (modo WAL) para los datos y una copia local del logo.
- Pruebas: `cargo test`, Vitest + Testing Library y WebdriverIO + `tauri-driver` para e2e.
- Plataforma principal: Windows 10/11 x64. macOS y Linux son viables con el mismo código.

## Desarrollo

### Requisitos

- Node.js 20 LTS o superior y npm.
- Rust estable (`rustup`) y los [requisitos previos de Tauri](https://tauri.app/start/prerequisites/): en Windows, WebView2 y Visual Studio Build Tools con la carga de trabajo de C++.
- Para e2e: `cargo install tauri-driver` y [Microsoft Edge WebDriver](https://developer.microsoft.com/microsoft-edge/tools/webdriver/) de la misma versión que WebView2, accesible en el `PATH`.

### Comandos

```powershell
npm install
npm run tauri dev                                         # arrancar en desarrollo
cargo test --manifest-path src-tauri/Cargo.toml           # pruebas del núcleo Rust
npm run test                                              # pruebas de la interfaz
npm run lint; npm run typecheck
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run test:e2e                                          # pruebas extremo a extremo
npm run seed:rates -- 500                                 # 500 tarifas de prueba (con la app cerrada)
```

Si la compilación de Rust falla con *"El archivo de paginación es demasiado pequeño"* (error 1455),
el equipo se queda sin memoria al compilar en paralelo: limita los procesos con
`$env:CARGO_BUILD_JOBS = 2` antes de lanzar el comando.

La guía completa de validación está en [quickstart.md](specs/001-business-profile-rates/quickstart.md).

### Dónde se guardan los datos

Todo queda en el equipo, en la carpeta de datos de la aplicación (en Windows,
`%APPDATA%\es.presupuestosvoz.app`):

```text
presupuestos.db        # base de datos SQLite: perfil, unidades y tarifas
branding/logo-*.png    # copia del logo (o .jpg)
```

Para hacer una copia de seguridad basta con copiar esa carpeta con la app cerrada. Los registros
técnicos están en `%LOCALAPPDATA%\es.presupuestosvoz.app\logs`. En builds de depuración se puede
usar otra carpeta con la variable `PRESUPUESTOS_DATA_DIR`.

## Metodología: Spec-Driven Development

El proyecto se construye con [Spec Kit](https://github.com/github/spec-kit). Cada feature recorre:

1. **Specify**: qué y por qué, sin detalles técnicos (`spec.md`).
2. **Clarify**: resolver ambigüedades.
3. **Plan**: stack, diseño, modelo de datos y contratos (`plan.md`, `research.md`, `data-model.md`, `contracts/`).
4. **Tasks**: tareas ordenadas por dependencias (`tasks.md`).
5. **Implement**: implementación guiada por pruebas.

Los artefactos de cada feature están en [`specs/`](specs/). Las instrucciones para el asistente de IA están en [CLAUDE.md](CLAUDE.md).

## Estructura del repositorio

```text
.specify/        # Configuración de Spec Kit, constitución, plantillas y scripts
.claude/skills/  # Comandos /speckit-* para Claude Code
specs/           # Especificaciones, planes y tareas por feature
src/             # interfaz React + TypeScript, organizada por features
src-tauri/       # núcleo Rust: dominio, servicios, almacenamiento y comandos IPC
e2e/             # pruebas extremo a extremo (WebdriverIO + tauri-driver)
```
