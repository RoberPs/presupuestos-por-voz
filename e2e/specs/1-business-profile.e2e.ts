// Escenarios 1–3 y 10 de quickstart.md (el logo usa el diálogo nativo: se valida a mano).
import { expect } from "@wdio/globals";
import { click, fill, inputByLabel, restartApp, text } from "../helpers";

describe("Mi negocio", () => {
  it("1 · primer arranque: abre la configuración guiada", async () => {
    await expect(await text("Bienvenido")).toBeDisplayed();
  });

  it("2 · al saltarla aparece el aviso permanente de perfil incompleto", async () => {
    await click("Saltar por ahora");
    await expect(await text("Aún no has rellenado los datos de tu negocio")).toBeDisplayed();
    await expect(await text("Completar ahora")).toBeDisplayed();
  });

  it("3 · con los datos completos el perfil pasa a completo y desaparece el aviso", async () => {
    await fill("Nombre comercial", "Fontanería López");
    await fill("NIF / CIF", "12345678Z");
    await fill("Teléfono", "600 123 456");
    await fill("Calle y número", "Calle Mayor 1");
    await fill("Código postal", "28001");
    await fill("Localidad", "Madrid");
    await fill("Provincia", "Madrid");
    await click("Guardar");
    await expect(await text("Perfil completo")).toBeDisplayed();
    await expect(await text("Tu perfil está incompleto")).not.toBeExisting();
  });

  it("10 · al cerrar y reabrir la app los datos siguen igual", async () => {
    await restartApp();
    await expect(await inputByLabel("Nombre comercial")).toHaveValue("Fontanería López");
    await expect(await inputByLabel("NIF / CIF")).toHaveValue("12345678Z");
    await expect(await inputByLabel("IVA general")).toHaveValue("21,00");
    await expect(await text("Perfil completo")).toBeDisplayed();
  });
});
