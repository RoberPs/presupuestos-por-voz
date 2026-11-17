# Quickstart: validación de "Perfil del negocio y tarifas"

**Feature**: `001-business-profile-rates` | **Fecha**: 2026-11-17

Guía para comprobar de principio a fin que la feature cumple la [spec](spec.md). Los detalles de datos y comandos están en [data-model.md](data-model.md) y [contracts/ipc-commands.md](contracts/ipc-commands.md).

## Requisitos previos (Windows 11)

- Node.js 20 LTS o superior y npm.
- Rust estable (`rustup`), con el *target* MSVC y Visual Studio Build Tools (C++).
- WebView2 Runtime (viene de serie en Windows 11).
- Para las pruebas extremo a extremo: `cargo install tauri-driver` y Microsoft Edge WebDriver de la misma versión que WebView2.

## Arranque

```powershell
npm install
npm run tauri dev
```

Para empezar desde un primer uso limpio, borra antes la carpeta de datos de la app (`%APPDATA%\<identificador-de-la-app>`).

## Pruebas automáticas

```powershell
cargo test --manifest-path src-tauri/Cargo.toml   # dominio, validadores, repositorios y comandos
npm run test                                      # Vitest + Testing Library (IPC simulado)
npm run lint; npm run typecheck
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run test:e2e                                  # WebdriverIO + tauri-driver
```

Resultado esperado: todo en verde y sin avisos de clippy ni de eslint.

## Escenarios de validación manual

Desactiva el wifi o el cable de red **antes** de empezar: todos los escenarios deben funcionar sin conexión (SC-006).

| # | Pasos | Resultado esperado | Requisitos |
|---|---|---|---|
| 1 | Primer arranque con los datos borrados | Se abre "Mi negocio" en modo guiado | FR-001c |
| 2 | Pulsa "Saltar" | Aparece el aviso permanente "Tu perfil está incompleto" con un enlace para completarlo | FR-001c, FR-008 |
| 3 | Rellena el nombre, NIF `12345678Z`, teléfono `600 123 456`, la dirección completa con CP `28001`; IVA 21 | El estado pasa a "completo" y desaparece el aviso | FR-001, FR-001b |
| 4 | Cambia el NIF a `12345678A` y guarda | Se guarda, el campo marca "NIF no válido" y el perfil vuelve a "incompleto" | FR-001b, FR-008 |
| 5 | Carga como logo un `.pdf` renombrado a `.png` | Se rechaza por formato no admitido y se conserva el logo anterior | FR-003 |
| 6 | Carga un PNG de 6 MB | Se rechaza por superar 5 MB | FR-002, FR-003 |
| 7 | Carga un PNG transparente de 150×150 | Avisa de que puede verse borroso pero se guarda; la vista previa mantiene la transparencia | FR-003b |
| 8 | Sustituye ese logo por un JPG de 800×400 y después pulsa "Quitar logo" y confirma | Primero se muestra el JPG sin recortar; después el perfil queda sin logo | FR-006 |
| 9 | Edita el teléfono sin guardar e intenta ir a "Tarifas" | Aparece el diálogo "Guardar / Descartar" | FR-009 |
| 10 | Cierra y reabre la app | Los datos y el logo están idénticos | SC-003, FR-005 |
| 11 | Abre "Tarifas" sin tarifas | Mensaje de lista vacía y botón "Crear primera tarifa" | FR-018 |
| 12 | Crea "Instalación de grifo" / unidad / `45,50` | Aparece en la lista como "45,50 €" con el aviso "Precios sin IVA" | FR-010, FR-019 |
| 13 | Intenta crear tarifas con precio vacío, `0`, `-3` o `12,345` | Se rechaza cada una con su mensaje | FR-012 |
| 14 | Crea "instalacion  de GRIFO" / unidad | Error de duplicado con enlace a la tarifa existente | FR-013 |
| 15 | Crea "Pintura" / hora y "Pintura" / m² | Se permiten las dos | FR-013 |
| 16 | Crea una tarifa con la unidad personalizada "punto de luz"; después abre "Nueva tarifa" | "punto de luz" aparece en la lista de unidades | FR-011 |
| 17 | Edita el precio de "Pintura" / hora a `30` | La lista muestra "30,00 €" | FR-014 |
| 18 | Borra "Pintura" / m², primero cancelando y luego confirmando | Al cancelar sigue en la lista; al confirmar desaparece | FR-015 |
| 19 | Busca `grifo`, `GRIFO` e `instalacion` | Solo aparecen las coincidencias, sin distinguir mayúsculas ni tildes | FR-020, FR-021 |
| 20 | Busca `zzz` | Mensaje "Sin resultados" y botón para crear una tarifa | FR-022 |
| 21 | Cierra y reabre la app | Las tarifas y las unidades personalizadas siguen ahí | FR-016, SC-003 |

## Comprobación de rendimiento (SC-005)

Carga 500 tarifas de prueba con el *seed* de desarrollo (`npm run seed:rates -- 500`). La lista filtrada debe actualizarse sin retraso perceptible (< 1 s) mientras se escribe. La prueba de integración en Rust `rates_search_500` comprueba que la consulta tarda menos de 50 ms.

## Comprobación de robustez

Durante el guardado de un logo grande, cierra la app desde el Administrador de tareas. Al reabrirla debe aparecer el logo anterior o el nuevo completo, nunca un archivo dañado, y los datos deben estar intactos.
