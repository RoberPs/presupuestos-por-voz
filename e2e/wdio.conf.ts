// Pruebas extremo a extremo con WebdriverIO + tauri-driver (R10).
// Requisitos: `cargo install tauri-driver` y msedgedriver (misma versión que WebView2) en el PATH.
// `npm run test:e2e` compila la app en modo debug y ejecuta los recorridos de quickstart.md.
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const exe = process.platform === "win32" ? "presupuestos-por-voz.exe" : "presupuestos-por-voz";
const application = path.resolve(here, "../src-tauri/target/debug", exe);

// Carpeta de datos temporal y nueva en cada ejecución (lib.rs la respeta solo en builds debug).
const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), "presupuestos-e2e-"));
process.env.PRESUPUESTOS_DATA_DIR = dataDir;

let tauriDriver: ChildProcess | undefined;

export const config: WebdriverIO.Config = {
  runner: "local",
  host: "127.0.0.1",
  port: 4444,
  specs: [path.join(here, "specs/**/*.e2e.ts")],
  maxInstances: 1,
  capabilities: [
    {
      // @ts-expect-error capacidad propia de tauri-driver
      "tauri:options": { application },
    },
  ],
  logLevel: "warn",
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: { ui: "bdd", timeout: 60_000 },

  onPrepare: () => {
    if (!fs.existsSync(application)) {
      throw new Error(
        `No existe ${application}. Ejecuta antes: npm run tauri build -- --debug --no-bundle`,
      );
    }
  },
  beforeSession: () => {
    const driver = path.resolve(os.homedir(), ".cargo", "bin", "tauri-driver");
    tauriDriver = spawn(driver, [], {
      stdio: [null, process.stdout, process.stderr],
      env: { ...process.env, PRESUPUESTOS_DATA_DIR: dataDir },
    });
  },
  afterSession: () => {
    tauriDriver?.kill();
  },
  onComplete: () => {
    fs.rmSync(dataDir, { recursive: true, force: true });
  },
};
