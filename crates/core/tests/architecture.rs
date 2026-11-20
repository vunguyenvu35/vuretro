//! Guards the dependency rule: inner crates never depend on outer ones.

use std::path::Path;

/// Names of the `[dependencies]` of a workspace crate, read from its Cargo.toml.
fn deps(krate: &str) -> Vec<String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(krate)
        .join("Cargo.toml");
    let text = std::fs::read_to_string(manifest).unwrap();
    let mut in_deps = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]";
        } else if in_deps && !line.is_empty() && !line.starts_with('#') {
            if let Some((name, _)) = line.split_once('=') {
                out.push(name.trim().trim_end_matches(".workspace").to_string());
            }
        }
    }
    out
}

fn assert_only(krate: &str, allowed: &[&str]) {
    for d in deps(krate) {
        assert!(
            allowed.contains(&d.as_str()),
            "{krate} must not depend on `{d}` (allowed: {allowed:?})"
        );
    }
}

#[test]
fn core_is_pure() {
    // No UI, windowing, audio, process or filesystem-walking crates in the innermost layer.
    assert_only(
        "core",
        &[
            "serde",
            "serde_json", // parses the embedded systems registry (no I/O)
            "vuretro-controls",
            "vuretro-settings",
        ],
    );
}

#[test]
fn shared_config_crates_stay_leaf_crates() {
    assert_only("controls", &["serde", "serde_json"]);
    assert_only("settings", &["serde", "serde_json"]);
}

#[test]
fn library_adapters_only_know_the_core_and_config_crates() {
    for d in deps("library") {
        assert!(
            !d.starts_with("vuretro-")
                || ["vuretro-core", "vuretro-controls", "vuretro-settings"].contains(&d.as_str()),
            "library may depend on core and the config crates only, found {d}"
        );
    }
}
