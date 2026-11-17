# Research: Perfil del negocio y tarifas

**Feature**: `001-business-profile-rates` | **Fecha**: 2026-11-17

Decisiones que resuelven los puntos abiertos del Technical Context de [plan.md](plan.md). Al ser la primera feature, también fijan el stack base del proyecto (TODO(TECH_STACK) de la constitución).

---

## R1. Stack de la aplicación de escritorio

- **Decision**: Tauri 2.x con núcleo en Rust (edición 2021, toolchain estable) e interfaz en React 18 + TypeScript 5 empaquetada con Vite.
- **Rationale**: instalador pequeño (unos 10 MB) y consumo de memoria bajo en equipos modestos. Usa el WebView del sistema (WebView2 en Windows). Rust ofrece bindings maduros a whisper.cpp (`whisper-rs`) para la futura transcripción local (Principio III) y generación de PDF sin servicios remotos. Toda la lógica de dominio y de datos vive en Rust, lo que facilita probarla sin interfaz. Lo eligió el responsable del proyecto el 2026-11-17.
- **Alternatives considered**: Electron + React (instalador de unos 150 MB y más memoria); Python + PySide6 (el empaquetado con PyInstaller es frágil); .NET + Avalonia (ecosistema web de UI más limitado).

## R2. Almacenamiento local

- **Decision**: SQLite embebido mediante `rusqlite` (feature `bundled`), en modo WAL, con migraciones versionadas (`rusqlite_migration`). El archivo de base de datos vive en el directorio de datos de la app que da Tauri (`app_data_dir`, en Windows `%APPDATA%\<identificador>`).
- **Rationale**: no necesita servidor (Principio II), es transaccional (un cierre inesperado no corrompe los datos ya guardados) y es un solo archivo, fácil de respaldar en la futura feature de exportación. El acceso desde Rust evita exponer SQL al WebView.
- **Alternatives considered**: `tauri-plugin-sql` desde el frontend (pone la lógica y las validaciones en JavaScript y amplía la superficie de IPC); `tauri-plugin-store` en JSON (sin unicidad ni transacciones); sled o redb (sin ventajas para un modelo relacional pequeño).

## R3. Almacenamiento y validación del logo

- **Decision**: el logo se copia al directorio de datos de la app (`app_data_dir/branding/logo.<ext>`) y la base de datos guarda su ruta relativa, formato, dimensiones y hash. Se valida en Rust con el crate `image`:
  1. Tamaño ≤ 5 MB, comprobado antes de leer el contenido.
  2. Formato detectado por los bytes iniciales (*magic bytes*), no por la extensión; solo se aceptan PNG y JPEG.
  3. Decodificación completa para descartar archivos dañados.
  4. Lado más corto < 200 px → aviso (`warning`), no error.

  La sustitución es atómica: se escribe un archivo temporal, se renombra y después se actualiza la fila en una transacción. El logo anterior se borra solo cuando todo lo demás ha terminado bien. No se recodifica la imagen: se conservan los bytes originales, sin recorte y con la transparencia del PNG intacta.
- **Rationale**: cumple FR-002, FR-003, FR-003b y FR-007, y la copia propia es independiente del archivo original. Guardarlo como archivo facilita incrustarlo en el PDF más adelante.
- **Alternatives considered**: BLOB en SQLite (infla la base de datos y complica la vista previa); guardar solo la ruta original (incumple FR-007).
- **Selección de archivo**: diálogo nativo (`tauri-plugin-dialog`) con filtro `png, jpg, jpeg`. Rust lee el archivo elegido; el WebView no accede al sistema de archivos.
- **Vista previa**: Rust devuelve el logo guardado como *data URL* en base64 (es pequeño), así no hace falta abrir el protocolo `asset:` ni el *scope* de archivos.

## R4. Dinero e IVA

- **Decision**: los precios se guardan como enteros en céntimos de euro (`i64`) y el IVA como entero en puntos básicos (2100 = 21,00 %). En la interfaz se admite coma o punto decimal (formato `es-ES`) y como máximo dos decimales; se muestra con `Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' })`.
- **Rationale**: evita errores de coma flotante en importes (crítico según el Principio IV) y valida "precio > 0" y "≤ 2 decimales" de forma exacta.
- **Alternatives considered**: `f64` (errores de redondeo); texto decimal (complica la ordenación y las comparaciones).

## R5. Normalización para duplicados y búsqueda

- **Decision**: una única función de normalización en Rust (`domain::normalize`): Unicode NFD, eliminación de marcas diacríticas, minúsculas, `trim` y colapso de espacios internos (crate `unicode-normalization`). Se guarda una columna `concept_norm`, con restricción `UNIQUE(concept_norm, unit_id)`. La búsqueda se hace en Rust con `concept_norm LIKE '%' || ?norm || '%'` (escapando `%` y `_`).
- **Rationale**: una sola fuente de verdad para FR-013 y FR-021, sin divergencias entre JavaScript y Rust. La restricción en base de datos protege la unicidad aunque falle la comprobación previa. Con 500 tarifas, la consulta local tarda milisegundos, muy por debajo de SC-005.
- **Alternatives considered**: filtrar en el frontend con `String.normalize` (duplica la lógica); SQLite FTS5 (excesivo para buscar subcadenas en una lista pequeña).
- **Ordenación (FR-017)**: `ORDER BY concept_norm, concept`, de modo que "Ábaco" quede junto a "abrazadera".

## R6. Validación de datos fiscales y de contacto

- **Decision**: validadores puros en Rust (`domain::validation`), con pruebas unitarias exhaustivas:
  - **DNI**: 8 dígitos + letra de control (`TRWAGMYFPDXBNJZSQVHLCKE`, índice = número mod 23).
  - **NIE**: X/Y/Z → 0/1/2 + 7 dígitos + letra (mismo algoritmo).
  - **CIF**: letra de organización + 7 dígitos + control (dígito o letra según el tipo), con el algoritmo oficial de sumas pares e impares.
  - **Código postal**: 5 dígitos con los dos primeros entre 01 y 52.
  - **Teléfono**: tras quitar espacios, guiones y puntos, 9 dígitos que empiezan por 6, 7, 8 o 9, con prefijo opcional `+34`/`0034`; con otro prefijo internacional, entre 6 y 15 dígitos.
  - **Email**: comprobación sintáctica básica (`local@dominio.tld`), sin verificación de entrega.

  La entrada se normaliza a mayúsculas y sin espacios ni guiones antes de validar y guardar.
- **Rationale**: FR-001b. Rust valida siempre al guardar y la interfaz muestra los errores de cada campo que devuelve el comando, sin duplicar reglas.
- **Alternatives considered**: crates de terceros para NIF (poco mantenidos; el algoritmo es corto y es mejor tenerlo probado en casa); validar solo en el frontend (se podría saltar).

## R7. Estado completo/incompleto del perfil

- **Decision**: el estado no se guarda; se calcula en cada lectura como `missing_fields`, la lista de campos obligatorios vacíos o con formato inválido. Perfil completo ⇔ `missing_fields` vacío. El comando de guardado acepta borradores (FR-008) y devuelve el perfil con su `missing_fields`.
- **Rationale**: no puede desincronizarse de los datos, y el aviso permanente de FR-001c y FR-008 se deriva directamente de él.

## R8. Interfaz, navegación y textos

- **Decision**:
  - **Navegación**: React Router 6 (en memoria). Pantallas: `Mi negocio` (con modo guiado en el primer arranque) y `Tarifas`.
  - **Formularios**: React Hook Form; los errores de campo vienen del backend y se completan en el cliente con comprobaciones triviales (obligatorio o vacío) para dar respuesta inmediata.
  - **Aviso de cambios sin guardar**: `useBlocker` de React Router y un diálogo propio (FR-009).
  - **Textos**: módulo `src/shared/i18n/es.ts` con todos los literales en español sencillo (FR-025). No hace falta librería i18n mientras haya un solo idioma.
  - **Estilos**: CSS Modules con *design tokens*: tamaño base de letra 16 px, objetivos táctiles de al menos 44 px, contraste AA y foco visible con teclado.
- **Rationale**: pocas dependencias y cobertura de los requisitos de UX de la spec para un usuario no técnico.
- **Alternatives considered**: Redux o Zustand (innecesarios: el estado del servidor es local y pequeño); TanStack Query (útil pero prescindible; se puede añadir en features posteriores).

## R9. Garantías sin red y sin servidor

- **Decision**:
  - La CSP de Tauri restringe `connect-src` a `ipc:` y `http://ipc.localhost` (sin orígenes remotos).
  - Las *capabilities* de Tauri solo conceden `dialog:allow-open` y los comandos propios.
  - Sin plugins HTTP, de telemetría ni de actualización automática.
  - Las fuentes y los recursos van empaquetados (sin CDN).
- **Rationale**: cumple los Principios I y II de forma verificable. Desactivar la red no cambia el comportamiento (SC-006).

## R10. Estrategia de pruebas

- **Decision**:
  - **Rust** (`cargo test`): pruebas unitarias de validadores, normalización y dinero; pruebas de repositorios contra SQLite en memoria o en un directorio temporal (`tempfile`); pruebas de comandos invocando las funciones de servicio sin WebView.
  - **Frontend**: Vitest + React Testing Library + `@testing-library/user-event`, con la capa IPC simulada mediante `@tauri-apps/api/mocks` (`mockIPC`).
  - **Extremo a extremo**: WebdriverIO + `tauri-driver` en Windows para los recorridos de [quickstart.md](quickstart.md), incluido uno con la red desactivada.
  - **Calidad**: `cargo clippy -D warnings`, `cargo fmt --check`, `eslint`, `tsc --noEmit`.
- **Rationale**: pirámide de pruebas con la mayor parte en el dominio de Rust, que es donde están las reglas de negocio de esta feature.
