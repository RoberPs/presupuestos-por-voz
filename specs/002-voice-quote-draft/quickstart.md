# Quickstart: Borrador de presupuesto por voz

**Feature**: `002-voice-quote-draft` | Guía para preparar el entorno y validar la feature de principio a fin.

## Prerrequisitos

Además de lo necesario para la feature 001:

| Requisito | Comprobación | Nota |
|---|---|---|
| CMake | `cmake --version` | Necesario para compilar whisper.cpp |
| LLVM/Clang | `clang --version` | Definir `LIBCLANG_PATH` si no se detecta. whisper.cpp se compila con `clang-cl`: con MSVC es unas 8 veces más lento |
| Ninja | `ninja --version` | Generador que usa CMake con `clang-cl` (ver `.cargo/config.toml`) |
| Modelo de voz | `npm run model:fetch` | Se descarga una vez, verifica su hash y queda fuera de git |
| Micrófono | Configuración de Windows > Privacidad > Micrófono | Solo para la validación manual |

```powershell
winget install Kitware.CMake
winget install LLVM.LLVM
winget install Ninja-build.Ninja
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
$env:CARGO_BUILD_JOBS = 2
npm install
npm run model:fetch
```

## Pruebas automáticas

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features   # dominio y servicios, sin whisper.cpp
cargo test --manifest-path src-tauri/Cargo.toml                         # todo, con el motor real
npm run test
npm run lint; npm run typecheck
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run test:e2e
```

## Datos de partida

Con `npm run tauri dev`, crear las tarifas **Bajante** (25,00 € por metro) y **Mano de obra** (30,00 € por hora), con IVA general del 21 %.

## Validación manual

| # | Escenario | Resultado esperado | Cubre |
|---|---|---|---|
| 1 | Pulsar grabar (o F9), dictar "cambiar bajante, 3 metros, mano de obra 2 horas" y parar | Bajante 3 metros 75,00 €; Mano de obra 2 horas 60,00 €; base 135,00 €; IVA 28,35 €; total 163,35 € | US1 |
| 2 | Cambiar la cantidad de bajante a 4 | Importe 100,00 €; base 160,00 €; IVA 33,60 €; total 193,60 €, al momento | US2 |
| 3 | Añadir una línea desde una tarifa y otra a mano, y borrar una | Los totales se actualizan y la línea manual consta como entrada manual | US2 |
| 4 | Desactivar la red y repetir el escenario 1 | Mismo borrador, sin avisos de red | US3 |
| 5 | Cambiar "3 metros" por "5 metros" en el texto y regenerar | Aviso si hay cambios manuales; línea de 5 metros, 125,00 € | US4 |
| 6 | Pegar el texto del ejemplo en un borrador nuevo | Mismas líneas y totales que en el escenario 1 | US5 |
| 7 | Importar una nota de voz M4A de iPhone y otra de Android | Mismo borrador que al dictar | FR-002b |
| 8 | Importar un `.txt` renombrado a `.mp3` y un audio de 11 minutos | Rechazo con el motivo; el borrador en curso sigue intacto | FR-002c |
| 9 | Dictar "cambiar grifo, 2 unidades" sin esa tarifa | Línea con "precio pendiente", fuera del total, y total provisional | FR-018 |
| 10 | Dictar "3 metros, no, mejor 4 de bajante" | "Cantidad pendiente" con el fragmento visible | FR-020 |
| 11 | Intentar dar por bueno con pendientes y después sin ellas | Primero se señalan las líneas; después queda como revisado | FR-034 |
| 12 | Grabar solo silencio | Aviso de que no se ha oído nada, sin borrador nuevo | FR-006 |
| 13 | Denegar el micrófono en Windows y pulsar grabar | Explicación clara y opciones de importar o escribir | FR-008 |
| 14 | Desconectar el micrófono mientras se graba | Se conserva lo grabado y se ofrece procesarlo | Casos límite |
| 15 | Renombrar el modelo y procesar un audio | Error claro, audio conservado y reintento posible al restaurarlo | Principio III |
| 16 | Cerrar la app a la fuerza mientras se graba y reabrirla | Se ofrece recuperar la grabación | SC-008 |
| 17 | Cambiar el IVA del negocio con un borrador abierto | El total no cambia; aviso con opción de actualizar | FR-029b |
| 18 | Borrar todas las tarifas y dictar el ejemplo | Todas las líneas pendientes y acceso a crear tarifas | FR-023 |

## Mediciones

| Medición | Objetivo | Cómo |
|---|---|---|
| Velocidad del modelo | Dictado de 1 minuto en < 60 s con 4 núcleos y 8 GB | Cronometrar desde parar hasta ver el borrador. **Hacerlo antes de construir el resto** ([research.md](research.md), R2) |
| Precisión | ≥ 90 % de conceptos, cantidades y unidades correctos | Al menos 20 dictados reales con ruido de obra y de coche (SC-003) |
| Tiempo total | Borrador de 5 líneas revisado en < 5 minutos | Prueba con un profesional (SC-001) |
