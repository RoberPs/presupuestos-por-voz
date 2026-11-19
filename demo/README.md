# Demo web

Versión para el navegador de Presupuestos por Voz, publicada en GitHub Pages:
**https://roberps.github.io/presupuestos-por-voz/**

## Qué es

La misma interfaz React de la app de escritorio, conectada a un núcleo Rust compilado a
WebAssembly. No hay servidor: todo se ejecuta en el navegador.

```text
Interfaz React (src/)  ──invoke()──▶  demo/web/demoBackend.ts  ──▶  demo/core (WebAssembly)
   misma que escritorio     mockIPC de Tauri     selector de archivos       │
                                                 y descarga del PDF         ├─ src-tauri/src/domain  ┐ mismo código
                                                                            ├─ src-tauri/src/pdf     │ que la app,
                                                                            ├─ src-tauri/src/error.rs┘ sin copiar
                                                                            └─ store.rs: datos en memoria
```

- **Código compartido, no copiado.** `demo/core` incluye con `#[path]` los módulos `domain`, `pdf` y
  `error.rs` de la app. El intérprete del texto, los importes, la numeración, los bloqueos, la
  maquetación y el PDF/A son exactamente los mismos. Sus pruebas unitarias también se ejecutan aquí.
- **Lo único distinto es el almacenamiento.** `store.rs` reproduce los servicios de la app sobre
  datos en memoria en lugar de SQLite. Los datos se pierden al cerrar la pestaña.
- **Sin voz.** whisper.cpp y su modelo de 190 MB no tienen sentido en una página web; dictar e
  importar audio muestran un aviso. Se puede escribir la visita.
- **El logo** se elige con el selector de archivos del navegador y **el PDF** se descarga.

La app de escritorio no cambia: solo `error.rs` excluye la conversión de errores de SQLite cuando lo
compila la demo (`cfg(demo_web)`, activado en `demo/core/build.rs`).

## Construir y probar en local

Requisitos, además de los del proyecto:

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # misma versión que demo/core/Cargo.lock
```

```powershell
cargo test --manifest-path demo/core/Cargo.toml   # recorrido completo sin navegador
npm run demo:build                                # WebAssembly + interfaz en dist-demo/
npm run demo:preview                              # http://localhost:4173
```

`npm run demo:dev` sirve la demo con recarga en caliente (necesita haber ejecutado antes
`npm run demo:wasm`).

## Publicación

`.github/workflows/demo.yml` ejecuta las pruebas, construye la demo y la publica en GitHub Pages en
cada cambio de `main` que afecte a la interfaz, al dominio, al PDF o a la propia demo.
