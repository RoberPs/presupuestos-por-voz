// Recorrido del texto escrito hasta dar el presupuesto por bueno (US5, US2 y escenario 6 de
// specs/002-voice-quote-draft/quickstart.md). El micrófono se valida a mano.
import { $, browser, expect } from "@wdio/globals";
import { click, fill, goTo, text } from "../helpers";

const EXAMPLE = "cambiar bajante, 3 metros, mano de obra 2 horas, cambiar grifo, 2 unidades";

const rateRow = (concept: string) => $(`//tr[td[normalize-space(.)="${concept}"]]`);
const lineField = (label: string) => $(`[aria-label="${label}"]`);

async function createRate(concept: string, unit: string, price: string) {
  await click("Nueva tarifa");
  await fill("Concepto", concept);
  await fill("Unidad", unit);
  await fill("Precio", price);
  await click("Guardar");
  await (await rateRow(concept)).waitForExist();
}

async function editLine(label: string, value: string) {
  const field = await lineField(label);
  await field.waitForExist();
  await field.clearValue();
  await field.setValue(value);
  await browser.keys("Tab");
}

describe("Presupuesto", () => {
  before(async () => {
    await goTo("Tarifas");
    await createRate("Bajante", "metro", "25");
    await createRate("Mano de obra", "hora", "30");
    await goTo("Presupuesto");
  });

  it("6 · pegar el texto da las líneas y los totales del ejemplo", async () => {
    await click("Escribir la visita");
    await fill("Lo que has contado en la visita", EXAMPLE);
    await click("Generar presupuesto");

    await expect(await text("135,00 €")).toBeDisplayed();
    await expect(await text("28,35 €")).toBeDisplayed();
    await expect(await text("163,35 €")).toBeDisplayed();
    await expect(await text("Precio pendiente")).toBeDisplayed();
    await expect(await text("Total provisional: falta 1 línea por completar.")).toBeDisplayed();
  });

  it("2 · cambiar una cantidad recalcula los importes al momento", async () => {
    await editLine("Cantidad de Bajante", "4");
    await expect(await text("100,00 €")).toBeDisplayed();
    await expect(await text("193,60 €")).toBeDisplayed();
  });

  it("11 · no se puede dar por bueno hasta completar la línea pendiente", async () => {
    await click("Dar por bueno");
    await expect(await text("Falta completar esta línea")).toBeDisplayed();

    await editLine("Precio de cambiar grifo", "15");
    await expect(await text("229,90 €")).toBeDisplayed();

    await click("Dar por bueno");
    await expect(await text("Presupuesto revisado")).toBeDisplayed();
  });
});
