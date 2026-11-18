# Quickstart: PDF del presupuesto

**Feature**: `003-quote-pdf-export` | Guía para validar la feature de principio a fin.

## Prerrequisitos

Los mismos que en la [feature 002](../002-voice-quote-draft/quickstart.md). Para la validación manual hacen falta además:

| Requisito | Para qué |
|---|---|
| Adobe Acrobat Reader, Edge y Chrome | Comprobar el documento en tres visores (SC-006) |
| Una memoria USB casi llena o un disco virtual pequeño | Probar la falta de espacio |
| Un logo PNG con transparencia | Comprobar el logo |

## Pruebas automáticas

```powershell
$env:CARGO_BUILD_JOBS = 2
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features
npm run test
npm run lint; npm run typecheck
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run test:e2e
```

## Datos de partida

Con `npm run tauri dev`:

1. Completar "Mi negocio" con todos los datos obligatorios y el logo PNG.
2. Crear las tarifas **Bajante** (25,00 € por metro) y **Mano de obra** (30,00 € por hora), con IVA del 21 %.
3. En "Presupuesto", escribir `cambiar bajante, 3 metros, mano de obra 2 horas`, generar el borrador y darlo por bueno.

## Escenarios

### 1. Documento correcto (Historia 1)

1. Pulsar generar PDF, escribir el cliente "Ana Ruiz" y su teléfono, y confirmar.
2. Guardar el archivo en una carpeta de prueba.

**Resultado esperado**:

- Dos líneas: 75,00 € y 60,00 €.
- Base 135,00 €, IVA (21 %) 28,35 € y total 163,35 €.
- Número `<año>-001`, fecha de hoy y validez a 30 días.
- Logo sin deformar ni recuadro, y sin etiquetas vacías en los datos del cliente.
- El documento se ve igual en Acrobat, Edge y Chrome.

### 2. Bloqueos (Historia 2)

| Situación | Resultado esperado |
|---|---|
| Línea con precio pendiente | Se enumera por su concepto y no se crea archivo |
| Línea con cantidad pendiente | Igual que el caso anterior |
| Borrador sin dar por bueno | Pide revisarlo primero |
| Perfil incompleto | Indica los datos que faltan y enlaza con "Mi negocio" |
| Cliente sin nombre | Indica que es obligatorio |
| Número vacío | Indica que hace falta un número |
| Concepto con un emoji | Indica el dato y el carácter afectados |

### 3. Guardado (Historia 3)

| Acción | Resultado esperado |
|---|---|
| Cancelar el diálogo | Sin archivo, y el siguiente intento propone el mismo número |
| Guardar sin escribir la extensión | El archivo termina en `.pdf` |
| Elegir un nombre existente y rechazar la sustitución | El archivo anterior queda intacto |
| Elegir un nombre existente y aceptar | El archivo se sustituye |
| Guardar sobre un PDF abierto en Acrobat | Avisa de que está en uso y permite reintentar |
| Guardar en `C:\Windows` | Explica que no hay permiso y permite elegir otra carpeta |
| Guardar en una memoria sin espacio | Lo indica y no deja un archivo a medias |
| Retirar la memoria USB antes de guardar | Explica que la ubicación no está disponible |

### 4. Numeración

1. Generar un PDF: número `<año>-001`.
2. Generar otro PDF del mismo borrador: propone el mismo número.
3. Corregir solo el teléfono del cliente: sigue proponiendo `<año>-001`.
4. Cambiar la cantidad de una línea, dar el borrador por bueno y generar: propone `<año>-002`.
5. Empezar un borrador nuevo: propone `<año>-003`.
6. Cambiarlo a `<año>-050` y guardar. En el siguiente borrador propone `<año>-051`.
7. Cambiarlo a `OBRA-7` y guardar. En el siguiente borrador sigue proponiendo `<año>-051`.

### 5. Datos conservados

1. Escribir los datos del cliente, cerrar la app y volver a abrirla: los datos siguen ahí.
2. Corregir el texto y regenerar el borrador: los datos del cliente y el número se conservan.
3. Empezar un borrador nuevo: los datos están vacíos.

### 6. Documentos largos

Con 200 líneas de conceptos de 160 caracteres:

- Hay varias páginas numeradas y la cabecera de la tabla se repite.
- Ninguna línea se parte ni pierde texto.
- Los totales aparecen una sola vez, al final.
- La generación tarda menos de 5 segundos.

### 7. Sin logo y logo ilegible

1. Quitar el logo: la confirmación lo avisa y el documento no deja hueco.
2. Con la app cerrada, dañar el archivo de `branding/`: la app permite seguir sin logo o ir a "Mi negocio".

### 8. Sin conexión (Historia 4)

Desactivar la red del equipo y repetir:

1. El escenario 1: el documento es equivalente.
2. El bloqueo por una línea con precio pendiente del escenario 2.
3. El guardado del escenario 3, eligiendo una carpeta y sustituyendo un archivo existente.

En ningún caso aparece un aviso por falta de red (SC-008).

## Verificación PDF/A

Validar un documento generado con veraPDF, perfil PDF/A-2b. Debe superar la validación sin errores.
