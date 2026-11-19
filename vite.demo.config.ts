// Demo web (GitHub Pages): la interfaz de la app con el núcleo Rust compilado a WebAssembly.
// Se construye con `npm run demo:build`; ver demo/README.md.
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  root: "demo/web",
  // Rutas relativas: GitHub Pages sirve la demo desde /presupuestos-por-voz/.
  base: "./",
  plugins: [react()],
  build: {
    target: "es2021",
    outDir: "../../dist-demo",
    emptyOutDir: true,
  },
  server: {
    port: 1430,
    strictPort: true,
    fs: { allow: ["../.."] },
  },
});
