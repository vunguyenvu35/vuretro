//! Fighting-game move notation turned into the steps of a macro sequence, so one key can
//! play a whole special move or combo. Pure, so it is unit-tested without a game.
//!
//! Notation (the usual one of fighting games, as numpad directions):
//! - directions are numpad digits as seen facing right: `2` down, `6` forward, `4` back,
//!   `8` up, `236` is a quarter circle forward; named motions `QCF QCB DP RDP HCF HCB DQCF
//!   CHARGE_BACK CHARGE_DOWN` stand for their digits;
//! - buttons are the Neo Geo face buttons `A B C D` (`P` is a punch = A, `K` a kick = B);
//! - prefixes `j.` (jumping), `cr.` (crouching), `cl.` and `far.` (standing, no extra input);
//! - `,` links one move after the other, `>` cancels into the next one at once;
//! - `x3` repeats a move, `[4]6` holds back before forward (a charge), `A/B` picks the first
//!   alternative and `( )` marks something optional that is left out.
//!
//! The timings are an approximation of 60 frames per second play; fine-tune with the
//! steps editor if a game needs it.

use crate::macros::Step;
use crate::ACTIONS;

const FRAME_MS: f64 = 1000.0 / 60.0;
/// Frames each direction of a motion is held, and a button is pressed.
const DIR_FRAMES: u32 = 3;
const BTN_FRAMES: u32 = 3;
/// A jump: up is pressed, then the attack comes this many frames later; landing takes a while.
const JUMP_PRESS_FRAMES: u32 = 2;
const JUMP_AIR_FRAMES: u32 = 14;
const LAND_FRAMES: u32 = 24;
/// Pause before the next move: `>` cancels almost at once, `,` waits for the first to end.
const CANCEL_GAP_FRAMES: u32 = 2;
const LINK_GAP_FRAMES: u32 = 8;
/// How long a charge direction (`[4]`) is held.
const CHARGE_FRAMES: u32 = 40;
pub const MAX_REPEAT: u32 = 20;
pub const MAX_STEPS: usize = 200;

/// Named motions, longest first so `DQCF` is not read as `D` + `QCF`.
const MOTIONS: [(&str, &str); 9] = [
    ("CHARGE_BACK", "[4]6"),
    ("CHARGE_DOWN", "[2]8"),
    ("DQCF", "236236"),
    ("QCF", "236"),
    ("QCB", "214"),
    ("HCF", "41236"),
    ("HCB", "63214"),
    ("RDP", "421"),
    ("DP", "623"),
];

fn frames_ms(frames: u32) -> u32 {
    (f64::from(frames) * FRAME_MS).round() as u32
}

/// RetroPad id of a Neo Geo face button letter.
fn button(c: char) -> Option<&'static str> {
    Some(match c {
        'A' | 'P' => "b",
        'B' | 'K' => "a",
        'C' => "y",
        'D' => "x",
        _ => return None,
    })
}

/// RetroPad directions of a numpad digit; `left` mirrors it for a character facing left.
fn direction(digit: u8, left: bool) -> Vec<&'static str> {
    let digit = if left {
        match digit {
            1 => 3,
            3 => 1,
            4 => 6,
            6 => 4,
            7 => 9,
            9 => 7,
            d => d,
        }
    } else {
        digit
    };
    match digit {
        1 => vec!["down", "left"],
        2 => vec!["down"],
        3 => vec!["down", "right"],
        4 => vec!["left"],
        6 => vec!["right"],
        7 => vec!["up", "left"],
        8 => vec!["up"],
        9 => vec!["up", "right"],
        _ => vec![], // 5: neutral
    }
}

/// One thing held for a number of frames, before it becomes a macro step.
#[derive(Debug, Clone, PartialEq)]
struct Piece {
    actions: Vec<&'static str>,
    frames: u32,
    /// Made of directions only (a button may still be added to it).
    directions_only: bool,
}

impl Piece {
    fn wait(frames: u32) -> Self {
        Piece {
            actions: vec![],
            frames,
            directions_only: false,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Tok {
    Dirs(Vec<u8>),
    Hold(Vec<u8>),
    Buttons(Vec<&'static str>),
}

/// Split the body of a move (`236P`, `[4]6A`, `QCFB`) into direction runs, holds and buttons.
fn tokens(body: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = body.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '[' {
            let end = chars[i..]
                .iter()
                .position(|c| *c == ']')
                .ok_or("a [ has no closing ]")?;
            let digits = digits(&chars[i + 1..i + end])?;
            out.push(Tok::Hold(digits));
            i += end + 1;
        } else if c.is_ascii_digit() {
            let n = chars[i..].iter().take_while(|c| c.is_ascii_digit()).count();
            out.push(Tok::Dirs(digits(&chars[i..i + n])?));
            i += n;
        } else if c.is_ascii_alphabetic() {
            // letters are not case sensitive: `236p` and `qcfa` work too
            let rest: String = chars[i..].iter().collect::<String>().to_ascii_uppercase();
            if let Some((name, digits_of)) = MOTIONS.iter().find(|(n, _)| rest.starts_with(n)) {
                out.extend(tokens(digits_of)?);
                i += name.len();
                continue;
            }
            let mut pressed = Vec::new();
            while let Some(b) = chars.get(i).and_then(|c| button(c.to_ascii_uppercase())) {
                pressed.push(b);
                i += 1;
            }
            if pressed.is_empty() {
                let word: String = chars[i..]
                    .iter()
                    .take_while(|c| c.is_alphanumeric() || **c == '_')
                    .collect();
                return Err(format!(
                    "\"{word}\" is not a move I can play (use directions like 236 and the buttons A B C D)"
                ));
            }
            out.push(Tok::Buttons(pressed));
        } else {
            let word: String = chars[i..]
                .iter()
                .take_while(|c| c.is_alphanumeric() || **c == '_')
                .collect();
            let word = if word.is_empty() { c.to_string() } else { word };
            return Err(format!(
                "\"{word}\" is not a move I can play (use directions like 236 and the buttons A B C D)"
            ));
        }
    }
    Ok(out)
}

fn digits(chars: &[char]) -> Result<Vec<u8>, String> {
    chars
        .iter()
        .map(|c| match c.to_digit(10) {
            Some(d @ 1..=9) => Ok(d as u8),
            _ => Err(format!("\"{c}\" is not a direction (use 1 to 9)")),
        })
        .collect()
}

/// Remove `( optional )` parts.
fn drop_optional(src: &str) -> String {
    let mut depth = 0u32;
    src.chars()
        .filter(|c| match c {
            '(' => {
                depth += 1;
                false
            }
            ')' => {
                depth = depth.saturating_sub(1);
                false
            }
            _ => depth == 0,
        })
        .collect()
}

/// A trailing `x3` (repeat) of a move, and what is left of it.
fn split_repeat(seg: &str) -> Result<(&str, u32), String> {
    let seg = seg.trim_end();
    if let Some(pos) = seg.rfind('x') {
        let n = &seg[pos + 1..];
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            let n: u32 = n.parse().map_err(|_| "that repeat count is too big")?;
            if n == 0 || n > MAX_REPEAT {
                return Err(format!("a move can repeat 1 to {MAX_REPEAT} times"));
            }
            return Ok((seg[..pos].trim_end(), n));
        }
    }
    Ok((seg, 1))
}

/// Remove each `+` together with the spaces around it, so `C + D` reads as `CD`.
fn join_plus(body: &str) -> String {
    let mut out = String::new();
    let mut skip_space = false;
    for c in body.chars() {
        if c == '+' {
            out.truncate(out.trim_end().len());
            skip_space = true;
        } else if skip_space && c.is_whitespace() {
            continue;
        } else {
            skip_space = false;
            out.push(c);
        }
    }
    out
}

/// The pieces of one move (no connector), and whether it ends in the air.
fn segment(seg: &str, left: bool) -> Result<(Vec<Piece>, bool), String> {
    // `A/B` and `6A/6B`: the first alternative
    let seg = seg.split('/').next().unwrap_or("").trim();
    let (seg, repeat) = split_repeat(seg)?;
    let lower = seg.to_ascii_lowercase();
    let (prefix, body) = ["j.", "cr.", "cl.", "far.", "st."]
        .iter()
        .find(|p| lower.starts_with(**p))
        .map_or(("", seg), |p| (*p, &seg[p.len()..]));
    if body.trim().is_empty() {
        return Err(format!("\"{}\" has no input after it", seg.trim()));
    }
    // `A+B` and `A + B` are the same as `AB`: buttons pressed together
    let body = join_plus(body);
    let body = body.as_str();

    let mut pieces: Vec<Piece> = Vec::new();
    for tok in tokens(body)? {
        match tok {
            Tok::Hold(ds) => pieces.push(Piece {
                actions: ds.iter().flat_map(|d| direction(*d, left)).collect(),
                frames: CHARGE_FRAMES,
                directions_only: false, // a button never joins a charge
            }),
            Tok::Dirs(ds) => {
                for d in ds {
                    pieces.push(Piece {
                        actions: direction(d, left),
                        frames: DIR_FRAMES,
                        directions_only: true,
                    });
                }
            }
            Tok::Buttons(bs) => match pieces.last_mut() {
                // the buttons go down with the last direction: 236P ends on forward + P
                Some(last) if last.directions_only => {
                    last.actions.extend(bs);
                    last.frames = last.frames.max(BTN_FRAMES);
                    last.directions_only = false;
                }
                _ => pieces.push(Piece {
                    actions: bs,
                    frames: BTN_FRAMES,
                    directions_only: false,
                }),
            },
        }
    }
    let crouch = prefix == "cr.";
    let jump = prefix == "j.";
    if crouch {
        // crouching: down is held with the buttons (or the whole move if it has none)
        match pieces
            .iter_mut()
            .find(|p| !p.directions_only && !p.actions.is_empty())
        {
            Some(p) => p.actions.push("down"),
            None => {
                for p in &mut pieces {
                    p.actions.push("down");
                }
            }
        }
    }
    if repeat > 1 {
        let once = pieces.clone();
        for _ in 1..repeat {
            pieces.push(Piece::wait(LINK_GAP_FRAMES));
            pieces.extend(once.clone());
        }
    }
    if jump {
        let mut air = vec![
            Piece {
                actions: vec!["up"],
                frames: JUMP_PRESS_FRAMES,
                directions_only: false,
            },
            Piece::wait(JUMP_AIR_FRAMES),
        ];
        air.extend(pieces);
        pieces = air;
    }
    Ok((pieces, jump))
}

/// Order the buttons of a piece the way the Controls screen lists them, without repeats.
fn normalise(actions: &[&str]) -> Vec<String> {
    ACTIONS
        .iter()
        .filter(|a| actions.contains(a))
        .map(|a| a.to_string())
        .collect()
}

/// Turn a move or combo into macro steps. `left` is for a character that faces left:
/// forward and back swap.
pub fn compile(src: &str, left: bool) -> Result<Vec<Step>, String> {
    let src = drop_optional(src);
    if src.trim().is_empty() {
        return Err("type a move first, for example 236P".into());
    }
    // split on `,` (link) and `>` (cancel), remembering which one came before each move
    let mut segs: Vec<(u32, String)> = Vec::new();
    let mut gap = 0;
    let mut cur = String::new();
    for c in src.chars() {
        if c == ',' || c == '>' {
            segs.push((gap, std::mem::take(&mut cur)));
            gap = if c == '>' {
                CANCEL_GAP_FRAMES
            } else {
                LINK_GAP_FRAMES
            };
        } else {
            cur.push(c);
        }
    }
    segs.push((gap, cur));

    let mut pieces: Vec<Piece> = Vec::new();
    let last = segs.len() - 1;
    for (i, (gap, text)) in segs.iter().enumerate() {
        let (mut seg, in_air) = segment(text, left)?;
        if i > 0 {
            pieces.push(Piece::wait(*gap));
        }
        pieces.append(&mut seg);
        if in_air && i != last {
            pieces.push(Piece::wait(LAND_FRAMES)); // the next move starts on the ground
        }
    }
    let mut steps: Vec<Step> = Vec::new();
    for p in pieces {
        let step = Step {
            actions: normalise(&p.actions),
            ms: frames_ms(p.frames),
        };
        // two waits in a row are one longer wait
        match steps.last_mut() {
            Some(prev) if prev.actions.is_empty() && step.actions.is_empty() => prev.ms += step.ms,
            _ => steps.push(step),
        }
    }
    while steps.last().is_some_and(|s| s.actions.is_empty()) {
        steps.pop(); // nothing to wait for at the end
    }
    if steps.len() > MAX_STEPS {
        return Err(format!(
            "that is {} steps; the most is {MAX_STEPS}",
            steps.len()
        ));
    }
    Ok(steps)
}

/// Total playing time of steps, in milliseconds.
pub fn total_ms(steps: &[Step]) -> u32 {
    steps.iter().map(|s| s.ms).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(steps: &[Step]) -> Vec<String> {
        steps
            .iter()
            .map(|s| format!("{}@{}", s.actions.join("+"), s.ms))
            .collect()
    }
    fn acts(src: &str) -> Vec<String> {
        compile(src, false)
            .unwrap()
            .iter()
            .map(|s| s.actions.join("+"))
            .collect()
    }

    #[test]
    fn a_quarter_circle_forward_ends_on_forward_with_the_button() {
        assert_eq!(acts("236P"), ["down", "down+right", "right+b"]);
        assert_eq!(
            show(&compile("236P", false).unwrap()),
            ["down@50", "down+right@50", "right+b@50"]
        );
    }

    #[test]
    fn neo_geo_buttons_map_to_the_retropad_ones() {
        assert_eq!(acts("A"), ["b"]);
        assert_eq!(acts("B"), ["a"]);
        assert_eq!(acts("C"), ["y"]);
        assert_eq!(acts("D"), ["x"]);
        assert_eq!(acts("P"), ["b"], "a punch is the light one");
        assert_eq!(acts("K"), ["a"], "a kick is the light one");
        assert_eq!(acts("AC"), ["b+y"], "buttons pressed together");
        assert_eq!(acts("A+B"), ["b+a"], "written with a plus");
        assert_eq!(acts("C + D"), ["y+x"]);
        assert_eq!(acts("6A+B"), ["right+b+a"]);
    }

    #[test]
    fn named_motions_are_their_digits() {
        assert_eq!(acts("QCFP"), acts("236P"));
        assert_eq!(acts("DP"), acts("623"));
        assert_eq!(acts("623"), ["right", "down", "down+right"]);
        assert_eq!(acts("QCBD"), acts("214D"));
        assert_eq!(acts("HCFA"), acts("41236A"));
        assert_eq!(acts("DQCFC").len(), 6);
        assert_eq!(acts("RDP"), acts("421"));
    }

    #[test]
    fn a_character_facing_left_has_forward_and_back_swapped() {
        let left: Vec<String> = compile("236P", true)
            .unwrap()
            .iter()
            .map(|s| s.actions.join("+"))
            .collect();
        assert_eq!(left, ["down", "down+left", "left+b"]);
        assert_eq!(compile("4", true).unwrap()[0].actions, ["right"]);
        assert_eq!(compile("9", true).unwrap()[0].actions, ["up", "left"]);
    }

    #[test]
    fn a_charge_holds_back_for_a_while_then_goes_forward() {
        let s = compile("[4]6A", false).unwrap();
        assert_eq!(show(&s), ["left@667", "right+b@50"]);
        assert_eq!(acts("CHARGE_BACKP"), acts("[4]6P"));
    }

    #[test]
    fn crouching_holds_down_with_the_button() {
        assert_eq!(acts("cr.B"), ["down+a"]);
        assert_eq!(
            acts("CR.b"),
            ["down+a"],
            "prefixes and their case are forgiving"
        );
        assert_eq!(acts("236p"), acts("236P"));
        assert_eq!(acts("qcfa"), acts("236A"));
        assert_eq!(acts("cl.C"), ["y"], "standing adds nothing");
        assert_eq!(acts("far.A"), ["b"]);
    }

    #[test]
    fn jumping_presses_up_waits_in_the_air_then_attacks() {
        assert_eq!(acts("j.D"), ["up", "", "x"]);
        let s = compile("j.D", false).unwrap();
        assert_eq!(s[0].ms, 33);
        assert_eq!(s[1].ms, 233, "air time");
    }

    #[test]
    fn a_link_waits_and_a_cancel_does_not() {
        let link = compile("cr.A, cr.B", false).unwrap();
        let cancel = compile("cr.A > cr.B", false).unwrap();
        assert_eq!(show(&link), ["down+b@50", "@133", "down+a@50"]);
        assert_eq!(show(&cancel), ["down+b@50", "@33", "down+a@50"]);
    }

    #[test]
    fn a_whole_combo_chains_its_moves() {
        let s = acts("j.D, cl.C > 236P");
        assert_eq!(
            s,
            ["up", "", "x", "", "y", "", "down", "down+right", "right+b"]
        );
        // after the jump-in the landing and the link are one wait
        let steps = compile("j.D, cl.C > 236P", false).unwrap();
        assert_eq!(steps[3].ms, frames_ms(LAND_FRAMES + LINK_GAP_FRAMES));
    }

    #[test]
    fn repeats_alternatives_and_optional_parts() {
        assert_eq!(acts("cr.A x3"), ["down+b", "", "down+b", "", "down+b"]);
        assert_eq!(acts("6A/6B"), acts("6A"), "the first alternative");
        assert_eq!(acts("j.C/D"), acts("j.C"));
        assert_eq!(acts("cl.C > (6A >) 236P"), acts("cl.C > 236P"));
        assert!(compile("cr.A x99", false).is_err());
        assert!(compile("cr.A x0", false).is_err());
    }

    #[test]
    fn nothing_waits_at_the_end() {
        let s = compile("j.D", false).unwrap();
        assert!(!s.last().unwrap().actions.is_empty());
    }

    #[test]
    fn what_cannot_be_played_says_so() {
        assert!(compile("", false).unwrap_err().contains("type a move"));
        assert!(compile("   ", false).unwrap_err().contains("type a move"));
        // the generic patterns of the template have placeholders for real inputs
        let e = compile("cl.C > DM", false).unwrap_err();
        assert!(e.contains("\"M\""), "{e}");
        let e = compile("cl.C > special", false).unwrap_err();
        assert!(e.contains("special"), "{e}");
        assert!(compile("236Z", false).is_err());
        assert!(compile("[4 6", false).is_err());
        assert!(compile("0A", false)
            .unwrap_err()
            .contains("not a direction"));
        assert!(compile("j.", false).unwrap_err().contains("no input"));
    }

    #[test]
    fn a_huge_combo_is_refused_not_played_for_ever() {
        let huge = vec!["236236236P"; 60].join(", ");
        assert!(compile(&huge, false).unwrap_err().contains("steps"));
    }

    #[test]
    fn total_time_adds_up() {
        assert_eq!(total_ms(&compile("236P", false).unwrap()), 150);
    }
}
