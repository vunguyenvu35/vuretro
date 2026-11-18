//! Cheat codes of a game: a text file with one code per line, shared by the app (which
//! edits it) and the runner (which hands the codes to the core).
//!
//! ```text
//! # infinite lives
//! 7E0DBE05
//! -7E0DBF09        <- a leading "-" keeps the code but switches it off
//! ```

#[derive(Debug, Clone, PartialEq)]
pub struct Cheat {
    pub enabled: bool,
    /// The code exactly as the core expects it (Game Genie, Action Replay, raw…).
    pub code: String,
}

/// Codes in file order. Blank lines and `#` comments are skipped; everything after ` #` on
/// a code line is a note.
pub fn parse(text: &str) -> Vec<Cheat> {
    text.lines()
        .filter_map(|line| {
            let line = line.split(" #").next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (enabled, code) = match line.strip_prefix('-') {
                Some(rest) => (false, rest.trim()),
                None => (true, line),
            };
            (!code.is_empty()).then(|| Cheat {
                enabled,
                code: code.to_string(),
            })
        })
        .collect()
}

/// Only the codes that are switched on.
pub fn active(text: &str) -> Vec<String> {
    parse(text)
        .into_iter()
        .filter(|c| c.enabled)
        .map(|c| c.code)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codes_notes_and_disabled_lines() {
        let text = "# lives\n7E0DBE05\n\n-7E0DBF09 # coins\n  SLXPDNAK+SLXPEKAK  \n-\n";
        let c = parse(text);
        assert_eq!(c.len(), 3);
        assert_eq!(
            c[0],
            Cheat {
                enabled: true,
                code: "7E0DBE05".into()
            }
        );
        assert_eq!(
            c[1],
            Cheat {
                enabled: false,
                code: "7E0DBF09".into()
            }
        );
        assert_eq!(c[2].code, "SLXPDNAK+SLXPEKAK");
        assert_eq!(active(text), vec!["7E0DBE05", "SLXPDNAK+SLXPEKAK"]);
        assert!(parse("").is_empty());
    }
}
