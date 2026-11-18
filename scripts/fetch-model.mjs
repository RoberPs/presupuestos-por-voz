// Descarga los modelos de voz al preparar el entorno de desarrollo y verifica su SHA-256.
// La aplicación nunca descarga nada: el modelo elegido viaja dentro del instalador.
import { createHash } from "node:crypto";
import {
  createReadStream,
  createWriteStream,
  existsSync,
  mkdirSync,
  renameSync,
  rmSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const BASE_URL = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
const MODELS_DIR = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "src-tauri",
  "resources",
  "models",
);

// Hashes de specs/002-voice-quote-draft/research.md (R2).
const MODELS = [
  {
    file: "ggml-small-q5_1.bin",
    sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
  },
  {
    file: "ggml-base-q5_1.bin",
    sha256: "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
  },
];

async function sha256Of(path) {
  const hash = createHash("sha256");
  await pipeline(createReadStream(path), hash);
  return hash.digest("hex");
}

async function fetchModel({ file, sha256 }) {
  const target = join(MODELS_DIR, file);

  if (existsSync(target)) {
    if ((await sha256Of(target)) === sha256) {
      console.log(`${file}: ya está descargado y verificado.`);
      return;
    }
    console.warn(`${file}: el archivo existente no coincide con el hash; se descarga de nuevo.`);
    rmSync(target);
  }

  const partial = `${target}.part`;
  console.log(`${file}: descargando...`);
  const response = await fetch(`${BASE_URL}/${file}`, { redirect: "follow" });
  if (!response.ok || !response.body) {
    throw new Error(`${file}: la descarga falló con el estado ${response.status}.`);
  }
  await pipeline(Readable.fromWeb(response.body), createWriteStream(partial));

  const actual = await sha256Of(partial);
  if (actual !== sha256) {
    rmSync(partial);
    throw new Error(`${file}: hash incorrecto (esperado ${sha256}, obtenido ${actual}).`);
  }
  renameSync(partial, target);
  console.log(`${file}: descargado y verificado.`);
}

mkdirSync(MODELS_DIR, { recursive: true });
for (const model of MODELS) {
  await fetchModel(model);
}
