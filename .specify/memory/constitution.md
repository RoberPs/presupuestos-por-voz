<!--
Sync Impact Report
==================
Cambio de versión: (plantilla sin versionar) → 1.0.0
Motivo: adopción inicial de la constitución (MAJOR inicial).

Principios definidos (desde placeholders de la plantilla):
  - [PRINCIPLE_1_NAME] → I. Offline-First
  - [PRINCIPLE_2_NAME] → II. Sin Servidor Propio
  - [PRINCIPLE_3_NAME] → III. IA con Degradación Elegante
  - [PRINCIPLE_4_NAME] → IV. Precios y Marca del Usuario, Nunca Inventados
  - [PRINCIPLE_5_NAME] → V. Revisión y Confirmación Humana Antes de Emitir

Secciones añadidas:
  - Restricciones del Producto y Tecnología (antes [SECTION_2_NAME])
  - Estándares de Calidad y Flujo de Trabajo (antes [SECTION_3_NAME])
  - Governance (contenido completo)

Secciones eliminadas: ninguna.

Plantillas dependientes: no se modifican (leen la constitución en tiempo de ejecución).
  - .specify/templates/plan-template.md       ⚠ revisar "Constitution Check" al ejecutar /speckit-plan
  - .specify/templates/spec-template.md       ✅ sin cambios requeridos
  - .specify/templates/tasks-template.md      ✅ sin cambios requeridos

TODOs diferidos:
  - TODO(QUALITY_STANDARDS): calidad de código, testing, UX, rendimiento, seguridad,
    accesibilidad y convenciones — pendiente de definir por Roberto.
  - TODO(TECH_STACK): stack tecnológico — se decide en /speckit-plan.
  - PROJECT_NAME inferido como "Presupuestos por Voz" (nombre de trabajo); confirmar.
-->

# Presupuestos por Voz Constitution

Aplicación de escritorio para profesionales de oficios (fontanería, electricidad, pintura, etc.)
que genera presupuestos en PDF a partir de la visita al cliente dictada por voz.

## Core Principles

### I. Offline-First

- Toda función principal (dictado y transcripción, edición del presupuesto, gestión de tarifas
  y clientes, generación del PDF) MUST funcionar sin conexión a Internet.
- La conexión MAY usarse solo para mejorar la experiencia (p. ej. mayor calidad de transcripción);
  NUNCA puede ser requisito para completar un flujo principal.
- Ninguna pantalla ni flujo principal MUST bloquearse, fallar o quedar inutilizable por falta de red.
- Cada feature MUST incluir en su especificación el comportamiento sin conexión y ser verificable
  con la red desactivada.

**Rationale**: el usuario trabaja en obras, sótanos y domicilios con cobertura irregular; la
herramienta tiene que servir siempre, no solo cuando hay red.

### II. Sin Servidor Propio

- El proyecto NO tiene backend, hosting, base de datos remota ni cuentas de usuario propias.
- Cada instalación da servicio a un único cliente (el profesional) y todos sus datos (tarifas,
  logo, clientes, presupuestos, grabaciones) MUST residir en su propio equipo.
- Cualquier servicio externo (p. ej. API de transcripción) MUST ser opcional, invocado
  directamente desde el cliente y configurado por el propio usuario; no puede introducir un
  servidor intermedio del proyecto.
- Queda prohibido introducir sincronización, telemetría o almacenamiento en la nube gestionados
  por el proyecto sin una enmienda de esta constitución.

**Rationale**: elimina costes operativos, superficie de ataque y dependencia de un tercero;
el profesional es dueño absoluto de sus datos.

### III. IA con Degradación Elegante

- La transcripción de voz MUST disponer siempre de un motor local funcional, que es la opción
  base garantizada.
- Cuando haya conexión y el usuario lo haya habilitado, MAY usarse una API externa como mejora.
- Si la API externa falla, expira, devuelve error o no hay red, la app MUST recurrir
  automáticamente a la opción local sin perder el audio ni el trabajo en curso.
- El usuario MUST ser informado de forma clara y no bloqueante de qué motor se ha usado y de
  cualquier fallback ocurrido.
- Toda integración con IA MUST tener pruebas del camino de fallo (API caída, sin red, timeout).

**Rationale**: la IA externa es un acelerador, no un punto único de fallo; la app nunca puede
quedar inservible por depender de un proveedor.

### IV. Precios y Marca del Usuario, Nunca Inventados

- Todo presupuesto MUST usar el logo, los datos de empresa y las tarifas configuradas por el
  propio usuario.
- La IA NUNCA inventa, estima ni infiere precios. Cada importe MUST proceder de una tarifa
  configurada o de una introducción manual del usuario.
- Si una partida no tiene tarifa asociada, MUST quedar marcada visiblemente como "precio
  pendiente" y no puede contabilizarse como importe definitivo.
- La trazabilidad de cada precio (tarifa de origen o entrada manual) MUST conservarse en el
  modelo de datos del presupuesto.

**Rationale**: un precio inventado expone al profesional a pérdidas económicas y a conflictos
con su cliente; la responsabilidad del precio es solo suya.

### V. Revisión y Confirmación Humana Antes de Emitir

- Ningún PDF de presupuesto MUST generarse ni emitirse sin una acción explícita de confirmación
  del usuario.
- Antes de confirmar, el usuario MUST poder revisar y editar todas las partidas, cantidades,
  precios, textos y datos del cliente.
- La generación del PDF MUST bloquearse (o advertir de forma explícita y requerir confirmación
  adicional) mientras existan partidas con precio pendiente.
- Ninguna acción automática (IA, plantillas, importaciones) puede saltarse este paso.

**Rationale**: el presupuesto es un documento comercial con valor contractual; el profesional
debe tener siempre la última palabra.

## Restricciones del Producto y Tecnología

- **Tipo de aplicación**: aplicación de escritorio instalable, de uso individual.
- **Datos**: almacenamiento exclusivamente local en el equipo del usuario; el usuario MUST poder
  exportar y respaldar sus datos sin depender de servicios del proyecto.
- **Credenciales de terceros**: las claves de API externas, si se usan, las aporta el usuario y
  MUST almacenarse localmente de forma protegida, nunca en texto plano dentro del código.
- **Stack tecnológico**: TODO(TECH_STACK): se decide en `/speckit-plan`. Cualquier elección
  MUST ser compatible con los principios I–V (ejecución local, motor de transcripción local,
  generación de PDF sin servicios remotos).

## Estándares de Calidad y Flujo de Trabajo

TODO(QUALITY_STANDARDS): pendiente de definir por Roberto. Áreas a cubrir:

- Calidad de código y convenciones.
- Estrategia de testing.
- UX.
- Rendimiento.
- Seguridad.
- Accesibilidad.

Mientras no se definan, cada plan y revisión MUST, como mínimo, verificar el cumplimiento de los
principios I–V (incluido el comportamiento sin red y el fallback de IA).

## Governance

- Esta constitución prevalece sobre cualquier otra práctica, guía o decisión de diseño del
  proyecto. Si una spec, plan o tarea entra en conflicto con ella, la constitución gana.
- **Enmiendas**: cualquier cambio MUST documentarse en este archivo con su Sync Impact Report,
  ser aprobado por el responsable del proyecto (Roberto) e incluir, si afecta a trabajo existente,
  un plan de migración.
- **Versionado** (semver):
  - MAJOR: eliminación o redefinición incompatible de un principio o de la gobernanza.
  - MINOR: nuevo principio o sección, o ampliación material de la guía.
  - PATCH: aclaraciones, redacción o correcciones sin cambio semántico.
- **Cumplimiento**: cada `/speckit-plan` MUST superar el "Constitution Check" contra los
  principios I–V; toda revisión de cambios MUST comprobarlo. Cualquier excepción MUST
  justificarse por escrito en el plan (sección de complejidad) o tramitarse como enmienda.

**Version**: 1.0.0 | **Ratified**: 2026-11-17 | **Last Amended**: 2026-11-17
