// Pruebas extremo a extremo con WebdriverIO + tauri-driver (R10).
// Requisitos: `cargo install tauri-driver` y msedgedriver (misma versión que WebView2), en el PATH
// o indicado con la variable MSEDGEDRIVER_PATH.
// `npm run test:e2e` compila la app en modo debug y ejecuta los recorridos de quickstart.md.
import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const exe = process.platform === "win32" ? "presupuestos-por-voz.exe" : "presupuestos-por-voz";
// La carpeta de compilación se define en .cargo/config.toml (build.target-dir).
const targetDir = process.env.CARGO_TARGET_DIR ?? "C:/vp-target";
const application = path.resolve(targetDir, "debug", exe);

// Carpeta de datos temporal y nueva en cada ejecución (lib.rs la respeta solo en builds debug).
const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), "presupuestos-e2e-"));
process.env.PRESUPUESTOS_DATA_DIR = dataDir;

let tauriDriver: ChildProcess | undefined;

/** Espera a que tauri-driver acepte conexiones antes de pedir la sesión. */
async function waitForPort(port: number, timeoutMs: number) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const open = await new Promise<boolean>((resolve) => {
      const socket = net.connect(port, "127.0.0.1");
      socket.once("connect", () => {
        socket.destroy();
        resolve(true);
      });
      socket.once("error", () => resolve(false));
    });
    if (open) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`tauri-driver no responde en el puerto ${port}`);
}

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
  beforeSession: async () => {
    const bin = process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver";
    const driver = path.resolve(os.homedir(), ".cargo", "bin", bin);
    // MSEDGEDRIVER_PATH: ruta a msedgedriver.exe si no está en el PATH.
    const nativeDriver = process.env.MSEDGEDRIVER_PATH;
    const args = nativeDriver ? ["--native-driver", nativeDriver] : [];
    tauriDriver = spawn(driver, args, {
      stdio: [null, process.stdout, process.stderr],
      env: { ...process.env, PRESUPUESTOS_DATA_DIR: dataDir },
    });
    await waitForPort(4444, 15_000);
  },
  afterSession: () => {
    tauriDriver?.kill();
  },
  onComplete: () => {
    fs.rmSync(dataDir, { recursive: true, force: true });
  },
};
