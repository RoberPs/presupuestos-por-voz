// Compila el núcleo de la demo (demo/core) a WebAssembly y genera el enlace JavaScript.
// Requiere `rustup target add wasm32-unknown-unknown` y `wasm-bindgen-cli` con la misma versión
// que la dependencia `wasm-bindgen` de demo/core/Cargo.lock.
import { execFileSync } from "node:child_process";

const targetDir = process.env.CARGO_TARGET_DIR || "demo/core/target";
const run = (command, args) => execFileSync(command, args, { stdio: "inherit" });

run("cargo", [
  "build",
  "--manifest-path",
  "demo/core/Cargo.toml",
  "--target",
  "wasm32-unknown-unknown",
  "--release",
  "--target-dir",
  targetDir,
]);
run("wasm-bindgen", [
  `${targetDir}/wasm32-unknown-unknown/release/presupuestos_demo_core.wasm`,
  "--target",
  "web",
  "--out-dir",
  "demo/web/pkg",
  // Los nombres de las funciones solo sirven para depurar y aumentan mucho la descarga.
  "--remove-name-section",
  "--remove-producers-section",
]);
