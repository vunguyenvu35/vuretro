//! Core options: `key = value` lines from a per-core config file, answered to
//! the core's `GET_VARIABLE` calls. Lines starting with `#` or `;` are comments.

use std::collections::HashMap;

/// One option announced by the core via `SET_VARIABLES`.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionDef {
    pub key: String,
    pub description: String,
    /// Allowed values; the first one is the core's default.
    pub values: Vec<String>,
}

pub fn parse_config(text: &str) -> HashMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with(';'))
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            let (k, v) = (k.trim(), v.trim().trim_matches('"'));
            (!k.is_empty()).then(|| (k.to_string(), v.to_string()))
        })
        .collect()
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
    fn parses_config() {
        let cfg = parse_config("# c\n\nfoo = bar\n ; x\nsnes9x_region= \"pal\" \nbad line\n=x\n");
        assert_eq!(cfg.len(), 2);
        assert_eq!(cfg["foo"], "bar");
        assert_eq!(cfg["snes9x_region"], "pal");
    }

    #[test]
    fn parses_definition() {
        let d = parse_definition("k", "Frameskip; off|auto|manual");
        assert_eq!(d.description, "Frameskip");
        assert_eq!(d.values, ["off", "auto", "manual"]);
        assert!(template(&[d]).contains("k = off"));
    }
}
