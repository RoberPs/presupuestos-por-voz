# Contrato del documento PDF

**Feature**: `003-quote-pdf-export` | **Fecha**: 2026-11-18

Define el resultado visible del documento. El diseño es provisional hasta que Roberto defina el definitivo (spec, Assumptions).

## Formato

| Propiedad | Valor |
|---|---|
| Página | A4 vertical, 595,28 × 841,89 pt |
| Márgenes | 42 pt en los cuatro lados |
| Perfil | PDF/A-2b |
| Tipografía | Liberation Sans Regular y Bold, incrustadas |
| Idioma del documento | `es-ES` |
| Título del documento | `Presupuesto <número>` |

## Estructura

```text
Página 1
┌───────────────────────────────────────────────┐
│ [Logo]   Nombre comercial        PRESUPUESTO  │
│          Razón social            Nº 2026-001  │
│          NIF · Teléfono          Fecha        │
│          Dirección               Válido hasta │
│          Email · Web                          │
├───────────────────────────────────────────────┤
│ Cliente: nombre, NIF, dirección, tel., email  │
├───────────────────────────────────────────────┤
│ Concepto    Cantidad  Unidad  Precio  Importe │
│ ...                                           │
├───────────────────────────────────────────────┤
│                     Base imponible   135,00 € │
│                     IVA (21 %)        28,35 € │
│                     TOTAL            163,35 € │
│                                  Página 1 de 1│
└───────────────────────────────────────────────┘

Páginas siguientes
┌───────────────────────────────────────────────┐
│ Presupuesto 2026-001 · Nombre comercial       │
│ Concepto    Cantidad  Unidad  Precio  Importe │
│ ...                                           │
│                                  Página 2 de 3│
└───────────────────────────────────────────────┘
```

## Tabla de líneas

Ancho útil: 511 pt.

| Columna | Encabezado | Ancho | Alineación |
|---|---|---:|---|
| Concepto | `Concepto` | 235 pt | Izquierda |
| Cantidad | `Cantidad` | 56 pt | Derecha |
| Unidad | `Unidad` | 62 pt | Izquierda |
| Precio | `Precio sin IVA` | 76 pt | Derecha |
| Importe | `Importe` | 82 pt | Derecha |

## Reglas de maquetación

1. El logo cabe en un recuadro de 150 × 70 pt, conserva sus proporciones y nunca se amplía por encima de su tamaño natural.
2. Sin logo, los datos del negocio ocupan su lugar y no queda hueco (FR-019).
3. Los datos opcionales vacíos no generan renglones ni etiquetas (FR-012).
4. Los textos se cortan por espacios. Una palabra que no cabe se reparte por caracteres (FR-018).
5. Una línea de la tabla nunca se parte entre dos páginas (FR-017).
6. La cabecera de la tabla se repite en cada página con líneas (FR-017).
7. Los totales aparecen una sola vez, al final, y no se separan entre páginas.
8. Un importe que no cabe en su columna reduce su tamaño hasta 7 pt y, si aún no cabe, pasa a varios renglones.
9. Todas las páginas llevan `Página X de Y`.

## Formatos de los valores

| Valor | Formato | Ejemplo |
|---|---|---|
| Importe | Formato del borrador | `1.234,56 €` |
| Cantidad | Sin ceros sobrantes | `1,5` |
| Tipo de IVA | Sin ceros sobrantes | `IVA (21 %)` |
| Fecha | `dd/mm/aaaa` | `18/12/2026` |

## Lo que nunca aparece

Origen de los precios, marcas de pendiente o de revisión, texto transcrito y fragmentos del dictado (FR-011).
