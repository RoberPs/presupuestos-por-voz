// Marca el código compartido con la app como compilado para la demo (sin SQLite).
fn main() {
    println!("cargo::rustc-check-cfg=cfg(demo_web)");
    println!("cargo::rustc-cfg=demo_web");
}
