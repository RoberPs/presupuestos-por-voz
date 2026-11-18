# Research: Borrador de presupuesto por voz

**Feature**: `002-voice-quote-draft` | **Fecha**: 2026-11-18

Las versiones se comprobaron en crates.io y el tamaño y el hash de los modelos en el repositorio público de whisper.cpp, el 2026-11-18.

## R1. Motor de transcripción local

- **Decisión**: whisper.cpp mediante `whisper-rs` 0.16, en un hilo dedicado, con idioma fijo `es`, muestreo voraz (*greedy*) y hasta 4 hilos de CPU. El modelo se carga al empezar a transcribir y se libera al terminar.
- **Motivo**: es el motor previsto en CLAUDE.md, cumple el motor local que exige el Principio III, funciona sin red y admite progreso y cancelación.
- **Prerrequisito de compilación**: `whisper-rs` necesita **CMake** y **LLVM/Clang**. En el equipo de desarrollo actual no están instalados (ver [quickstart.md](quickstart.md)).
- **Alternativas descartadas**: Vosk (peor precisión en español con ruido); API externa (la constitución la permite solo como mejora opcional, y queda fuera de esta feature).

## R2. Modelo incluido

- **Decisión**: `ggml-small-q5_1.bin`, 181 MiB, SHA-256 `ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb`, como recurso del instalador. `scripts/fetch-model.mjs` lo descarga al preparar el entorno de desarrollo y verifica el hash; la app nunca descarga nada.
- **Motivo**: el dictado tiene ruido de obra y de coche, y `small` ofrece mejor precisión que `base` con un tamaño asumible una vez cuantizado.
- **Riesgo abierto**: no se ha medido aún si cumple SC-002 (1 minuto de audio en < 60 s con 4 núcleos y 8 GB). **La primera tarea de implementación debe medirlo.** Si no lo cumple, se usa `ggml-base-q5_1.bin` (57 MiB, SHA-256 `422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898`) y se comprueba SC-003 con él.
- **Alternativas descartadas**: `medium-q5_0` (514 MiB, demasiado lento para SC-002); descarga en el primer arranque (incumple el Principio I).

## R3. Captura del micrófono

- **Decisión**: `cpal` 0.18 en el núcleo Rust, con el flujo de audio en un hilo propio, porque no se puede mover entre hilos. Se convierte a mono y 16 kHz mientras se graba.
- **Motivo**: no requiere abrir permisos de medios en el WebView, mantiene la CSP actual y permite seguir grabando en disco aunque la interfaz se bloquee.
- **Errores**: sin dispositivo → `MIC_NOT_FOUND`; acceso denegado por la privacidad de Windows → `MIC_ACCESS_DENIED`, con la indicación de abrir Configuración > Privacidad > Micrófono; ocupado u otro fallo → `MIC_UNAVAILABLE`; desconexión durante la grabación → se detiene y se conserva lo grabado.
- **Alternativa descartada**: `getUserMedia` en el WebView (depende de permisos de WebView2 y pierde el audio si la interfaz falla).

## R4. Importación y normalización de audio

- **Decisión**: `symphonia` 0.6 con `isomp4`, `aac`, `alac`, `mp3`, `wav` y `pcm`, y `rubato` 5 para pasar a 16 kHz. La decodificación es en flujo y se detiene en cuanto supera los 10 minutos, así un archivo enorme no agota la memoria.
- **Riesgo conocido**: `symphonia` decodifica AAC-LC, que es el habitual en notas de voz, pero no HE-AAC. Esos archivos se rechazan como formato no admitido. Se validará con audios reales de iPhone y Android.
- **Alternativa descartada**: incluir FFmpeg (aumenta el instalador y complica las licencias).

## R5. Almacenamiento del audio y recuperación

- **Decisión**: la grabación se escribe en `recordings/incoming.wav.part` con volcado a disco cada segundo. Al arrancar, si existe ese archivo, se repara la cabecera a partir de su tamaño y se ofrece recuperarlo. Se pierde como máximo un segundo.
- **Sustitución segura**: el audio nuevo ocupa una ranura `incoming` y el borrador anterior sigue intacto hasta que el nuevo se genera, en una única transacción. Si la transcripción falla, se cancela o no hay voz, el borrador anterior no se pierde (SC-008).
- **Alternativa descartada**: guardar el audio en SQLite (engorda la base de datos y dificulta la escritura incremental).

## R6. Interpretación del texto

- **Decisión**: intérprete **determinista por reglas** en `domain/draft`, sin modelo generativo.
- **Motivo**: FR-014 y SC-007 exigen el mismo resultado para el mismo texto, y el Principio IV prohíbe que una IA influya en los precios. Además es rápido y se prueba por completo con tablas de casos.
- **Reglas**:

| Paso | Comportamiento |
|---|---|
| Segmentación | Corta por puntos, comas, saltos de línea y por el final de cada grupo cantidad + unidad |
| Patrones | `concepto cantidad unidad`, `cantidad unidad de concepto`, `cantidad concepto`; un segmento con solo cantidad y unidad se une al concepto anterior si este no tiene cantidad |
| Cantidades | Cifras, palabras hasta miles, `medio`, `metro y medio`, `dos horas y media`, `media hora`, `cuarto de hora`, `un par de` |
| Cantidad ambigua | `unos`, `unas`, `algunos`, `varios`, `unos cuantos`, `más o menos`, `aproximadamente` → `cantidad pendiente` |
| Rectificación | `no, mejor`, `perdón`, `digo`, `mejor dicho`, `o sea` junto a una cantidad → `cantidad pendiente` con el fragmento |
| Sin interpretar | Lo que no forma un trabajo se guarda como fragmento sin interpretar (FR-022) |

- **Alternativa descartada**: un modelo de lenguaje local (no determinista, pesado y con riesgo de inventar datos).

## R7. Coincidencia con las tarifas

- **Decisión**: se aplica la regla aclarada en FR-017, palabra por palabra sobre el texto normalizado de la feature 001.

| Situación | Resultado |
|---|---|
| Todas las palabras de una única tarifa aparecen en el fragmento, y la unidad es compatible o no se ha dictado | Se aplica la tarifa |
| Encajan varias tarifas y la unidad dictada deja solo una | Se aplica esa tarifa |
| Encajan varias tarifas sin que la unidad decida | Pendiente, con esas tarifas para elegir (FR-019) |
| Encaja por palabras, pero la unidad dictada es distinta | Pendiente, con la tarifa como sugerencia (FR-021) |
| Comparte alguna palabra significativa o una palabra difiere en 1 o 2 letras | Pendiente, con hasta 5 sugerencias sin precio (FR-017b) |
| Nada parecido | Pendiente, sin sugerencias (FR-018) |

- **Singular y plural**: dos palabras coinciden si son iguales o si una es la otra más `s` o `es`, además de `z` ↔ `ces`. Todas las palabras cuentan, también `de`.
- **Orden estable**: las sugerencias se ordenan por parecido, concepto normalizado e identificador.

## R8. Unidades

- **Decisión**: tabla de alias para las semillas: `metro`/`metros`/`m`; `hora`/`horas`/`h`; `unidad`/`unidades`/`ud`/`uds`; `m²`/`m2`/`metro cuadrado`/`metros cuadrados`; `metro lineal`/`metros lineales`/`ml`. Las unidades personalizadas se reconocen por su texto normalizado, en singular o plural. Nunca se convierten unidades entre sí.

## R9. Dinero y cantidades

- **Decisión**: cantidad en centésimas enteras (1,5 → 150). Importe = `(cantidad × precio + 50) / 100`, e IVA = `(base × puntos básicos + 5000) / 10000`, ambos con redondeo al céntimo más cercano y cálculo intermedio en `i128`. Un resultado que no quepa en `i64` se rechaza con `AMOUNT_TOO_LARGE`.
- **Comprobación**: 300 × 2500 → 7500; 200 × 3000 → 6000; base 13500 al 21 % → 2835, igual que la spec.

## R10. Silencio y calidad baja

- **Decisión**: antes de cargar el modelo se analiza la energía en ventanas de 20 ms; con menos de 300 ms de voz se considera silencio (FR-006). También se considera sin voz un resultado vacío, y se descartan los segmentos con probabilidad de no-voz superior a 0,6, para evitar textos inventados por el motor con ruido. La calidad es baja si la probabilidad media de las palabras es < 0,5 (FR-007).
- **Pendiente**: estos umbrales son iniciales y se ajustarán con el conjunto de audios reales de SC-003.

## R11. Procesos largos y cancelación

- **Decisión**: los comandos de transcripción se ejecutan fuera del hilo principal, informan del progreso mediante `tauri::ipc::Channel` y se cancelan con una señal atómica. La conexión SQLite no se bloquea mientras se transcribe.
- **Vocabulario**: los conceptos de las tarifas se pasan como contexto inicial del motor, hasta unos 600 caracteres, para mejorar el reconocimiento de términos del oficio. No afecta a los precios.

## R12. Gesto de grabación

- **Decisión**: un botón grande (mínimo 96 px) y la tecla **F9** dentro de la app, para empezar y parar.
- **Motivo**: F9 no interfiere al escribir, a diferencia de la barra espaciadora. Un atajo global exigiría un plugin y un permiso nuevos, que las reglas del proyecto no permiten.
