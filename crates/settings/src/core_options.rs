//! Core option overrides: `key = value` lines in `<data dir>/config/<core>.cfg`,
//! written by the app and read by the runner. `#` and `;` start comment lines.

use std::collections::BTreeMap;

pub fn parse_config(text: &str) -> BTreeMap<String, String> {
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

/// A key or value that survives a round trip through the file format.
pub fn is_storable(s: &str) -> bool {
    !s.contains(['\n', '\r']) && s == s.trim()
}

pub fn render_config(options: &BTreeMap<String, String>) -> String {
    let mut out = String::from(
        "# vuretro core options (edited in the app). Delete a line to use the core default.\n",
    );
    for (k, v) in options {
        out.push_str(&format!("{k} = {v}\n"));
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
    fn render_then_parse_is_the_identity() {
        let mut m = BTreeMap::new();
        m.insert("a_b".to_string(), "1 x".to_string());
        m.insert("z".to_string(), "off".to_string());
        assert_eq!(parse_config(&render_config(&m)), m);
        assert!(is_storable("on") && !is_storable("a\nb") && !is_storable(" x"));
    }
}
