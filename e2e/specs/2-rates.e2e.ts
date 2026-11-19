// Escenarios 11, 12, 17, 18 y 19 de quickstart.md.
import { $, expect } from "@wdio/globals";
import { button, click, clickInDialog, fill, goTo, text } from "../helpers";

const row = (concept: string) => $(`//tr[td[normalize-space(.)="${concept}"]]`);

async function createRate(concept: string, unit: string, price: string) {
  await click("Nueva tarifa");
  await fill("Concepto", concept);
  await fill("Unidad", unit);
  await fill("Precio", price);
  await click("Guardar");
  await (await row(concept)).waitForExist();
}

describe("Tarifas", () => {
  before(async () => {
    await goTo("Tarifas");
  });

  it("11 · sin tarifas: mensaje de lista vacía y botón para crear la primera", async () => {
    await expect(await text("Aún no tienes tarifas")).toBeDisplayed();
    await expect(await button("Crear primera tarifa")).toBeDisplayed();
  });

  it("12 · crear una tarifa la muestra con su precio y el aviso de precios sin IVA", async () => {
    await click("Crear primera tarifa");
    await fill("Concepto", "Instalación de grifo");
    await fill("Unidad", "unidad");
    await fill("Precio", "45,50");
    await click("Guardar");
    await expect(await row("Instalación de grifo")).toHaveText(expect.stringContaining("45,50 €"));
    await expect(await text("Los precios se guardan sin IVA")).toBeDisplayed();
  });

  it("17 · editar el precio actualiza la lista", async () => {
    await createRate("Pintura", "hora", "25");
    await click("Editar Pintura");
    await fill("Precio", "30");
    await click("Guardar");
    await expect(await row("Pintura")).toHaveText(expect.stringContaining("30,00 €"));
  });

  it("18 · borrar: cancelar la mantiene y confirmar la quita", async () => {
    await createRate("Alicatado", "m²", "20");
    await click("Borrar Alicatado");
    await clickInDialog("Cancelar");
    await expect(await row("Alicatado")).toBeExisting();
    await click("Borrar Alicatado");
    await clickInDialog("Borrar");
    await expect(await row("Alicatado")).not.toBeExisting();
  });

  it("19 · la búsqueda ignora mayúsculas y tildes", async () => {
    await createRate("Cambio de grifo", "unidad", "30");
    await fill("Buscar tarifa", "GRIFO");
    await expect(await row("Pintura")).not.toBeExisting();
    await expect(await row("Instalación de grifo")).toBeExisting();
    await expect(await row("Cambio de grifo")).toBeExisting();
    await fill("Buscar tarifa", "instalacion");
    await expect(await row("Cambio de grifo")).not.toBeExisting();
    await expect(await row("Instalación de grifo")).toBeExisting();
  });
});
