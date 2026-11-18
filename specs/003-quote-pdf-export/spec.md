# Feature Specification: PDF del presupuesto

**Feature Branch**: `003-quote-pdf-export`

**Created**: 2026-11-18

**Status**: Draft

**Input**: User description: "El profesional convierte un borrador de presupuesto ya revisado en un PDF con su logo y sus datos, listo para entregar al cliente. Usuario: el profesional de oficio. Problema: el cliente espera un documento con aspecto profesional, no una lista en pantalla. Historias: generar el PDF de un presupuesto revisado con logo, datos, líneas, importes y total (P1); no poder generar el PDF si hay líneas pendientes de precio (P1); elegir dónde se guarda el PDF (P1); generar el PDF sin conexión (P2). Casos límite: presupuesto sin líneas, muchas líneas que no caben en una página, conceptos muy largos, sin logo configurado, ubicación de guardado sin permisos o sin espacio, archivo con el mismo nombre ya existente. Contenido y diseño exactos del PDF (numeración, fecha, validez, IVA, datos del cliente): [TBD: Roberto los define]. Fuera de alcance: edición del PDF dentro de la app, envío por email o WhatsApp, firma, historial de presupuestos, plantillas de diseño alternativas."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Generar el PDF del presupuesto revisado (Priority: P1)

Con el borrador ya dado por bueno, el profesional pulsa para generar el PDF. La app le muestra un resumen de lo que va a emitir y, cuando lo confirma, crea un documento con su logo, sus datos, las líneas, los importes y el total, listo para entregar al cliente.

**Why this priority**: es el resultado final del producto. Sin este documento, el trabajo de dictar y revisar no llega al cliente.

**Independent Test**: con un perfil completo, un logo y un borrador revisado, se genera el PDF, se abre en un visor distinto de la app y se compara línea a línea con el borrador.

**Acceptance Scenarios**:

1. **Given** un borrador revisado con una línea de bajante de 3 metros a 25,00 € (75,00 €), una línea de mano de obra de 2 horas a 30,00 € (60,00 €) y un IVA del 21 %, **When** el profesional confirma la generación, **Then** el PDF muestra esas dos líneas con su concepto, cantidad, unidad, precio e importe, una base de 135,00 €, un IVA del 21 % de 28,35 € y un total de 163,35 €.
2. **Given** un perfil del negocio completo con logo, **When** se genera el PDF, **Then** el documento muestra el logo sin deformar y el nombre comercial, la razón social si existe, el NIF/CIF, el teléfono, la dirección y, si existen, el email y la web.
3. **Given** un PDF generado, **When** se abre en cualquier visor de PDF habitual, en este equipo o en otro, **Then** se ve igual, con las tildes, la "ñ" y el símbolo "€" correctos.
4. **Given** un borrador revisado, **When** el profesional pulsa generar, **Then** la app le pide los datos del cliente, el número y la validez, le muestra el número de líneas y el total, y no crea ningún archivo hasta que confirme.
5. **Given** la petición de confirmación, **When** el profesional cancela, **Then** no se crea ningún archivo, el borrador no cambia y el número propuesto no se gasta.
6. **Given** el nombre del cliente "Ana Ruiz" y su teléfono, sin más datos, **When** se genera el PDF, **Then** el documento muestra ese nombre y ese teléfono, sin huecos ni etiquetas vacías para los datos no rellenados.
7. **Given** la petición de confirmación con el nombre del cliente vacío, **When** el profesional intenta confirmar, **Then** la app le indica que el nombre es obligatorio y no crea ningún archivo.
8. **Given** que el último presupuesto guardado en 2026 lleva el número 2026-007, **When** el profesional prepara otro presupuesto ese año, **Then** la app propone el número 2026-008 y le permite cambiarlo antes de confirmar.
9. **Given** un presupuesto emitido el 18/11/2026 con la validez propuesta de 30 días, **When** se genera el PDF, **Then** el documento muestra la fecha de emisión 18/11/2026 y la validez hasta el 18/12/2026.

---

### User Story 2 - Impedir el PDF mientras falten datos (Priority: P1)

Si alguna línea sigue pendiente, la app no genera el documento: le dice al profesional qué líneas faltan y le lleva a completarlas.

**Why this priority**: un presupuesto con importes sin definir expone al profesional ante su cliente, y la constitución exige que la última palabra sobre cada precio sea suya (Principios IV y V).

**Independent Test**: con un borrador que tiene una línea sin precio, se intenta generar el PDF y se comprueba que no se crea ningún archivo y que la app señala esa línea; al completarla, el PDF se genera.

**Acceptance Scenarios**:

1. **Given** un borrador con una línea con "precio pendiente", **When** el profesional intenta generar el PDF, **Then** la app no crea ningún archivo e indica qué línea falta por completar, identificándola por su concepto.
2. **Given** un borrador con una línea con "cantidad pendiente", **When** el profesional intenta generar el PDF, **Then** la app actúa igual que con un precio pendiente.
3. **Given** varias líneas pendientes, **When** el profesional intenta generar el PDF, **Then** la app las enumera todas y le permite ir directamente a completarlas.
4. **Given** un borrador bloqueado por líneas pendientes, **When** el profesional las completa y da el borrador por bueno, **Then** puede generar el PDF.
5. **Given** un borrador sin líneas pendientes que aún no se ha dado por bueno, **When** el profesional intenta generar el PDF, **Then** la app le pide que lo revise y lo dé por bueno primero.

---

### User Story 3 - Elegir dónde se guarda el PDF (Priority: P1)

Al generar el documento, el profesional elige la carpeta y el nombre del archivo, y la app le confirma dónde ha quedado.

**Why this priority**: el profesional tiene que encontrar el archivo para poder entregarlo; sin esto, el PDF generado no le sirve.

**Independent Test**: se genera un PDF eligiendo una carpeta concreta, se comprueba que el archivo está en ella y que se abre desde allí.

**Acceptance Scenarios**:

1. **Given** la generación confirmada, **When** se pide la ubicación, **Then** la app propone un nombre de archivo que incluye la palabra "Presupuesto", el número del presupuesto y la fecha del día, y el profesional puede cambiarlo.
2. **Given** una carpeta elegida, **When** el PDF termina de generarse, **Then** el archivo está en esa carpeta y la app muestra su nombre y su ubicación completa.
3. **Given** la elección de ubicación abierta, **When** el profesional la cancela, **Then** no se crea ningún archivo y el borrador no cambia.
4. **Given** un archivo con el mismo nombre en la carpeta elegida, **When** el profesional intenta guardar, **Then** la app le pregunta si quiere sustituirlo y, si no acepta, conserva el archivo existente y le deja elegir otro nombre.

---

### User Story 4 - Generar el PDF sin conexión (Priority: P2)

El profesional genera el documento en cualquier lugar, tenga o no conexión a Internet.

**Why this priority**: el Principio I de la constitución exige que el flujo principal nunca dependa de la red. Tiene prioridad P2 porque se apoya en las historias anteriores.

**Independent Test**: con la red del equipo desactivada, se repite la generación de la Historia 1 y se obtiene el mismo documento.

**Acceptance Scenarios**:

1. **Given** el equipo sin conexión a Internet, **When** el profesional genera el PDF, **Then** obtiene el mismo documento que con conexión y no aparece ningún aviso por falta de red.

---

### Edge Cases

- **Presupuesto sin líneas**: la app no genera el PDF e indica que hay que añadir al menos una línea.
- **Muchas líneas que no caben en una página**: el documento continúa en páginas sucesivas, repite la cabecera de la tabla de líneas en cada una, numera las páginas ("Página 2 de 3") y muestra la base, el IVA y el total una sola vez, al final. Ninguna línea se parte entre dos páginas.
- **Conceptos muy largos**: el concepto completo se muestra en varios renglones, sin recortarse y sin invadir las demás columnas.
- **Datos del negocio muy largos**: se muestran completos en varios renglones, sin solaparse con el logo.
- **Sin logo configurado**: el PDF se genera sin logo y sin dejar un hueco vacío; la petición de confirmación avisa de que saldrá sin logo.
- **Logo guardado que ya no se puede leer**: la app avisa y permite generar el PDF sin logo o ir a "Mi negocio" para cargarlo de nuevo.
- **Perfil del negocio incompleto**: la app no genera el PDF, indica qué datos obligatorios faltan y ofrece un acceso directo a "Mi negocio".
- **Cliente sin nombre**: la app no genera el PDF e indica que el nombre es obligatorio.
- **Datos del cliente muy largos**: se muestran completos en varios renglones, sin solaparse con otros elementos.
- **Número vacío**: la app no genera el PDF e indica que el presupuesto necesita un número.
- **Número cambiado a mano**: si sigue el formato año-número, la numeración continúa a partir de él; con cualquier otro formato se usa solo en ese presupuesto y la serie automática no cambia.
- **Primer presupuesto del año**: la numeración empieza en 001 con el año de la fecha de emisión.
- **Más de 999 presupuestos en un año**: la numeración continúa con más cifras (p. ej. 2026-1000).
- **Generación cancelada o fallida**: el número propuesto no se gasta y se vuelve a proponer.
- **Otro PDF del mismo borrador sin cambios**: la app propone el número que ya tenía ese presupuesto, sin gastar uno nuevo.
- **Otro PDF de un borrador con líneas o IVA cambiados**: la app propone el siguiente número de la serie, para que dos documentos con contenido distinto no compartan número. El profesional puede cambiarlo antes de confirmar.
- **Validez no válida**: la app rechaza un plazo que no sea un número entero de días entre 1 y 365 e indica el motivo.
- **Ubicación sin permisos**: la app explica en lenguaje sencillo que no puede guardar ahí y permite elegir otra ubicación sin repetir la confirmación.
- **Sin espacio suficiente**: la app lo indica, permite elegir otra ubicación y no deja ningún archivo incompleto.
- **Ubicación que desaparece al guardar** (p. ej. se retira la memoria USB): se trata igual que una ubicación sin permisos.
- **Archivo con el mismo nombre ya existente**: solo se sustituye si el profesional lo confirma. Si la escritura falla, el archivo anterior queda intacto.
- **Archivo existente abierto en otro programa**: la app avisa de que está en uso y permite cerrarlo y reintentar, o elegir otro nombre.
- **Carácter que el PDF no puede mostrar** (p. ej. un emoji en un concepto o en el nombre del cliente): la app no genera el PDF e indica qué dato y qué carácter hay que corregir.
- **IVA del negocio cambiado después de generar el borrador**: el aviso de la confirmación no impide generar el PDF con el tipo aplicado al borrador. Si el profesional elige actualizarlo, los importes se recalculan y el borrador vuelve a requerir revisión antes de generar el PDF.
- **Fragmentos del dictado sin interpretar**: no aparecen en el PDF; la petición de confirmación recuerda que existen para que el profesional decida si le falta alguna línea.
- **Borrador editado después de generar un PDF**: el archivo ya generado no cambia. El borrador vuelve a requerir revisión y se puede generar otro PDF con un número nuevo.
- **Pulsación repetida del botón de generar**: se genera un único documento.
- **Cierre inesperado durante la generación**: el borrador se conserva y no queda ningún archivo incompleto con el nombre elegido.

## Requirements *(mandatory)*

### Functional Requirements

**Condiciones para generar**

- **FR-001**: El sistema MUST permitir generar el PDF solo si el borrador está revisado, tiene al menos una línea, ninguna está pendiente de precio o de cantidad, y el presupuesto tiene nombre del cliente, número y validez correctos.
- **FR-002**: Si no se cumple FR-001, el sistema MUST NOT crear ningún archivo y MUST indicar el motivo, enumerando por su concepto cada línea pendiente y ofreciendo ir a completarlas.
- **FR-003**: El sistema MUST NOT generar el PDF si el perfil del negocio está incompleto, y MUST indicar qué datos obligatorios faltan y ofrecer un acceso directo a "Mi negocio".
- **FR-004**: El sistema MUST pedir una confirmación explícita antes de crear el archivo, mostrando los datos del cliente, el número del presupuesto, las fechas, el número de líneas y el total, y permitiendo corregir los datos del cliente, el número y la validez. Ninguna acción automática MUST poder saltarse este paso (Principio V).
- **FR-005**: La petición de confirmación MUST avisar, cuando corresponda, de que el documento saldrá sin logo, de que el tipo de IVA del negocio ha cambiado o de que hay fragmentos del dictado sin interpretar.
- **FR-005b**: El aviso de cambio de IVA MUST NOT impedir generar el PDF con el tipo aplicado al borrador. Si el profesional actualiza el tipo, el sistema MUST recalcular los importes y devolver el borrador al estado de pendiente de revisión.

**Contenido del documento**

- **FR-006**: El PDF MUST mostrar el logo del negocio, si existe, conservando sus proporciones y su transparencia.
- **FR-007**: El PDF MUST mostrar los datos del negocio: nombre comercial, NIF/CIF, teléfono y dirección completa y, si existen, razón social, email y web.
- **FR-008**: El PDF MUST mostrar cada línea del borrador, en el mismo orden, con su concepto, cantidad, unidad, precio unitario sin IVA e importe.
- **FR-009**: El PDF MUST mostrar la base, el tipo de IVA aplicado, el importe del IVA y el total con IVA, con los mismos valores que el borrador. El sistema MUST NOT recalcular, redondear de otra forma ni modificar ningún importe.
- **FR-010**: Los importes MUST mostrarse en euros con el formato español (p. ej. "1.234,56 €").
- **FR-011**: El PDF MUST NOT incluir información interna de la app: origen de los precios, marcas de revisión, texto transcrito ni fragmentos del dictado.
- **FR-012**: El PDF MUST mostrar el nombre del cliente, que es obligatorio, y, si se han rellenado, su dirección, teléfono, email y NIF. Los datos no rellenados MUST NOT dejar huecos ni etiquetas vacías.
- **FR-012b**: El sistema MUST conservar los datos del cliente, el número y la validez junto al borrador en curso entre sesiones, y MUST vaciarlos al empezar un borrador nuevo.
- **FR-013**: El PDF MUST mostrar el número del presupuesto. El sistema MUST proponer el siguiente número correlativo del año de emisión con el formato año-número de al menos tres cifras (p. ej. 2026-001), y el profesional MUST poder cambiarlo antes de confirmar.
- **FR-013b**: Un número MUST gastarse solo cuando el PDF se haya guardado correctamente. Si las líneas y el tipo de IVA del borrador no han cambiado desde el último PDF guardado, el sistema MUST proponer el número que ya tenía; si han cambiado, MUST proponer el siguiente número de la serie. Corregir solo los datos del cliente o la validez MUST NOT cambiar el número propuesto.
- **FR-013c**: Si el profesional cambia el número por otro con formato año-número, la numeración de ese año MUST continuar a partir del mayor número usado. Un número con otro formato MUST NOT alterar la serie automática.
- **FR-014**: El PDF MUST mostrar la fecha de emisión, que es automáticamente el día de la generación, y la fecha de validez, en formato dd/mm/aaaa.
- **FR-014b**: La validez MUST ser de 30 días por defecto y el profesional MUST poder cambiarla en cada presupuesto por un número entero de días entre 1 y 365. La fecha de validez MUST ser la fecha de emisión más esos días.

**Formato y paginación**

- **FR-015**: El documento MUST poder abrirse y verse igual en cualquier visor de PDF habitual, sin depender de las tipografías instaladas en el equipo de quien lo abre.
- **FR-016**: El documento MUST tener tamaño A4 en vertical y un único diseño.
- **FR-017**: Si las líneas no caben en una página, el documento MUST continuar en las siguientes, repitiendo la cabecera de la tabla, numerando las páginas y sin partir ninguna línea entre dos páginas.
- **FR-018**: Los conceptos y los datos del negocio largos MUST mostrarse completos en varios renglones, sin recortarse ni solaparse con otros elementos.
- **FR-018b**: Si un texto del documento contiene un carácter que el PDF no puede mostrar, el sistema MUST NOT generar el archivo y MUST indicar qué dato y qué carácter hay que corregir.
- **FR-019**: Sin logo, el documento MUST reorganizar la cabecera sin dejar un hueco vacío.

**Guardado del archivo**

- **FR-020**: El profesional MUST poder elegir la carpeta y el nombre del archivo en cada generación. El sistema MUST proponer un nombre que incluya "Presupuesto", el número del presupuesto y la fecha del día (p. ej. "Presupuesto 2026-001 2026-11-18.pdf"), sustituyendo por un guion los caracteres que no se admiten en un nombre de archivo, y MUST asegurar que el archivo termine en ".pdf".
- **FR-021**: Al terminar, el sistema MUST confirmar que el PDF se ha guardado e indicar su nombre y su ubicación completa.
- **FR-022**: Si ya existe un archivo con ese nombre, el sistema MUST sustituirlo solo con la confirmación explícita del profesional.
- **FR-023**: Si el archivo no se puede guardar por falta de permisos, de espacio, porque la ubicación no está disponible o porque está en uso, el sistema MUST explicar el motivo en lenguaje sencillo y permitir elegir otra ubicación o reintentar.
- **FR-024**: Un fallo al guardar MUST NOT dejar archivos incompletos, dañar un archivo existente ni modificar el borrador.
- **FR-025**: Cancelar la confirmación o la elección de ubicación MUST NOT crear ningún archivo ni modificar el borrador.
- **FR-026**: El sistema MUST evitar que una pulsación repetida inicie varias generaciones a la vez.

**Transversales**

- **FR-027**: Todas las funciones de esta feature MUST funcionar sin conexión a Internet (Principio I).
- **FR-028**: El documento y sus datos MUST permanecer en el equipo del usuario, sin enviarse a ningún servicio externo (Principio II).
- **FR-028b**: Los registros técnicos locales de la app MUST NOT incluir los datos del cliente ni el contenido del presupuesto.
- **FR-029**: Todos los mensajes, etiquetas y avisos de la app y todos los textos del documento MUST estar en español, en lenguaje sencillo y sin términos técnicos.

### Key Entities

- **Documento de presupuesto**: archivo PDF creado a partir del borrador revisado. Atributos: contenido, número del presupuesto, fecha de emisión, fecha de validez, número de páginas, nombre y ubicación elegidos. La app no guarda un registro de los documentos generados.
- **Cliente del presupuesto**: destinatario del documento. Atributos: nombre (obligatorio), dirección, teléfono, email y NIF (opcionales). Pertenece al borrador en curso; no existe una agenda de clientes.
- **Numeración de presupuestos**: serie correlativa por año. Atributos: año y último número usado. De los presupuestos ya emitidos, la app solo recuerda esta serie y el último número emitido del borrador en curso.
- **Borrador de presupuesto**: definido en la feature 002. Aporta las líneas, la base, el tipo de IVA aplicado, el IVA, el total y el estado de revisión.
- **Línea del borrador**: definida en la feature 002. Aporta el concepto, la cantidad, la unidad, el precio, el importe y las marcas de pendiente.
- **Perfil del negocio y logo**: definidos en la feature 001. Aportan la identidad del profesional en el documento.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Un profesional pasa de un borrador revisado a tener el PDF guardado en menos de 1 minuto y sin ayuda.
- **SC-002**: En un equipo de gama media, un presupuesto de 50 líneas se genera en menos de 5 segundos desde que se elige la ubicación.
- **SC-003**: En el 100 % de los documentos de prueba, las líneas, los importes, el IVA y el total coinciden exactamente con el borrador.
- **SC-004**: El 100 % de los intentos de generar con líneas pendientes, sin líneas, sin revisar o sin nombre del cliente terminan sin crear ningún archivo y con el motivo indicado.
- **SC-005**: Un presupuesto de 200 líneas con conceptos de 160 caracteres se genera sin texto recortado, solapado ni líneas perdidas.
- **SC-006**: Los documentos de prueba se abren correctamente en al menos tres visores de PDF habituales.
- **SC-007**: Ningún fallo de guardado provoca la pérdida del borrador ni daña un archivo existente.
- **SC-008**: Todas las historias de usuario se completan con éxito con la red desactivada.

## Assumptions

- El PDF se genera a partir del único borrador en curso de la feature 002. Dar el borrador por bueno no sustituye a la confirmación explícita de esta feature.
- El IVA se muestra desglosado como en el borrador: base, tipo aplicado, importe del IVA y total.
- El diseño es único, en A4 vertical: cabecera con logo y datos del negocio, datos del cliente, tabla de líneas y totales al final.
- Generar el PDF no cambia el estado del borrador, que sigue en curso hasta que el profesional empiece otro.
- El profesional abre el archivo desde la carpeta elegida; la app solo le indica dónde está.
- Los datos del cliente se muestran tal como se escriben, sin comprobar su formato, porque pueden ser de particulares, empresas o clientes extranjeros.
- Sin historial de presupuestos, la app no puede detectar si un número escrito a mano ya se usó en un presupuesto anterior.
- Un "equipo de gama media" es un portátil con 4 núcleos y 8 GB de memoria, como en la feature 002.
- Fuera de alcance: edición del PDF dentro de la app, envío por email o WhatsApp, firma, historial de presupuestos, agenda de clientes y plantillas de diseño alternativas.

## Clarifications

### Session 2026-11-18

- Q: ¿Qué datos del cliente aparecen en el PDF y cuáles son obligatorios? → A: Nombre obligatorio; dirección, teléfono, email y NIF opcionales.
- Q: ¿El presupuesto lleva número y cómo se asigna? → A: Automático y correlativo por año (p. ej. 2026-001), editable antes de confirmar.
- Q: ¿Qué fechas aparecen y cómo se fija la validez? → A: Fecha de emisión automática y validez de 30 días, editable en cada presupuesto.
- Q: ¿Qué debe hacer la app si un dato del presupuesto lleva un carácter que el PDF no puede mostrar? → A: No generar el PDF e indicar qué dato y qué carácter hay que corregir.
- Q: ¿El cambio del IVA del negocio impide generar el PDF? → A: No; se puede generar con el tipo aplicado al borrador. Si el profesional lo actualiza, el borrador vuelve a requerir revisión.
- Q: ¿Pueden los registros técnicos locales contener datos del cliente o del presupuesto? → A: No; solo el detalle técnico necesario para diagnosticar fallos.
- Q: ¿Se acepta el diseño propuesto para el documento? → A: Sí: A4 vertical, IVA desglosado y la estructura definida en el contrato del documento.
- Q: Si se edita un presupuesto ya emitido, ¿qué número propone la app? → A: El siguiente de la serie si cambian las líneas o el IVA; el mismo si no cambian.
