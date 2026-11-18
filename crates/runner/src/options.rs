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

/// Definitions as JSON for the app: `[{"key","description","values"}]`.
pub fn to_json(defs: &[OptionDef]) -> String {
    let list: Vec<_> = defs
        .iter()
        .map(|d| serde_json::json!({ "key": d.key, "description": d.description, "values": d.values }))
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
            to_json(&[d]),
            r#"[{"description":"Frameskip","key":"k","values":["off","auto","manual"]}]"#
        );
    }
}
