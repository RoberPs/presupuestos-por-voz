# Resultados de la validación manual: Perfil del negocio y tarifas

**Feature**: `001-business-profile-rates` | **Tarea**: T062 | **Fecha**: 2026-11-18
**Guía**: [quickstart.md](../quickstart.md)
**Entorno**: Windows 11, `npm run tauri dev`, con la base de datos creada desde cero al empezar.
**Origen de los resultados**: ejecución manual del responsable del proyecto, que informó de todos los escenarios como correctos.

## Escenarios

| # | Escenario | Requisitos | Resultado | Nota |
|---|---|---|---|---|
| 1 | Primer arranque en modo guiado | FR-001c | OK | |
| 2 | "Saltar" muestra el aviso de perfil incompleto | FR-001c, FR-008 | OK | |
| 3 | Perfil completo con datos válidos | FR-001, FR-001b | OK | |
| 4 | NIF no válido se guarda y marca el perfil incompleto | FR-001b, FR-008 | OK | |
| 5 | Rechazo de un PDF renombrado a `.png` | FR-003 | OK | |
| 6 | Rechazo de un PNG de 6 MB | FR-002, FR-003 | OK | |
| 7 | PNG transparente de 150×150 con aviso de baja resolución | FR-003b | OK | Los PNG válidos de hasta 5 MB se admiten, como pide FR-002. |
| 8 | Sustituir por JPG de 800×400 y quitar el logo | FR-006 | OK | |
| 9 | Diálogo "Guardar / Descartar" con cambios sin guardar | FR-009 | OK | |
| 10 | Datos y logo idénticos al reabrir | SC-003, FR-005 | OK | |
| 11 | Lista de tarifas vacía | FR-018 | OK | |
| 12 | Crear tarifa y ver "Precios sin IVA" | FR-010, FR-019 | OK | |
| 13 | Rechazo de precios no válidos | FR-012 | OK | |
| 14 | Error de duplicado con enlace a la tarifa existente | FR-013 | OK | |
| 15 | Mismo concepto con distinta unidad | FR-013 | OK | |
| 16 | Unidad personalizada disponible en "Nueva tarifa" | FR-011 | OK | |
| 17 | Editar el precio de una tarifa | FR-014 | OK | |
| 18 | Borrar una tarifa, cancelando y confirmando | FR-015 | OK | |
| 19 | Búsqueda sin distinguir mayúsculas ni tildes | FR-020, FR-021 | OK | |
| 20 | Búsqueda sin resultados | FR-022 | OK | |
| 21 | Tarifas y unidades conservadas al reabrir | FR-016, SC-003 | OK | |

## Comprobaciones adicionales

| Comprobación | Resultado | Nota |
|---|---|---|
| Robustez: cierre forzado durante el guardado de un logo | OK | Informado dentro de la validación completa. |
| Red desactivada durante los escenarios (SC-006) | OK | Repetidas sin red por el responsable del proyecto, que informó de que todo funciona (T060 y T062). |

## Prueba automática relacionada

`cargo test --test logo_service`: 8 pruebas superadas el 2026-11-18, incluidos el rechazo de archivos renombrados, de más de 5 MB y de PNG truncados.
