# Feature Specification: Borrador de presupuesto por voz

**Feature Branch**: `002-voice-quote-draft`

**Created**: 2026-11-18

**Status**: Draft

**Input**: User description: "El profesional dicta por voz lo que ha visto en la visita y la app le propone un borrador de presupuesto con sus tarifas, que él revisa y corrige. Usuario: el profesional de oficio, normalmente en obra o en el coche, con las manos ocupadas y ruido ambiente. Problema: redactar presupuestos a mano después de la visita lleva tiempo y se retrasa. Ejemplo de dictado: 'cambiar bajante, 3 metros, mano de obra 2 horas'. Historias: dictar y obtener un borrador con líneas calculadas con sus tarifas (P1); editar, añadir y borrar líneas (P1); dictar sin conexión a Internet (P1); ver y corregir el texto transcrito y regenerar el borrador (P2); escribir o pegar la visita a mano (P2). Casos límite: concepto sin tarifa (pendiente de precio, nunca inventado), cantidad o unidad ambiguas o ausentes, audio vacío, muy ruidoso o muy largo, varios trabajos mezclados, micrófono no disponible o sin permiso, sin tarifas creadas. Fuera de alcance: generar y exportar el PDF, historial de presupuestos, datos del cliente final [TBD: ¿entran en esta feature o en otra?], envío por email o WhatsApp, otros idiomas distintos del español."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Dictar la visita y obtener un borrador (Priority: P1)

Al terminar la visita, el profesional pulsa para dictar y cuenta lo que ha visto con sus propias palabras. La app transcribe el dictado, reconoce los trabajos, las cantidades y las unidades, busca cada concepto en sus tarifas y le propone un borrador con sus líneas, importes y total.

**Why this priority**: es el valor central del producto: convertir una visita hablada en un presupuesto casi hecho, sin redactar a mano.

**Independent Test**: con las tarifas "Bajante" (25,00 € por metro) y "Mano de obra" (30,00 € por hora) creadas, se dicta "cambiar bajante, 3 metros, mano de obra 2 horas" y se comprueba que el borrador contiene las dos líneas con sus importes y el total.

**Acceptance Scenarios**:

1. **Given** las tarifas "Bajante" (25,00 €/metro) y "Mano de obra" (30,00 €/hora) y un IVA general del 21 %, **When** el profesional dicta "cambiar bajante, 3 metros, mano de obra 2 horas", **Then** el borrador muestra una línea de bajante de 3 metros a 25,00 € con importe 75,00 €, una línea de mano de obra de 2 horas a 30,00 € con importe 60,00 €, una base de 135,00 €, un IVA de 28,35 € y un total de 163,35 €.
2. **Given** un dictado con un concepto que no existe en las tarifas, **When** se genera el borrador, **Then** esa línea aparece marcada visiblemente como "precio pendiente", sin precio ni importe, y el resto de líneas se calcula con normalidad.
3. **Given** un dictado en curso, **When** el profesional lo detiene, **Then** la app indica que está procesando y muestra el borrador al terminar, sin más pasos.
4. **Given** un audio con el dictado del ejemplo grabado en el móvil y copiado al equipo, **When** el profesional lo importa, **Then** obtiene las mismas líneas, importes y total que al dictarlo en el ordenador.

---

### User Story 2 - Revisar y corregir las líneas del borrador (Priority: P1)

El profesional repasa el borrador propuesto: cambia cantidades, unidades, conceptos o precios, añade lo que faltó en el dictado y borra lo que sobra. Cuando está conforme, lo da por bueno.

**Why this priority**: la propuesta automática puede equivocarse y la última palabra es siempre del profesional (Principio V de la constitución).

**Independent Test**: sobre un borrador ya generado, se cambia una cantidad, se añade una línea y se borra otra; los importes y el total se actualizan en cada cambio y los cambios se conservan al cerrar y reabrir la app.

**Acceptance Scenarios**:

1. **Given** la línea de bajante de 3 metros a 25,00 € y la de mano de obra de 60,00 €, **When** el profesional cambia la cantidad de bajante a 4, **Then** al momento el importe de la línea pasa a 100,00 €, la base a 160,00 €, el IVA a 33,60 € y el total a 193,60 €.
2. **Given** un borrador, **When** el profesional añade una línea eligiendo una de sus tarifas e indica la cantidad, **Then** la línea toma el precio de la tarifa y el total se actualiza.
3. **Given** un borrador, **When** el profesional añade una línea escribiendo concepto, cantidad, unidad y precio a mano, **Then** la línea se incluye en el total y su precio queda identificado como entrada manual.
4. **Given** un borrador, **When** el profesional borra una línea y confirma, **Then** la línea desaparece y el total se actualiza.
5. **Given** una línea con precio pendiente, **When** el profesional le asigna una tarifa o escribe un precio, **Then** deja de estar pendiente y su importe entra en el total.
6. **Given** un borrador con alguna línea pendiente, **When** el profesional intenta darlo por bueno, **Then** la app no lo permite y le señala las líneas que faltan por completar.
7. **Given** un borrador sin líneas pendientes, **When** el profesional lo da por bueno, **Then** la app lo indica como revisado.

---

### User Story 3 - Dictar sin conexión a Internet (Priority: P1)

El profesional dicta en un sótano, en una obra o en una zona sin cobertura, y obtiene el borrador exactamente igual que con conexión.

**Why this priority**: el trabajo se hace a menudo sin cobertura, y la constitución exige que el flujo principal nunca dependa de la red (Principio I).

**Independent Test**: con la red del equipo desactivada, se repite el dictado de la Historia 1 y se obtiene el mismo borrador.

**Acceptance Scenarios**:

1. **Given** el equipo sin conexión a Internet, **When** el profesional dicta la visita del ejemplo, **Then** obtiene el borrador con sus líneas, importes y total.
2. **Given** el equipo sin conexión, **When** el profesional edita el borrador y lo da por bueno, **Then** todo funciona igual que con conexión y no aparece ningún aviso de error por falta de red.

---

### User Story 4 - Ver y corregir el texto transcrito (Priority: P2)

Junto al borrador, el profesional ve el texto que la app ha entendido. Si algo se ha entendido mal por el ruido, corrige el texto y vuelve a generar el borrador a partir de él, sin tener que dictar de nuevo.

**Why this priority**: corrige de una vez los errores de transcripción, aunque también se pueden arreglar editando las líneas una a una.

**Independent Test**: tras un dictado, se cambia "3 metros" por "5 metros" en el texto, se regenera y la línea pasa a 5 metros con su importe recalculado.

**Acceptance Scenarios**:

1. **Given** un borrador generado desde un dictado, **When** el profesional modifica el texto transcrito y pulsa regenerar, **Then** el borrador se rehace a partir del texto modificado.
2. **Given** un borrador con cambios hechos a mano, **When** el profesional pulsa regenerar, **Then** la app le avisa de que esos cambios se perderán y solo regenera si lo confirma.

---

### User Story 5 - Escribir la visita a mano (Priority: P2)

Si no puede dictar (micrófono averiado, lugar inadecuado, notas ya tomadas), el profesional escribe o pega el texto de la visita y obtiene el borrador igual.

**Why this priority**: garantiza que la app sirva aunque la voz no esté disponible, pero el dictado es el camino principal.

**Independent Test**: se pega el texto "cambiar bajante, 3 metros, mano de obra 2 horas" y el borrador coincide con el obtenido en la Historia 1.

**Acceptance Scenarios**:

1. **Given** las tarifas del ejemplo, **When** el profesional escribe o pega "cambiar bajante, 3 metros, mano de obra 2 horas" y genera el borrador, **Then** obtiene las mismas líneas, importes y total que al dictar ese mismo texto.
2. **Given** el cuadro de texto vacío, **When** el profesional intenta generar el borrador, **Then** la app le indica que escriba primero lo que quiere presupuestar.

---

### Edge Cases

- **Concepto sin tarifa**: la línea conserva el concepto dictado y queda como "precio pendiente"; la app nunca inventa, estima ni deduce un precio.
- **Concepto que encaja con varias tarifas** (p. ej. "pintura" con tarifas por metro y por hora, sin unidad dictada): la app no elige por su cuenta; la línea queda pendiente y ofrece esas tarifas para elegir una.
- **Cantidad ausente o ambigua** (p. ej. "unos cuantos metros"): la línea queda como "cantidad pendiente", sin importe y fuera del total, hasta que el profesional la indique.
- **Unidad ausente**: si el concepto solo tiene una tarifa, se usa la unidad de esa tarifa.
- **Unidad dictada distinta de la tarifa** (p. ej. "2 horas" con una tarifa por metro): la app no convierte ni aplica ese precio; la línea queda como "precio pendiente" y sugiere la tarifa parecida.
- **Rectificación durante el dictado** (p. ej. "3 metros, no, mejor 4"): la línea se marca para revisar y muestra el fragmento dictado original.
- **Audio vacío o solo silencio**: no se crea ningún borrador; la app avisa de que no ha oído nada y ofrece repetir el dictado o escribir el texto.
- **Audio muy ruidoso**: la app transcribe lo que pueda, avisa de que conviene revisar el texto y conserva el audio para poder repetir la transcripción.
- **Audio muy largo**: un minuto antes del límite de 10 minutos la app avisa; al llegar al límite detiene la grabación y procesa lo grabado sin perderlo.
- **Audio importado no válido**: si el archivo no tiene un formato admitido (M4A, MP3 o WAV) o está dañado, la app lo rechaza con un mensaje claro y conserva el borrador en curso.
- **Audio importado de más de 10 minutos**: la app lo rechaza indicando el límite, sin procesar solo una parte.
- **Varios trabajos mezclados en un dictado**: cada trabajo reconocido genera su propia línea, en el orden dictado, dentro de un único borrador. Un mismo concepto dictado dos veces da dos líneas, que no se suman.
- **Texto que no se puede interpretar**: los fragmentos que no se convierten en línea se muestran aparte, para que nada dictado se pierda sin que el profesional lo sepa.
- **Micrófono no disponible o sin permiso**: la app explica el problema en lenguaje sencillo, indica cómo resolverlo y ofrece escribir la visita a mano.
- **Micrófono desconectado mientras se graba**: la grabación se detiene, se conserva lo grabado y la app ofrece procesarlo.
- **Sin tarifas creadas**: el borrador se genera con todas las líneas como "precio pendiente" y la app ofrece un acceso directo a crear tarifas.
- **Tarifa modificada o borrada después de generar el borrador**: las líneas conservan el precio que tenían; solo cambia si el profesional regenera el borrador o reasigna la tarifa.
- **Cierre inesperado de la app durante el dictado o la edición**: al volver a abrirla se recuperan el audio ya grabado y el borrador en curso.

## Requirements *(mandatory)*

### Functional Requirements

**Dictado y transcripción**

- **FR-001**: El profesional MUST poder iniciar y detener un dictado, viendo en todo momento que la app está grabando y el tiempo transcurrido.
- **FR-002**: El sistema MUST permitir iniciar y detener el dictado en el ordenador con un solo gesto (un botón grande o una tecla).
- **FR-002b**: El profesional MUST poder importar un archivo de audio grabado con otro dispositivo, como el móvil, en formato M4A, MP3 o WAV. El audio importado MUST seguir el mismo proceso que un dictado y el sistema MUST guardar una copia propia, de forma que siga disponible aunque el archivo original se mueva o se borre.
- **FR-002c**: El sistema MUST rechazar los archivos de audio con formato no admitido, dañados o de más de 10 minutos, indicando el motivo y conservando el borrador en curso.
- **FR-003**: El sistema MUST transcribir el dictado en español dentro del propio equipo, sin necesitar conexión a Internet.
- **FR-004**: El sistema MUST conservar el audio del dictado hasta que el borrador se sustituya o se descarte, y MUST permitir repetir la transcripción si falla, sin que el profesional pierda lo grabado.
- **FR-005**: El sistema MUST limitar cada dictado a 10 minutos, avisar un minuto antes del límite y procesar lo grabado al alcanzarlo.
- **FR-006**: Si el audio está vacío o no contiene voz, el sistema MUST avisar, no crear ningún borrador y ofrecer repetir el dictado o escribir el texto.
- **FR-007**: Si la calidad del audio es baja, el sistema MUST avisar de que conviene revisar el texto transcrito.
- **FR-008**: Si el micrófono no está disponible o no tiene permiso, el sistema MUST explicar el motivo y cómo resolverlo, y MUST ofrecer importar un audio o escribir la visita a mano.
- **FR-009**: El sistema MUST mostrar que está procesando el dictado y MUST permitir cancelar el proceso sin perder el audio.

**Texto de la visita**

- **FR-010**: Al terminar la transcripción, el sistema MUST generar el borrador automáticamente y mostrar junto a él el texto transcrito, que el profesional MUST poder editar.
- **FR-011**: El profesional MUST poder regenerar el borrador a partir del texto editado.
- **FR-012**: Si el borrador tiene cambios manuales, el sistema MUST avisar de que se perderán y pedir confirmación antes de regenerarlo.
- **FR-013**: El profesional MUST poder escribir o pegar el texto de la visita y generar el borrador sin dictar.
- **FR-014**: A igual texto e iguales tarifas, el sistema MUST generar siempre el mismo borrador, tanto si el texto procede de un dictado como si se ha escrito o pegado.

**Generación del borrador**

- **FR-015**: El sistema MUST convertir el texto en líneas con concepto, cantidad, unidad, precio e importe, creando una línea por cada trabajo reconocido y respetando el orden del texto.
- **FR-016**: El sistema MUST reconocer cantidades expresadas con cifras o con palabras, incluidas las fracciones habituales (p. ej. "tres", "metro y medio").
- **FR-017**: El sistema MUST buscar cada concepto en las tarifas del profesional sin distinguir mayúsculas, tildes ni espacios sobrantes, teniendo en cuenta la unidad cuando exista más de una tarifa con ese concepto.
- **FR-018**: Si un concepto no tiene tarifa, el sistema MUST marcar la línea visiblemente como "precio pendiente" y MUST NOT inventar, estimar ni deducir ningún precio.
- **FR-019**: Si un concepto encaja con más de una tarifa, el sistema MUST dejar la línea pendiente y ofrecer esas tarifas para que el profesional elija.
- **FR-020**: Si la cantidad falta o es ambigua, el sistema MUST marcar la línea como "cantidad pendiente" y MUST NOT suponer ningún valor.
- **FR-021**: Si la unidad dictada no coincide con la de la tarifa, el sistema MUST NOT convertir unidades ni aplicar ese precio, y MUST dejar la línea como "precio pendiente" sugiriendo la tarifa parecida.
- **FR-022**: El sistema MUST mostrar aparte los fragmentos del texto que no haya podido convertir en líneas.
- **FR-023**: Sin tarifas creadas, el sistema MUST generar el borrador con las líneas pendientes de precio y ofrecer un acceso directo a crear tarifas.

**Edición del borrador**

- **FR-024**: El profesional MUST poder editar el concepto, la cantidad, la unidad y el precio de cualquier línea.
- **FR-025**: El profesional MUST poder añadir líneas eligiendo una tarifa o escribiéndolas a mano, y MUST poder borrar líneas previa confirmación.
- **FR-026**: El profesional MUST poder completar una línea pendiente asignándole una tarifa o escribiendo un precio.
- **FR-027**: El sistema MUST rechazar cantidades y precios que no sean mayores que cero o que tengan más de dos decimales, indicando el motivo.

**Cálculo**

- **FR-028**: El importe de cada línea MUST ser la cantidad por el precio, redondeado al céntimo.
- **FR-029**: El sistema MUST mostrar la base (suma de importes sin IVA), el IVA según el tipo general del negocio y el total con IVA.
- **FR-030**: Cualquier cambio en una línea MUST actualizar al momento su importe, la base, el IVA y el total.
- **FR-031**: Las líneas pendientes MUST quedar fuera de la base y del total; mientras existan, el sistema MUST indicar que el total es provisional y cuántas líneas faltan por completar.

**Trazabilidad y estado**

- **FR-032**: Cada línea MUST conservar el origen de su precio: la tarifa de la que procede, una entrada manual o pendiente. Cambiar a mano un precio procedente de una tarifa MUST convertir su origen en entrada manual.
- **FR-033**: Cada línea MUST conservar el precio asignado aunque la tarifa de origen se modifique o se borre después.
- **FR-034**: El profesional MUST poder dar el borrador por bueno solo cuando no queden líneas pendientes y haya al menos una línea. Editar un borrador revisado MUST devolverlo al estado de borrador.
- **FR-035**: El sistema MUST conservar el borrador en curso, su texto y su audio entre sesiones y ante un cierre inesperado.
- **FR-036**: El sistema MUST mantener un único borrador en curso; empezar uno nuevo MUST requerir confirmación si el actual tiene contenido.

**Transversales**

- **FR-037**: Todas las funciones de esta feature MUST funcionar sin conexión a Internet (Principio I).
- **FR-038**: El audio, el texto y el borrador MUST almacenarse solo en el equipo del usuario, sin enviarse a ningún servicio externo (Principio II).
- **FR-039**: El sistema MUST informar de forma clara y no bloqueante de que la transcripción se ha realizado en el propio equipo (Principio III).
- **FR-040**: Todos los mensajes, etiquetas y avisos MUST estar en español, en lenguaje sencillo y sin términos técnicos.
- **FR-041**: El borrador MUST NOT pedir ni guardar datos del cliente final; se incorporarán en la feature de generación del PDF.

### Key Entities

- **Dictado**: grabación de la visita. Atributos: audio, duración, origen (grabado en el equipo o importado), estado (grabando, procesando, transcrito, fallido). Pertenece a un borrador.
- **Texto de la visita**: contenido del que nace el borrador. Atributos: texto, origen (transcrito o escrito a mano), si se ha corregido.
- **Borrador de presupuesto**: propuesta en revisión. Atributos: líneas, base, tipo de IVA aplicado, IVA, total, estado (borrador o revisado), si tiene cambios manuales. Solo hay uno en curso.
- **Línea del borrador**: un trabajo o material. Atributos: concepto, cantidad, unidad, precio, importe, origen del precio (tarifa, manual o pendiente), tarifa de origen, marcas de pendiente o de revisión y fragmento de texto del que procede.
- **Tarifa**: definida en la feature 001. Es la fuente de los precios de las líneas.
- **Perfil del negocio**: definido en la feature 001. Aporta el tipo de IVA general.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Un profesional pasa de empezar a dictar a tener revisado un borrador de 5 líneas en menos de 5 minutos.
- **SC-002**: En un equipo de gama media, el borrador de un dictado de 1 minuto aparece en menos de 60 segundos desde que termina la grabación.
- **SC-003**: En un conjunto de prueba de dictados con ruido ambiente de obra o de coche, al menos el 90 % de los conceptos, cantidades y unidades se reconocen correctamente sin corrección manual.
- **SC-004**: El 100 % de las líneas sin tarifa aplicable quedan marcadas como pendientes; ningún borrador contiene un precio que no proceda de una tarifa o de una entrada manual.
- **SC-005**: Los importes y el total se actualizan de forma percibida como instantánea (menos de 1 segundo) tras cada cambio.
- **SC-006**: Todas las historias de usuario se completan con éxito con la red desactivada.
- **SC-007**: El 100 % de los textos idénticos, dictados o escritos, producen el mismo borrador con las mismas tarifas.
- **SC-008**: Ningún fallo de transcripción, cancelación o cierre inesperado provoca la pérdida del audio ya grabado ni del borrador en curso.

## Assumptions

- La app se usa en un ordenador con Windows; no hay versión para móvil en esta feature. Cuando no tiene el ordenador delante, el profesional graba con la aplicación de grabación de su móvil y copia el audio al equipo por sus propios medios.
- El motor de transcripción local se instala con la app y funciona desde el primer uso, sin descargas posteriores.
- Esta feature solo usa la transcripción local; la API externa opcional que permite la constitución se tratará en otra feature.
- El borrador se genera automáticamente tras la transcripción, porque el profesional suele tener las manos ocupadas; el texto se corrige después y el borrador se regenera.
- Dar el borrador por bueno solo lo marca como revisado; la confirmación definitiva y el bloqueo del PDF se tratarán en la feature de generación del PDF.
- Se reutilizan las tarifas, las unidades y el tipo de IVA general de la feature 001; un perfil del negocio incompleto no impide generar borradores.
- Un "equipo de gama media" es un portátil con 4 núcleos y 8 GB de memoria.
- Fuera de alcance: generar y exportar el PDF, historial de presupuestos, datos del cliente final, envío por email o WhatsApp, y otros idiomas distintos del español.

## Clarifications

### Session 2026-11-18

- Q: ¿Los datos del cliente final entran en esta feature o en otra? → A: En otra, junto con la generación del PDF.
- Q: ¿Cómo se graba cuando el profesional tiene las manos ocupadas? → A: En el ordenador con un solo gesto para empezar y otro para parar, y además importando un audio grabado con el móvil.
