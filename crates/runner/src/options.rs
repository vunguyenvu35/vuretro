//! Core options: `key = value` lines from a per-core config file, answered to
//! the core's `GET_VARIABLE` calls. Lines starting with `#` or `;` are comments.

use std::collections::HashMap;

pub fn parse_config(text: &str) -> HashMap<String, String> {
    vuretro_settings::core_options::parse_config(text)
        .into_iter()
        .collect()
}

/// One option announced by the core via `SET_VARIABLES`.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionDef {
    pub key: String,
    pub description: String,
    /// Allowed values; the first one is the core's default.
    pub values: Vec<String>,
}

/// Parse a libretro variable definition: `"Description; a|b|c"`.
pub fn parse_definition(key: &str, value: &str) -> OptionDef {
    let (description, values) = value.split_once(';').unwrap_or((value, ""));
    OptionDef {
        key: key.to_string(),
        description: description.trim().to_string(),
        values: values
            .split('|')
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect(),
    }
}

/// Options the frontend sets itself unless the user chose otherwise: `(key, value)`. They are
/// what a game really runs with, so the app shows them as the default.
///
/// FBNeo's own default Neo Geo mode boots the MVS BIOS (a garbled green system check, and it
/// needs coins). The mode is picked from what `neogeo.zip` in `system_dir` contains, so the
/// BIOS the user has decides (see `neogeo::default_mode`).
pub fn frontend_defaults(core_stem: &str, system_dir: &std::path::Path) -> Vec<(String, String)> {
    if core_stem.starts_with("fbneo") {
        let mode = std::fs::read(system_dir.join("neogeo.zip"))
            .map(|zip| vuretro_play::neogeo::default_mode(&zip))
            .unwrap_or("AES_EUR");
        vec![("fbneo-neogeo-mode".to_string(), mode.to_string())]
    } else {
        Vec::new()
    }
}

/// Definitions as JSON for the app: `[{"key","description","values","frontend_default"?}]`.
pub fn to_json(defs: &[OptionDef], defaults: &[(String, String)]) -> String {
    let list: Vec<_> = defs
        .iter()
        .map(|d| {
            let mut o = serde_json::json!({ "key": d.key, "description": d.description, "values": d.values });
            if let Some((_, v)) = defaults.iter().find(|(k, _)| *k == d.key) {
                o["frontend_default"] = serde_json::json!(v);
            }
            o
        })
        .collect();
    serde_json::Value::Array(list).to_string()
}

/// Render definitions as a commented config template (defaults uncommented).
pub fn template(defs: &[OptionDef]) -> String {
    let mut out = String::from("# vuretro core options. Delete a line to use the core default.\n");
    for d in defs {
        out.push_str(&format!(
            "\n# {} (choices: {})\n{} = {}\n",
            d.description,
            d.values.join(", "),
            d.key,
            d.values.first().map(String::as_str).unwrap_or("")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_definition() {
        let d = parse_definition("k", "Frameskip; off|auto|manual");
        assert_eq!(d.description, "Frameskip");
        assert_eq!(d.values, ["off", "auto", "manual"]);
        assert!(template(std::slice::from_ref(&d)).contains("k = off"));
        assert_eq!(
            to_json(std::slice::from_ref(&d), &[]),
            r#"[{"description":"Frameskip","key":"k","values":["off","auto","manual"]}]"#
        );
        assert_eq!(
            to_json(
                std::slice::from_ref(&d),
                &[("k".to_string(), "auto".to_string())]
            ),
            r#"[{"description":"Frameskip","frontend_default":"auto","key":"k","values":["off","auto","manual"]}]"#
        );
        // no BIOS to look at: AES, as before
        let nowhere = std::path::Path::new("/nonexistent");
        assert_eq!(
            frontend_defaults("fbneo_libretro", nowhere),
            [("fbneo-neogeo-mode".to_string(), "AES_EUR".to_string())]
        );
        assert!(frontend_defaults("snes9x_libretro", nowhere).is_empty());
    }
}
