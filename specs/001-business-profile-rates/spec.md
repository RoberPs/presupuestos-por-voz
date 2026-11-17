# Feature Specification: Perfil del negocio y tarifas

**Feature Branch**: `001-business-profile-rates`

**Created**: 2026-11-17

**Status**: Draft

**Input**: User description: "Un profesional de oficio (fontanero, electricista, pintor) configura una sola vez los datos de su negocio y sus tarifas, para que los presupuestos salgan con su imagen y sus precios. Usuario: el propio profesional, único usuario de la app, sin conocimientos técnicos. Historias: guardar datos del negocio y logo (P1); crear, editar y borrar tarifas con concepto, unidad y precio (P1); buscar tarifas por nombre (P2); datos y logo modificables cuando el dueño quiera. Casos límite: logo no válido o demasiado grande, tarifa sin precio o con precio ≤ 0, conceptos duplicados, primer uso sin tarifas, datos del negocio incompletos. Fuera de alcance: dictado por voz, generación de presupuestos y PDF, importar tarifas desde Excel, varios negocios o usuarios."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Guardar los datos del negocio y el logo (Priority: P1)

El profesional abre la sección "Mi negocio", rellena los datos de su empresa (nombre, identificación fiscal, contacto, dirección…) y elige una imagen de su equipo como logo. Al guardar, ve una vista previa de cómo quedan sus datos y su logo. Cuando cierra y vuelve a abrir la app, todo sigue tal como lo dejó.

**Why this priority**: sin los datos y el logo del profesional, ningún presupuesto futuro puede llevar su imagen. Es la base de la identidad de todos los documentos (Principio IV de la constitución).

**Independent Test**: se rellena el formulario del negocio, se carga un logo válido, se guarda, se cierra la app y se reabre; los datos y el logo aparecen idénticos.

**Acceptance Scenarios**:

1. **Given** una instalación nueva sin datos del negocio, **When** el profesional rellena todos los datos obligatorios, carga un logo válido y pulsa "Guardar", **Then** la app confirma el guardado y muestra los datos y el logo en la vista previa.
2. **Given** datos del negocio y logo guardados, **When** el profesional cierra y vuelve a abrir la app, **Then** los datos y el logo se muestran exactamente como se guardaron.
3. **Given** el equipo sin conexión a Internet, **When** el profesional guarda o consulta sus datos y su logo, **Then** todo funciona igual que con conexión.

---

### User Story 2 - Gestionar la lista de tarifas (Priority: P1)

El profesional mantiene su lista de precios: crea tarifas indicando concepto (p. ej. "Instalación de grifo"), unidad (metro, hora, unidad…) y precio; corrige el precio o el texto de una tarifa existente; y borra las que ya no usa.

**Why this priority**: los importes de los presupuestos solo pueden salir de tarifas configuradas por el propio profesional; sin ellas la app no puede calcular nada (Principio IV).

**Independent Test**: se crea una tarifa y aparece en la lista; se edita su precio y la lista muestra el precio nuevo; se borra y desaparece de la lista; tras reabrir la app los cambios se mantienen.

**Acceptance Scenarios**:

1. **Given** la lista de tarifas, **When** el profesional crea una tarifa con concepto, unidad y precio válidos, **Then** la tarifa nueva aparece en la lista.
2. **Given** una tarifa existente, **When** el profesional cambia su precio y guarda, **Then** la lista muestra el precio nuevo.
3. **Given** una tarifa existente, **When** el profesional la borra y confirma el borrado, **Then** la tarifa desaparece de la lista.
4. **Given** una tarifa existente, **When** el profesional pulsa borrar pero cancela la confirmación, **Then** la tarifa sigue en la lista sin cambios.
5. **Given** cambios en las tarifas, **When** el profesional cierra y reabre la app, **Then** la lista refleja todos los cambios realizados.
6. **Given** el primer uso de la app sin tarifas, **When** el profesional abre la sección de tarifas, **Then** ve un mensaje que explica que aún no tiene tarifas y un acceso directo para crear la primera.

---

### User Story 3 - Modificar o sustituir los datos y el logo (Priority: P2)

Con el tiempo el negocio cambia (nuevo teléfono, nueva dirección, logo renovado). El profesional puede volver a "Mi negocio" en cualquier momento, cambiar cualquier dato, sustituir el logo por otro o quitarlo, y guardar.

**Why this priority**: los datos se configuran "una vez", pero deben poder actualizarse sin ayuda técnica; es necesario para que los presupuestos sigan siendo correctos, aunque no bloquea el primer uso.

**Independent Test**: con datos y logo ya guardados, se cambia un dato y se sustituye el logo por otro; tras reabrir la app aparecen los valores nuevos y el logo nuevo, sin rastro del anterior.

**Acceptance Scenarios**:

1. **Given** datos del negocio guardados, **When** el profesional modifica un dato y guarda, **Then** se muestra el valor nuevo y se conserva tras reabrir la app.
2. **Given** un logo guardado, **When** el profesional carga otro logo válido y guarda, **Then** el nuevo logo sustituye al anterior.
3. **Given** un logo guardado, **When** el profesional elige quitar el logo y confirma, **Then** el negocio queda sin logo y la app lo indica claramente.
4. **Given** cambios sin guardar en "Mi negocio", **When** el profesional intenta salir de la sección, **Then** la app le avisa y le permite guardar o descartar los cambios.

---

### User Story 4 - Buscar una tarifa por nombre (Priority: P2)

Con una lista larga de tarifas, el profesional escribe parte del concepto en un buscador y la lista se reduce a las tarifas que coinciden.

**Why this priority**: ahorra tiempo cuando hay muchas tarifas, pero la gestión de tarifas funciona sin ella.

**Independent Test**: con varias tarifas creadas, se escribe un fragmento del concepto y solo se muestran las coincidentes; al vaciar el buscador vuelve la lista completa.

**Acceptance Scenarios**:

1. **Given** las tarifas "Instalación de grifo", "Cambio de grifo" y "Pintura de pared", **When** el profesional escribe "grifo", **Then** solo se muestran las dos tarifas de grifo.
2. **Given** una búsqueda escrita en minúsculas y sin tildes (p. ej. "instalacion"), **When** existe la tarifa "Instalación de grifo", **Then** esa tarifa aparece en los resultados.
3. **Given** un texto que no coincide con ninguna tarifa, **When** el profesional lo escribe, **Then** la app indica que no hay resultados y ofrece crear una tarifa nueva.
4. **Given** una búsqueda activa, **When** el profesional borra el texto del buscador, **Then** se vuelve a mostrar la lista completa.

---

### Edge Cases

- **Logo en formato no válido**: si el archivo no es una imagen en un formato admitido (PNG o JPG), la app lo rechaza con un mensaje claro que indica los formatos admitidos y conserva el logo anterior, si lo había.
- **Logo demasiado grande**: si el archivo supera el tamaño máximo (5 MB), la app lo rechaza con un mensaje que indica el límite y conserva el logo anterior.
- **Archivo de logo dañado o ilegible**: se trata como formato no válido.
- **Tarifa sin precio**: no se puede guardar; la app señala el campo precio como obligatorio.
- **Precio negativo o cero**: no se puede guardar; la app indica que el precio debe ser mayor que cero.
- **Precio con formato incorrecto** (letras, más de dos decimales): no se puede guardar; la app indica el formato esperado.
- **Tarifa sin concepto o sin unidad**: no se puede guardar; la app señala los campos que faltan.
- **Dos tarifas con el mismo concepto**: si ya existe una tarifa con el mismo concepto y la misma unidad (sin distinguir mayúsculas, tildes ni espacios sobrantes), la app impide guardarla e indica cuál es la tarifa existente para que el profesional la edite. El mismo concepto con distinta unidad (p. ej. "Pintura" por metro y por hora) sí se permite.
- **Primer uso sin tarifas**: la lista vacía muestra un mensaje explicativo y un acceso directo para crear la primera tarifa.
- **Datos del negocio incompletos**: la app permite guardar un borrador incompleto para no perder lo escrito, pero marca el perfil como "incompleto", señala los campos obligatorios que faltan y muestra un aviso visible hasta completarlo.
- **Cierre inesperado de la app durante la edición**: los datos ya guardados no se corrompen ni se pierden.

## Requirements *(mandatory)*

### Functional Requirements

**Datos del negocio y logo**

- **FR-001**: El sistema MUST permitir al profesional introducir y guardar los datos de su negocio. Campos obligatorios: nombre comercial, NIF/CIF, teléfono y dirección. Campos opcionales: razón social, email y web.
- **FR-002**: El sistema MUST permitir cargar un logo desde un archivo del equipo del usuario, aceptando solo formatos PNG y JPG de hasta 5 MB.
- **FR-003**: El sistema MUST rechazar logos con formato no admitido, dañados o que superen el tamaño máximo, mostrando un mensaje comprensible y conservando el logo anterior.
- **FR-004**: El sistema MUST mostrar una vista previa del logo y de los datos del negocio antes y después de guardar.
- **FR-005**: El sistema MUST conservar los datos del negocio y el logo entre sesiones, de forma que al reabrir la app se muestren exactamente como se guardaron.
- **FR-006**: El profesional MUST poder modificar cualquier dato del negocio, sustituir el logo por otro o quitarlo en cualquier momento.
- **FR-007**: El sistema MUST guardar una copia propia del logo, de forma que siga disponible aunque el archivo original se mueva o borre del equipo.
- **FR-008**: El sistema MUST permitir guardar datos incompletos, marcando el perfil como "incompleto", indicando qué campos obligatorios faltan y mostrando un aviso visible hasta que se completen.
- **FR-009**: El sistema MUST avisar al salir de "Mi negocio" con cambios sin guardar y ofrecer guardar o descartar.

**Tarifas**

- **FR-010**: El profesional MUST poder crear tarifas con concepto (texto obligatorio), unidad (obligatoria) y precio (obligatorio).
- **FR-011**: El sistema MUST ofrecer una lista de unidades predefinidas (al menos: unidad, hora, metro, metro cuadrado, metro lineal) y permitir escribir una unidad personalizada.
- **FR-012**: El sistema MUST rechazar tarifas sin precio, con precio igual a cero, negativo, no numérico o con más de dos decimales, indicando el motivo.
- **FR-013**: El sistema MUST impedir guardar una tarifa con el mismo concepto y la misma unidad que otra existente (comparación sin distinguir mayúsculas, tildes ni espacios sobrantes) e indicar cuál es la tarifa existente.
- **FR-014**: El profesional MUST poder editar el concepto, la unidad y el precio de una tarifa existente, aplicando las mismas validaciones que al crearla.
- **FR-015**: El profesional MUST poder borrar una tarifa, previa confirmación explícita.
- **FR-016**: El sistema MUST conservar las tarifas entre sesiones.
- **FR-017**: El sistema MUST mostrar las tarifas ordenadas alfabéticamente por concepto, con su unidad y su precio visibles.
- **FR-018**: Con la lista de tarifas vacía, el sistema MUST mostrar un mensaje explicativo y un acceso directo para crear la primera tarifa.
- **FR-019**: Los precios de las tarifas MUST expresarse en euros (€) y sin IVA. El perfil del negocio MUST incluir un tipo de IVA general, configurable por el profesional en "Mi negocio", con valor por defecto del 21 %, que se aplicará en los futuros presupuestos. La lista de tarifas MUST indicar visiblemente que los precios no incluyen IVA.
- **FR-019b**: El tipo de IVA MUST aceptar valores entre 0 % y 100 % con hasta dos decimales; un valor fuera de ese rango no se puede guardar y la app indica el motivo.

**Búsqueda**

- **FR-020**: El sistema MUST permitir filtrar la lista de tarifas escribiendo parte del concepto, mostrando solo las coincidentes a medida que se escribe.
- **FR-021**: La búsqueda MUST ignorar mayúsculas, minúsculas y tildes.
- **FR-022**: Si la búsqueda no tiene resultados, el sistema MUST indicarlo y ofrecer crear una tarifa nueva.

**Transversales**

- **FR-023**: Todas las funciones de esta feature MUST funcionar sin conexión a Internet (Principio I).
- **FR-024**: Todos los datos de esta feature (datos del negocio, logo, tarifas) MUST almacenarse solo en el equipo del usuario, sin enviarse a ningún servicio externo (Principio II).
- **FR-025**: Todos los mensajes, etiquetas y avisos MUST estar en español, en lenguaje sencillo y sin términos técnicos.

### Key Entities

- **Perfil del negocio**: datos de identidad del profesional que aparecerán en sus presupuestos. Atributos: nombre comercial, razón social, identificación fiscal, teléfono, email, dirección, web, logo (opcional), moneda (euro, fija), tipo de IVA general (21 % por defecto), estado (completo / incompleto). Existe uno solo por instalación.
- **Logo**: imagen que representa al negocio. Atributos: imagen, formato, tamaño. Pertenece al perfil del negocio; como máximo uno a la vez.
- **Tarifa**: precio unitario de un trabajo o material. Atributos: concepto, unidad, precio. La combinación concepto + unidad es única. Las tarifas serán la fuente de los importes de los futuros presupuestos.
- **Unidad**: forma de medir una tarifa (unidad, hora, metro, m², metro lineal o personalizada).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Un profesional sin conocimientos técnicos completa la configuración inicial de su negocio (datos obligatorios y logo) en menos de 5 minutos y sin ayuda.
- **SC-002**: Crear una tarifa nueva lleva menos de 30 segundos.
- **SC-003**: El 100 % de los datos, el logo y las tarifas guardados se recuperan idénticos tras cerrar y reabrir la app.
- **SC-004**: El 100 % de los intentos de guardar logos no válidos o tarifas con precio ausente, cero o negativo se rechazan con un mensaje que el usuario entiende sin ayuda.
- **SC-005**: Con 500 tarifas, la búsqueda muestra los resultados de forma inmediata (percibida como instantánea, menos de 1 segundo) mientras se escribe.
- **SC-006**: Todas las historias de usuario se completan con éxito con la red desactivada.
- **SC-007**: El 90 % de los usuarios encuentra una tarifa concreta en una lista de más de 50 tarifas en menos de 10 segundos.

## Assumptions

- Una sola instalación sirve a un único profesional y a un único negocio; no hay cuentas, inicio de sesión ni varios perfiles (fuera de alcance).
- Los formatos de logo admitidos son PNG y JPG, con un máximo de 5 MB; es un valor razonable para un logo y puede ajustarse en planificación.
- Los precios admiten como máximo dos decimales.
- Moneda única: euro. Los precios de tarifa se guardan sin IVA; un único tipo de IVA general por negocio (21 % por defecto). Tipos de IVA distintos por tarifa quedan fuera de esta feature.

## Clarifications

### Session 2026-11-17

- Q: ¿Qué datos del negocio son obligatorios? → A: Nombre comercial, NIF/CIF, teléfono y dirección; opcionales: razón social, email y web.
- Q: ¿Moneda y tratamiento del IVA? → A: Euros, precios de tarifa sin IVA y un tipo de IVA general configurable (21 % por defecto).
- El mismo concepto con distinta unidad se considera una tarifa distinta y legítima.
- Borrar una tarifa no afecta a presupuestos ya existentes; como la generación de presupuestos está fuera de alcance, esta relación se definirá en su propia feature.
- La exportación y copia de seguridad de los datos (exigida por la constitución) se tratará en una feature aparte; esta feature solo garantiza su conservación local.
- Fuera de alcance: dictado por voz, generación de presupuestos y PDF, importación de tarifas desde Excel, varios negocios o varios usuarios.
