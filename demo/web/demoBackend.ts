// Backend de la demo web: atiende los comandos IPC de la interfaz con el núcleo Rust compilado a
// WebAssembly (demo/core). La interfaz es la misma que la de escritorio; solo cambia quién responde.
import { mockIPC } from "@tauri-apps/api/mocks";
import init, { invoke, logo_set, quote_generate } from "./pkg/presupuestos_demo_core.js";
import type { AppError, LogoSetResult, QuoteGenerateResult } from "../../src/shared/ipc/types";

const NO_VOICE: AppError = {
  kind: "microphone",
  code: "MIC_UNAVAILABLE",
  message:
    "En la demo web no se puede dictar ni importar audio: la transcripción funciona dentro de la app de escritorio.",
};

const UNKNOWN: AppError = {
  kind: "invalidState",
  message: "Esta acción solo está disponible en la app de escritorio.",
};

const DOWNLOADS = "Carpeta de descargas del navegador";

const pad = (n: number) => String(n).padStart(2, "0");

/** Fecha local del navegador, como la del equipo en la app (`aaaa-mm-dd`). */
function today(): string {
  const d = new Date();
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Los errores llegan de Rust como `AppError` en JSON. */
function appError(e: unknown): unknown {
  if (typeof e !== "string") return e;
  try {
    return JSON.parse(e);
  } catch {
    return e;
  }
}

function call(command: string, args: unknown): unknown {
  try {
    return JSON.parse(invoke(command, JSON.stringify(args ?? {}), today()));
  } catch (e) {
    throw appError(e);
  }
}

/** Selector de archivos del navegador en lugar del diálogo nativo. `null` si se cancela. */
function pickLogo(): Promise<LogoSetResult | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/png,image/jpeg";
    input.addEventListener("cancel", () => resolve(null));
    input.addEventListener("change", async () => {
      const file = input.files?.[0];
      if (!file) return resolve(null);
      try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        resolve(JSON.parse(logo_set(bytes)) as LogoSetResult);
      } catch (e) {
        reject(appError(e));
      }
    });
    input.click();
  });
}

/** Genera el PDF con el núcleo Rust y lo descarga en lugar de guardarlo con el diálogo nativo. */
function generatePdf(token: string, allowMissingLogo: boolean): QuoteGenerateResult {
  let pdf;
  try {
    pdf = quote_generate(token, allowMissingLogo, today());
  } catch (e) {
    throw appError(e);
  }
  const { fileName, pageCount } = pdf;
  const url = URL.createObjectURL(new Blob([pdf.bytes], { type: "application/pdf" }));
  pdf.free();
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
  return { outcome: "saved", fileName, fullPath: DOWNLOADS, pageCount };
}

type Args = Record<string, unknown>;

export async function startDemoBackend(): Promise<void> {
  await init();
  mockIPC((command, payload) => {
    const args = (payload ?? {}) as Args;
    switch (command) {
      case "logo_pick_and_set":
        return pickLogo();
      case "quote_generate":
        return generatePdf(String(args.confirmationToken), Boolean(args.allowMissingLogo));
      case "dictation_subscribe":
        return null;
      case "dictation_start":
      case "dictation_import":
      case "dictation_stop":
      case "dictation_process":
      case "dictation_recover":
        throw NO_VOICE;
      case "dictation_cancel_processing":
        return null;
      case "dictation_discard":
        return call("session_get", {});
      default:
        if (command.startsWith("plugin:")) throw UNKNOWN;
        return call(command, args);
    }
  });
}
