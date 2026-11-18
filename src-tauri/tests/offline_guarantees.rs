//! Garantías sin conexión comprobadas sobre la configuración (FR-037 y FR-038).

use std::path::Path;

fn read(relative: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

fn json(relative: &str) -> serde_json::Value {
    serde_json::from_str(&read(relative)).unwrap()
}

#[test]
fn the_csp_has_no_remote_origins() {
    let config = json("tauri.conf.json");
    let csp = config["app"]["security"]["csp"].as_str().unwrap();
    for source in csp.split([';', ' ']).filter(|part| part.contains("://")) {
        assert_eq!(
            source, "http://ipc.localhost",
            "origen remoto en la CSP: {source}"
        );
    }
    assert!(!csp.contains('*'), "la CSP no admite comodines");
}

#[test]
fn capabilities_stay_minimal() {
    let capabilities = json("capabilities/default.json");
    let permissions: Vec<&str> = capabilities["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|permission| permission.as_str().unwrap())
        .collect();
    assert_eq!(
        permissions,
        ["core:default", "dialog:allow-open", "log:default"]
    );
}

#[test]
fn no_network_clients_or_plugins_are_declared() {
    const FORBIDDEN: [&str; 12] = [
        "reqwest",
        "hyper",
        "ureq",
        "surf",
        "isahc",
        "attohttpc",
        "curl",
        "tungstenite",
        "tauri-plugin-http",
        "tauri-plugin-updater",
        "tauri-plugin-websocket",
        "tauri-plugin-upload",
    ];
    let manifest = read("Cargo.toml");
    let declared: Vec<&str> = manifest
        .lines()
        .filter_map(|line| line.split('=').next())
        .map(str::trim)
        .collect();
    for name in FORBIDDEN {
        assert!(!declared.contains(&name), "dependencia de red: {name}");
    }
}

#[test]
fn the_voice_model_travels_inside_the_installer() {
    let config = json("tauri.conf.json");
    let resources: Vec<&str> = config["bundle"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|resource| resource.as_str().unwrap())
        .collect();
    assert!(resources.contains(&"resources/models/ggml-small-q5_1.bin"));
}

#[test]
fn the_app_never_downloads_the_model() {
    for file in ["src/lib.rs", "src/transcription/whisper.rs"] {
        let source = read(file);
        assert!(!source.contains("http://"), "{file} contiene una dirección");
        assert!(
            !source.contains("https://"),
            "{file} contiene una dirección"
        );
    }
}
