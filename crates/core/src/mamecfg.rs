//! MAME keeps what the player changed in its own input menu in `<saves>/<core>/cfg/<game>.cfg`
//! and loads it on every start. A remap there (say Down moved to a joystick hat the app never
//! sends) wins over the app's RetroPad buttons, so the pad seems dead. These helpers drop the
//! remaps and leave everything else in the file (coin counters, sound settings) alone.

/// The text of a `.cfg` without its `<input>` blocks, or `None` when it has none.
pub fn without_input(cfg: &str) -> Option<String> {
    let mut out = String::with_capacity(cfg.len());
    let mut rest = cfg;
    let mut found = false;
    while let Some(start) = rest.find("<input") {
        // `<input>` or `<input ...>`, not some longer tag name that starts the same way
        let after = rest[start + 6..].chars().next();
        if !matches!(
            after,
            Some('>') | Some(' ') | Some('\n') | Some('\r') | Some('\t')
        ) {
            out.push_str(&rest[..start + 6]);
            rest = &rest[start + 6..];
            continue;
        }
        let Some(len) = rest[start..].find("</input>") else {
            break; // a broken file: leave it as it is
        };
        found = true;
        // take the whole line the block started on, with its indentation
        let line_start = rest[..start].rfind('\n').map_or(0, |i| i);
        out.push_str(&rest[..line_start]);
        rest = &rest[start + len + "</input>".len()..];
    }
    out.push_str(rest);
    found.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WITH: &str = "<mameconfig>\n    <system name=\"g\">\n        <counters>\n            <coins index=\"0\" number=\"87\" />\n        </counters>\n        <input>\n            <port tag=\":P1\" type=\"P1_JOYSTICK_DOWN\">\n                <newseq type=\"standard\">\n                    JOYCODE_1_HAT1UP\n                </newseq>\n            </port>\n        </input>\n        <mixer />\n    </system>\n</mameconfig>\n";

    #[test]
    fn drops_the_remaps_and_keeps_the_rest() {
        let out = without_input(WITH).unwrap();
        assert!(!out.contains("input") && !out.contains("HAT1UP"), "{out}");
        assert!(
            out.contains("number=\"87\"") && out.contains("<mixer />"),
            "{out}"
        );
        assert!(out.contains("</counters>\n        <mixer />"), "{out}");
    }

    #[test]
    fn a_file_without_remaps_is_not_touched() {
        assert_eq!(
            without_input("<mameconfig>\n  <system name=\"g\" />\n</mameconfig>\n"),
            None
        );
        assert_eq!(
            without_input("<a><inputs/></a>"),
            None,
            "another tag is not <input>"
        );
    }

    #[test]
    fn an_unclosed_block_is_left_alone() {
        assert_eq!(without_input("<a>\n<input>\n<port/>"), None);
    }

    #[test]
    fn more_than_one_block_all_go() {
        let two = "<a>\n  <input>x</input>\n  <b/>\n  <input type=\"1\">y</input>\n</a>";
        assert_eq!(without_input(two).unwrap(), "<a>\n  <b/>\n</a>");
    }
}
