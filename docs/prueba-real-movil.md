# Prueba real: instalador de Windows y móvil como apoyo

Valida la versión 0.1.0 tal como la usará un profesional: instalada desde el archivo de descarga en un
ordenador limpio, grabando la visita con el móvil y llevando el PDF al móvil.

- **Instalador**: `Presupuestos por Voz_0.1.0_x64-setup.exe`
- **SHA-256**: `68094c9a3f5267ea69af180affc69c730229d26d306ca346a5e36a2774adaf0d`
- **Origen**: `main` en `af06a01`. El código de la app es el mismo que en `194781a`, el commit con el que se compiló.

Anota en cada fila ✅, ❌ o una observación, y la fecha.

## 1. Instalación en un ordenador limpio

Usa un equipo distinto del de desarrollo, que ya tiene Rust, WebView2 y las librerías de Visual C++. Lo ideal es
un Windows 10 y un Windows 11 de un usuario normal.

| # | Comprobación | Resultado |
|---|---|---|
| 1.1 | `Get-FileHash` coincide con `SHA256.txt` | |
| 1.2 | SmartScreen avisa y se puede seguir con «Más información» → «Ejecutar de todas formas» | |
| 1.3 | Se instala sin permisos de administrador y aparece en el menú Inicio | |
| 1.4 | Windows 10: si falta WebView2, el instalador lo resuelve (anota si necesitó Internet) | |
| 1.5 | La app arranca con la red desactivada, sin avisos de conexión | |
| 1.6 | «Mi negocio» y «Tarifas» se guardan y siguen ahí al cerrar y reabrir | |

## 2. Grabación con el móvil e importación

Graba la misma visita con cada móvil, hablando como en una obra real. Ejemplo:
«cambiar bajante, 3 metros; mano de obra 2 horas; codo de PVC, 4 unidades».

| # | Móvil y app de grabación | Formato | Se importa | Transcripción correcta | Líneas con precio |
|---|---|---|---|---|---|
| 2.1 | iPhone · Notas de voz (calidad «Comprimida») | M4A · AAC | | | |
| 2.2 | iPhone · Notas de voz (calidad «Sin pérdida») | M4A · ALAC | | | |
| 2.3 | Samsung · Grabadora de voz | M4A | | | |
| 2.4 | Xiaomi · Grabadora | MP3 o M4A | | | |
| 2.5 | Google Pixel · Grabadora (exportar audio) | M4A o WAV | | | |

Casos límite:

| # | Comprobación | Resultado |
|---|---|---|
| 2.6 | Nota de voz de WhatsApp (`.opus`): el diálogo de importación no la ofrece; el LEEME lo explica | |
| 2.7 | Audio de más de 10 minutos: la app lo rechaza con un mensaje claro | |
| 2.8 | Grabación en una obra con ruido (radial, furgoneta, calle) | |
| 2.9 | Audio con silencio: «No se ha oído nada en el audio…» | |
| 2.10 | Paso del audio por cable USB, por correo y por nube: el archivo llega y se importa igual | |

## 3. PDF en el móvil

Genera el PDF de la prueba 2.1 y llévalo al móvil.

| # | Comprobación | Resultado |
|---|---|---|
| 3.1 | Android: se abre en el visor de Google Drive y en Adobe Acrobat; logo, tildes, «ñ» y «€» correctos | |
| 3.2 | iPhone: se abre en Archivos y en Adobe Acrobat; mismo contenido | |
| 3.3 | Enviado por WhatsApp, el cliente lo recibe y lo abre | |
| 3.4 | Enviado por correo como adjunto, se abre en Gmail y en Outlook del móvil | |
| 3.5 | Un presupuesto de varias páginas se lee bien en pantalla pequeña (cabecera de tabla repetida, «Página X de Y») | |
| 3.6 | Impreso en A4 desde el móvil o el ordenador, márgenes correctos | |

## 4. Tiempos de uso real

| # | Medición | Objetivo | Resultado |
|---|---|---|---|
| 4.1 | Desde importar el audio hasta tener el borrador | — | |
| 4.2 | Desde el borrador revisado hasta el PDF guardado | < 1 min (SC-001) | |
| 4.3 | Visita completa: grabar → PDF en el móvil del cliente | — | |

## Resultado

- **Fecha**: 2026-11-19. Prueba hecha por el responsable del proyecto.
- **Resultado**: todas las comprobaciones correctas. El audio grabado en el móvil se pasa al ordenador (por
  Google Drive) y se importa y transcribe bien. El PDF generado se lleva al móvil y se abre correctamente.
- **Fallos encontrados**: ninguno.
- **¿Se puede entregar?**: sí, la versión 0.1.0.
