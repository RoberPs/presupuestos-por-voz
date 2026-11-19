# Research: PDF del presupuesto

**Feature**: `003-quote-pdf-export` | **Fecha**: 2026-11-18

Decisiones técnicas de la Fase 0. Las versiones se comprobaron en crates.io el 2026-11-18.

## Mediciones de la prueba aislada

Se generaron documentos reales con `krilla` 0.8.2 en un proyecto aparte, compilado en modo release y con `-j 2`. Cada línea tenía un concepto de unos 160 caracteres.

| Líneas | Páginas | Tamaño | Tiempo de generación |
|---:|---:|---:|---:|
| 2 | 1 | 29 KB | 44 ms |
| 50 | 4 | 34 KB | 192 ms |
| 200 | 13 | 46 KB | 800 ms |

Comprobado sobre esos archivos con PyMuPDF y pypdf:

- Se abren con el número de páginas esperado y llevan las tipografías incrustadas.
- Las letras `ñ Ñ á é í ó ú ü`, los signos `¿ ¡` y el símbolo `€` se extraen correctamente.
- Un PNG con transparencia se ve sobre un fondo de color sin recuadro.
- Una palabra de 160 caracteres sin espacios se reparte en varios renglones.

**Validación manual (2026-11-19)**: el responsable del proyecto completó [quickstart.md](quickstart.md) sobre la app en desarrollo. Los documentos generados se abren correctamente en Adobe Acrobat, Edge y Chrome (SC-006) y superan la validación de veraPDF con el perfil PDF/A-2b, sin errores (T061).

**Pendiente**: medir la generación de 50 líneas en la app compilada en modo release (T059, SC-002). Las cifras de la tabla son de la prueba aislada, no de la app.

---

## R1. Motor de generación del PDF

- **Decisión**: `krilla` 0.8.2, sin funciones por defecto salvo `raster-images` y `simple-text`.
- **Rationale**: es Rust puro, funciona sin red, incrusta solo los caracteres usados y ofrece validación PDF/A. Cubre SC-002 con mucho margen: 192 ms para 50 líneas frente a los 5 s exigidos.
- **Alternativas descartadas**:
  - `printpdf` 0.12.9: API de más bajo nivel para texto compuesto y sin validación equivalente.
  - `genpdf` 0.2.0: maqueta tablas, pero lleva años sin evolucionar y no controla la paginación que exige FR-017.
  - `pdf-writer` 0.15 directo: obligaría a programar a mano el subconjunto de tipografías.
  - Imprimir desde WebView2: depende del motor instalado y no garantiza un resultado idéntico entre equipos.

## R2. Compatibilidad con cualquier visor

- **Decisión**: exportar en **PDF/A-2b** con el validador de `krilla`.
- **Rationale**: el perfil obliga a incrustar las tipografías y a declarar el color, que es lo que exige FR-015. Si el documento no cumple, la exportación falla en lugar de producir un archivo dudoso. En la prueba, el validador exigió fecha del documento y desactivar la interpolación de imágenes.
- **Alternativas descartadas**: PDF sin perfil (nada detecta un incumplimiento) y PDF/UA (exige etiquetar toda la estructura, sin requisito en la spec).

## R3. Tipografía

- **Decisión**: **Liberation Sans 2.1.5**, Regular y Bold, en `src-tauri/assets/fonts/`, incrustada en el ejecutable con `include_bytes!`.
- **Rationale**: tiene licencia SIL OFL 1.1, aspecto sobrio y cubre el español. Al ir dentro del ejecutable no puede faltar en ejecución (Principio I).
- **Integridad**: los archivos se añaden a git una sola vez, sin descargas en ejecución ni al compilar.

| Archivo | SHA-256 |
|---|---|
| `LiberationSans-Regular.ttf` | `76D04C18EA243F426B7DE1F3AD208E927008F961DC5945E5AAD352D0DFDE8EE8` |
| `LiberationSans-Bold.ttf` | `788ABEE4C806D660E8AEE46689DD8540CD4BB98DA03DCC9D171CE3EFD99A9173` |

- **Licencia**: la OFL exige distribuirla con la tipografía, así que el texto se añade a los recursos del instalador en `resources/licenses/`.
- **Coste**: unos 825 KB más de ejecutable.
- **Alternativas descartadas**: Arial o Segoe UI (licencia no redistribuible) y Noto Sans (más pesada, sin ventaja para el español).

## R4. Maquetación

- **Decisión**: maquetación propia y pura en `domain/quote_doc/layout.rs`, que recibe un trait `TextMeasurer` y devuelve páginas con elementos ya colocados. `pdf/renderer.rs` solo dibuja ese resultado.
- **Rationale**: la paginación, los saltos de renglón y el "Página X de Y" se prueban sin generar un PDF. Las medidas reales usan `rustybuzz` 0.20, el mismo motor con el que `krilla` compone el texto, y cada texto se mide con su propia tipografía.
- **Alternativas descartadas**: maquetar dentro del renderizador, porque obligaría a inspeccionar PDF para probar cada regla.

## R5. Caracteres que la tipografía no contiene

- **Decisión**: si un texto contiene un carácter sin representación (p. ej. un emoji), la app **no genera el PDF** e indica el dato afectado y el carácter (FR-018b).
- **Rationale**: FR-018 exige mostrar los textos completos. Sustituir el carácter por otro alteraría un documento comercial sin avisar.

## R6. Logo

- **Decisión**: decodificar con `image` (ya en el proyecto), reducir a 640 px como máximo en el lado mayor sin ampliar nunca, y reescribir como PNG RGBA antes de incrustarlo.
- **Rationale**: unifica PNG y JPG, conserva la transparencia (FR-006) y evita documentos de 5 MB. Si el archivo no se puede leer, se aplica el caso límite "logo que ya no se puede leer".

## R7. Diálogo de guardado

- **Decisión**: abrir el diálogo **desde Rust** con `tauri-plugin-dialog` (`blocking_save_file`), igual que hace hoy el logo con `blocking_pick_file`.
- **Rationale**: los permisos de capabilities solo afectan a las llamadas desde la interfaz, así que **no hace falta añadir `dialog:allow-save`** y `capabilities/default.json` no cambia.
- **Sustitución de archivos (FR-022)**: el diálogo de guardado de Windows pide confirmación por defecto y `rfd` 0.17.2 no modifica esas opciones en el modo de guardado. Se valida a mano en [quickstart.md](quickstart.md).

## R8. Escritura segura

- **Decisión**: escribir en un temporal de nombre corto y fijo (`.presupuesto.tmp`) en la **misma carpeta**, forzar el volcado a disco y renombrarlo al nombre elegido. Ante cualquier fallo, se borra el temporal. Al ser fijo, no alarga la ruta y la siguiente generación sustituye el que deje un cierre inesperado.
- **Rationale**: el renombrado es atómico en un mismo volumen, de modo que nunca queda un archivo a medias y el anterior sigue intacto (FR-024).

| Error de Windows | Código del contrato |
|---|---|
| 5 al crear el temporal | `PDF_ACCESS_DENIED` |
| 5, 32 o 33 al renombrar | `PDF_FILE_IN_USE` |
| 39 o 112 | `PDF_DISK_FULL` |
| 3, 15, 21, 53, 67 o 1167 | `PDF_LOCATION_UNAVAILABLE` |
| Cualquier otro | `PDF_WRITE_FAILED` |

Un error 5 al renombrar indica que el destino está bloqueado o es de solo lectura, porque crear el temporal ya demostró que hay permiso en la carpeta.

## R9. Numeración

- **Decisión**: tabla `quote_number_sequences` con el último número de cada año. La propuesta es `último + 1` y solo se actualiza al guardar el PDF, con `MAX(último, usado)`.
- **Formato reconocido**: `^\d{4}-\d{3,9}$`. Cualquier otro texto se admite, pero no altera la serie (FR-013c).
- **Rationale**: es lo mínimo que exige FR-013 sin crear un historial, que está fuera de alcance.
- **Orden**: se actualiza el contador dentro de una transacción, se escribe el archivo y se confirma. Si falla la escritura, el número no se gasta (FR-013b).
- **Reemisión**: al guardar el PDF se conserva una huella de las líneas y del IVA. Si la huella coincide, se propone el mismo número; si no, el siguiente. Así, dos documentos con contenido distinto no comparten número, sin necesidad de un historial.

## R10. Datos del presupuesto al regenerar

- **Decisión**: tabla `quote_details` ligada a `quote_draft` con `ON DELETE CASCADE`. `regenerate` guarda esos datos y los restaura en la misma transacción.
- **Rationale**: `draft_repo::replace` borra y vuelve a crear la fila del borrador. La cascada vacía los datos en todo borrador nuevo (FR-012b), y regenerar el mismo borrador no debe perder el cliente ni el número.

## R11. Lo confirmado es lo que se emite

- **Decisión**: `quote_prepare` devuelve un `confirmationToken`, el SHA-256 del contenido del documento. `quote_generate` lo recalcula antes y después del diálogo, y rechaza la operación si ha cambiado.
- **Rationale**: garantiza el Principio V aunque cambie algún dato o pase la medianoche con la confirmación abierta. Usa `sha2`, que ya está en el proyecto.

## R12. Fechas

- **Decisión**: `time` 0.3 con `local-offset`, ya presente de forma indirecta. El dominio recibe la fecha como parámetro y solo el servicio consulta el reloj local.
- **Rationale**: las pruebas fijan la fecha y cubren cambios de mes y de año.

## R13. Estrategia de pruebas

| Nivel | Qué cubre |
|---|---|
| Dominio | Validación, numeración, fechas, saltos de renglón y paginación |
| Integración Rust | Bloqueos, token, numeración y escritura segura, con carpeta temporal y archivo bloqueado. Estructura del PDF con `lopdf` 0.47 como dependencia de desarrollo |
| Interfaz | Confirmación, avisos, bloqueos y errores con IPC simulado |
| e2e | Recorrido hasta la confirmación; el diálogo nativo no se puede automatizar |
| Manual | Visores reales, sustitución, disco lleno y red desactivada |
