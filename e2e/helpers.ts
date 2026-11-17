import { $, browser } from "@wdio/globals";

/** Campo por el texto de su etiqueta (los id de React contienen ":"; se buscan por atributo). */
export async function inputByLabel(label: string) {
  const labelEl = await $(`//label[contains(normalize-space(.), "${label}")]`);
  await labelEl.waitForExist();
  const id = await labelEl.getAttribute("for");
  return $(`[id="${id}"]`);
}

export async function fill(label: string, value: string) {
  const input = await inputByLabel(label);
  await input.clearValue();
  await input.setValue(value);
}

export function button(name: string) {
  return $(`//button[normalize-space(.)="${name}" or @aria-label="${name}"]`);
}

export function text(contains: string) {
  return $(`//*[contains(normalize-space(text()), "${contains}")]`);
}

export async function click(name: string) {
  const b = await button(name);
  await b.waitForClickable();
  await b.click();
}

export async function goTo(section: "Mi negocio" | "Tarifas") {
  const link = await $(`//nav//a[normalize-space(.)="${section}"]`);
  await link.click();
}

/** Cierra y vuelve a abrir la app (nueva sesión de tauri-driver con la misma carpeta de datos). */
export async function restartApp() {
  await browser.reloadSession();
}
