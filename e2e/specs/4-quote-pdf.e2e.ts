// Recorrido hasta la confirmación del PDF (specs/003-quote-pdf-export/quickstart.md, escenarios 1 y 2).
// Continúa el presupuesto revisado de 3-voice-draft.e2e.ts, con el perfil de 1-business-profile.e2e.ts.
// El diálogo nativo de guardado no se puede automatizar: se valida a mano.
import { $, expect } from "@wdio/globals";
import { click, clickInDialog, fill, goTo, text } from "../helpers";

const CONFIRM = "Confirmar y elegir dónde guardar";

describe("PDF del presupuesto", () => {
  before(async () => {
    await goTo("Presupuesto");
    await click("Generar PDF");
  });

  it("1 · la confirmación resume lo que se va a emitir", async () => {
    await expect(await text("229,90 €")).toBeDisplayed();
    const number = await $('//*[@role="dialog"]//label[contains(., "Número del presupuesto")]');
    await number.waitForExist();
  });

  it("2 · sin el nombre del cliente indica que es obligatorio", async () => {
    await clickInDialog(CONFIRM);
    await expect(await text("Escribe el nombre del cliente.")).toBeDisplayed();
  });

  it("1 · con el nombre del cliente desaparece el bloqueo", async () => {
    await fill("Nombre del cliente", "Ana Ruiz");
    await fill("Teléfono del cliente", "611 222 333");
    const blocker = await text("Escribe el nombre del cliente.");
    await blocker.waitForExist({ reverse: true });
  });

  it("1 · cancelar cierra la confirmación sin generar nada", async () => {
    await clickInDialog("Cancelar");
    const dialog = await $('//*[@role="dialog"]');
    await dialog.waitForExist({ reverse: true });
  });
});
