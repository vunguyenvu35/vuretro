//! What is known about Windows games: the recipes in `data/pc_games.json`. A recipe says how to
//! recognise the disc of a game and what it needs to run, found by running it: the folder of the
//! disc that is the game, a stand-in for a driver, entries of the registry, the title of its
//! window. A disc that no recipe knows is tried as it is (see [`generic`]).
//!
//! To add a game, add its entry to the data file: no code changes.

use super::wine_installer::resolve_ci;
use crate::iso::Iso;
use serde::Deserialize;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Stub {
    /// The file to put in the game's folder.
    pub file: String,
    /// Which stand-in: `aweman32` (the manager of the Sound Blaster AWE32, which answers that
    /// there is no card).
    pub kind: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RegValue {
    pub name: String,
    /// `REG_SZ` or `REG_DWORD`.
    #[serde(rename = "type")]
    pub kind: String,
    /// The data; `{install}` stands for the folder that the game was installed in.
    pub data: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RegKey {
    /// The key under the hive.
    pub key: String,
    /// `HKCU` and/or `HKLM`.
    pub hives: Vec<String>,
    /// For a program of 32 bits: its view of the registry.
    #[serde(default)]
    pub view32: bool,
    pub values: Vec<RegValue>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Title {
    /// The title the program pushes, and what it becomes. A letter above 127 is one byte of the
    /// Windows code page 1252 (the program's text is not Unicode).
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    /// Changes when what is done to the game changes, so that a game set up with an older
    /// recipe is set up again where it matters.
    #[serde(default)]
    pub revision: u32,
    /// Files of the disc, any of which says this is the game.
    #[serde(default)]
    pub detect: Vec<String>,
    /// The folder of the disc that holds the game (the whole disc when it is not there).
    #[serde(default)]
    pub root: Option<String>,
    /// The name under which that folder shows up on the CD drive (`D:\<this>`).
    #[serde(default)]
    pub drive_folder: Option<String>,
    /// The volume label of the CD.
    #[serde(default)]
    pub label: Option<String>,
    /// The folder on `C:` that the game says it is installed in (its id when it is not given).
    #[serde(default)]
    pub install_dir: Option<String>,
    /// The program to start, a path on the CD drive; found on the disc when it is not given.
    #[serde(default)]
    pub program: Option<String>,
    /// The screen the game asks for, `640x480`.
    #[serde(default = "default_screen")]
    pub screen: String,
    #[serde(default)]
    pub stubs: Vec<Stub>,
    #[serde(default)]
    pub registry: Vec<RegKey>,
    #[serde(default)]
    pub title: Option<Title>,
    /// The program of the disc that installs the game, a path on the CD drive; found by its name
    /// (`SETUP.EXE`, `INSTALL.EXE`...) when it is not given.
    #[serde(default)]
    pub installer: Option<String>,
    /// Verbs of `winetricks` to run in the game's Windows (`vcrun2015`, `corefonts`...): the
    /// libraries and fonts that the game needs and Wine does not have.
    #[serde(default)]
    pub winetricks: Vec<String>,
    /// Entries for `WINEDLLOVERRIDES` (`quartz=n,b`): for a library that the game brings in its
    /// own folder and must be used instead of Wine's.
    #[serde(default)]
    pub overrides: Vec<String>,
    /// What draws Direct3D: `wined3d` (Wine's own, for Direct3D up to 9) or `dxvk` (Direct3D 10
    /// and 11 over Vulkan, which Wine's own does not do well).
    #[serde(default = "default_renderer")]
    pub renderer: String,
}

fn default_renderer() -> String {
    "wined3d".into()
}

fn default_screen() -> String {
    "800x600".into()
}

impl Recipe {
    /// Where the game says it is installed: a path on `C:` with a backslash at the end.
    pub fn install_path(&self) -> String {
        format!("C:\\{}\\", self.install_dir.as_deref().unwrap_or(&self.id))
    }

    /// A disc that no recipe knows.
    pub fn generic() -> Recipe {
        Recipe {
            id: "generic".into(),
            name: "Windows game".into(),
            revision: 0,
            detect: Vec::new(),
            root: None,
            drive_folder: None,
            label: None,
            install_dir: None,
            program: None,
            screen: default_screen(),
            stubs: Vec::new(),
            registry: Vec::new(),
            title: None,
            installer: None,
            winetricks: Vec::new(),
            overrides: Vec::new(),
            renderer: default_renderer(),
        }
    }

    /// The title text as the bytes the program has it (one per letter, as in the code page).
    pub fn title_bytes(&self) -> Option<(String, Vec<u8>)> {
        let t = self.title.as_ref()?;
        let bytes =
            t.to.chars()
                .map(|c| u8::try_from(c as u32).ok())
                .collect::<Option<Vec<u8>>>()?;
        Some((t.from.clone(), bytes))
    }

    /// The path of the program in the folder of the game's files: the program is a path on the
    /// CD drive, whose folder `drive_folder` is the folder of the files.
    pub fn program_in_game(&self, game: &Path, program: &str) -> Option<std::path::PathBuf> {
        let mut rel = program
            .trim_start_matches("D:")
            .trim_start_matches(['\\', '/']);
        if let Some(folder) = &self.drive_folder {
            if rel.len() > folder.len()
                && rel[..folder.len()].eq_ignore_ascii_case(folder)
                && rel[folder.len()..].starts_with(['\\', '/'])
            {
                rel = &rel[folder.len() + 1..];
            }
        }
        resolve_ci(game, rel).map(|names| game.join(names.join("/")))
    }
}

static RECIPES: OnceLock<Vec<Recipe>> = OnceLock::new();

/// Every known recipe.
pub fn all() -> &'static [Recipe] {
    RECIPES.get_or_init(|| {
        parse(include_str!("../../data/pc_games.json"))
            .expect("data/pc_games.json is valid (checked by tests)")
    })
}

/// Read a data file of recipes; errors name the problem.
pub fn parse(json: &str) -> Result<Vec<Recipe>, String> {
    let recipes: Vec<Recipe> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut seen = std::collections::HashSet::new();
    for r in &recipes {
        if !seen.insert(r.id.clone()) {
            return Err(format!("duplicate recipe {}", r.id));
        }
        if r.detect.is_empty() {
            return Err(format!(
                "recipe {} says nothing about how to find its disc",
                r.id
            ));
        }
        for key in &r.registry {
            if key.hives.iter().any(|h| h != "HKCU" && h != "HKLM") {
                return Err(format!("recipe {}: a hive is HKCU or HKLM", r.id));
            }
            if key
                .values
                .iter()
                .any(|v| v.kind != "REG_SZ" && v.kind != "REG_DWORD")
            {
                return Err(format!("recipe {}: a value is REG_SZ or REG_DWORD", r.id));
            }
        }
        if r.stubs.iter().any(|s| s.kind != "aweman32") {
            return Err(format!(
                "recipe {}: a stand-in this program does not have",
                r.id
            ));
        }
        let plain = |s: &str, extra: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
        };
        if let Some(v) = r.winetricks.iter().find(|v| !plain(v, "_.-")) {
            return Err(format!("recipe {}: \"{v}\" is not a winetricks verb", r.id));
        }
        if let Some(o) = r.overrides.iter().find(|o| {
            o.split_once('=')
                .is_none_or(|(name, mode)| !plain(name, "_.-") || !plain(mode, ",_"))
        }) {
            return Err(format!("recipe {}: \"{o}\" is not a DLL override", r.id));
        }
        if r.renderer != "wined3d" && r.renderer != "dxvk" {
            return Err(format!("recipe {}: a renderer is wined3d or dxvk", r.id));
        }
        if r.title.is_some() && r.title_bytes().is_none() {
            return Err(format!(
                "recipe {}: the title has a letter outside the code page",
                r.id
            ));
        }
    }
    Ok(recipes)
}

/// A file or folder of a folder, by a path in any letter case: where it is.
pub fn find_ci(folder: &Path, rel: &str) -> Option<std::path::PathBuf> {
    let mut at = folder.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let name = std::fs::read_dir(&at)
            .ok()?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.eq_ignore_ascii_case(part))?;
        at.push(name);
    }
    Some(at)
}

/// Whether a disc (an image, or a folder it was unpacked to) holds `path`.
fn disc_has(disc: &Path, path: &str) -> Result<bool, String> {
    if disc.is_dir() {
        return Ok(find_ci(disc, path).is_some());
    }
    Ok(Iso::open(disc)?.find(path).is_ok())
}

/// The recipe of a disc: the first whose files are on it, or the generic one.
pub fn for_disc(disc: &Path) -> Result<&'static Recipe, String> {
    static GENERIC: OnceLock<Recipe> = OnceLock::new();
    // open an image once for all the questions
    let iso = if disc.is_dir() {
        None
    } else {
        Some(Iso::open(disc)?)
    };
    let has = |path: &str| match &iso {
        Some(iso) => iso.find(path).is_ok(),
        None => find_ci(disc, path).is_some(),
    };
    Ok(all()
        .iter()
        .find(|r| r.detect.iter().any(|p| has(p)))
        .unwrap_or_else(|| GENERIC.get_or_init(Recipe::generic)))
}

/// The recipe with this id, or the generic one.
pub fn by_id(id: &str) -> &'static Recipe {
    static GENERIC: OnceLock<Recipe> = OnceLock::new();
    all()
        .iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| GENERIC.get_or_init(Recipe::generic))
}

/// The folder of the disc to copy: `root` when it is on the disc, else the whole disc (`""`).
pub fn root_on(disc: &Path, recipe: &Recipe) -> Result<String, String> {
    match &recipe.root {
        Some(root) if disc_has(disc, root)? => Ok(root.clone()),
        _ => Ok(String::new()),
    }
}

/// The letter of the CD drive that Wine is given. Not `D:`: Wine gives that letter to a disc or a
/// disk image that is mounted on the computer, over what is set up for the game. Recipes name
/// the CD drive `D:`, as the games' own files do, and the path is changed when it is used.
pub const CD_DRIVE: char = 'R';

/// A path on the CD drive as the recipes and the `ready` file write it (`D:\...`), on the drive
/// that is really used.
pub fn on_cd_drive(path: &str) -> String {
    match path.get(..2) {
        Some(d) if d.eq_ignore_ascii_case("D:") => format!("{CD_DRIVE}:{}", &path[2..]),
        _ => path.to_string(),
    }
}

/// The command that makes the CD drive a CD-ROM drive for Wine (`reg add`, like the others).
pub fn drive_command() -> Vec<String> {
    let letter = format!("{}:", CD_DRIVE.to_ascii_lowercase());
    [
        "reg",
        "add",
        "HKLM\\Software\\Wine\\Drives",
        "/v",
        &letter,
        "/d",
        "cdrom",
        "/f",
    ]
    .map(String::from)
    .to_vec()
}

/// The `reg add` commands (program `reg` first) for the registry entries of a recipe. `install` is
/// where the game was installed, with a backslash at the end.
pub fn registry_commands(recipe: &Recipe, install: &str) -> Vec<Vec<String>> {
    let mut commands = Vec::new();
    for key in &recipe.registry {
        for hive in &key.hives {
            let full = format!("{hive}\\{}", key.key);
            for v in &key.values {
                let data = v.data.replace("{install}", install);
                let mut c: Vec<String> = [
                    "reg", "add", &full, "/v", &v.name, "/t", &v.kind, "/d", &data,
                ]
                .map(String::from)
                .to_vec();
                c.push("/f".into());
                if key.view32 {
                    // the game is a 32-bit program: it reads the 32-bit view of the registry
                    c.push("/reg:32".into());
                }
                commands.push(c);
            }
        }
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_file_is_valid_and_road_rash_is_in_it() {
        let r = all().iter().find(|r| r.id == "road-rash").unwrap();
        assert_eq!(r.screen, "640x480");
        assert_eq!(r.program.as_deref(), Some("D:\\RoadRash\\ROADRASH.EXE"));
        assert_eq!(r.stubs[0].file, "AWEMAN32.DLL");
        let (from, to) = r.title_bytes().unwrap();
        assert_eq!(from, "Road Rash");
        assert_eq!(to, b"Road Rash \x97 Wine");
    }

    #[test]
    fn a_bad_data_file_says_what_is_wrong() {
        assert!(parse("[{\"id\":\"a\",\"name\":\"A\"}]")
            .unwrap_err()
            .contains("find its disc"));
        let twice =
            r#"[{"id":"a","name":"A","detect":["X"]},{"id":"a","name":"B","detect":["Y"]}]"#;
        assert!(parse(twice).unwrap_err().contains("duplicate"));
        let hive = r#"[{"id":"a","name":"A","detect":["X"],"registry":[{"key":"K","hives":["HKCR"],"values":[]}]}]"#;
        assert!(parse(hive).unwrap_err().contains("hive"));
        let kind = r#"[{"id":"a","name":"A","detect":["X"],"registry":[{"key":"K","hives":["HKCU"],"values":[{"name":"n","type":"REG_QWORD","data":"1"}]}]}]"#;
        assert!(parse(kind).unwrap_err().contains("REG_SZ"));
        let stub =
            r#"[{"id":"a","name":"A","detect":["X"],"stubs":[{"file":"F","kind":"magic"}]}]"#;
        assert!(parse(stub).unwrap_err().contains("stand-in"));
        let verb = r#"[{"id":"a","name":"A","detect":["X"],"winetricks":["vcrun2015; rm -rf ~"]}]"#;
        assert!(parse(verb).unwrap_err().contains("winetricks verb"));
        let over = r#"[{"id":"a","name":"A","detect":["X"],"overrides":["quartz"]}]"#;
        assert!(parse(over).unwrap_err().contains("override"));
        let ok = r#"[{"id":"a","name":"A","detect":["X"],"winetricks":["vcrun2015","corefonts"],"overrides":["quartz=n,b"]}]"#;
        assert_eq!(parse(ok).unwrap()[0].winetricks, ["vcrun2015", "corefonts"]);
        let renderer = r#"[{"id":"a","name":"A","detect":["X"],"renderer":"metal"}]"#;
        assert!(parse(renderer).unwrap_err().contains("renderer"));
        let title =
            r#"[{"id":"a","name":"A","detect":["X"],"title":{"from":"A","to":"A \u0394"}}]"#;
        assert!(parse(title).unwrap_err().contains("code page"));
    }

    #[test]
    fn the_registry_commands_of_road_rash_are_what_was_found_to_work() {
        let r = by_id("road-rash");
        let commands = registry_commands(r, "C:\\RoadRash\\");
        // the current user's, then the machine's, six values each, all in the 32-bit view
        assert_eq!(commands.len(), 12);
        let text: Vec<String> = commands.iter().map(|c| c.join(" ")).collect();
        assert_eq!(
            text[0],
            "reg add HKCU\\Software\\Electronic Arts\\RoadRash 95 /v Path /t REG_SZ /d C:\\RoadRash\\ /f /reg:32"
        );
        assert!(text[6].starts_with("reg add HKLM\\"));
        assert!(text.iter().all(|c| c.ends_with("/f /reg:32")));
        assert!(text
            .iter()
            .any(|c| c.contains("/v Intro /t REG_DWORD /d 0")));
    }

    #[test]
    fn a_recipe_is_found_by_the_files_of_the_disc_in_any_letter_case() {
        let dir = std::env::temp_dir().join(format!("vuretro-recipe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("RoadRash")).unwrap();
        std::fs::write(dir.join("RoadRash/roadrash.exe"), "x").unwrap();
        assert_eq!(for_disc(&dir).unwrap().id, "road-rash");
        assert_eq!(root_on(&dir, for_disc(&dir).unwrap()).unwrap(), "ROADRASH");
        let other = dir.join("other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("GAME.EXE"), "x").unwrap();
        assert_eq!(for_disc(&other).unwrap().id, "generic");
        assert_eq!(root_on(&other, for_disc(&other).unwrap()).unwrap(), "");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_program_of_a_recipe_is_found_in_the_files_that_were_copied() {
        let game = std::env::temp_dir().join(format!("vuretro-prog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&game);
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("ROADRASH.EXE"), "x").unwrap();
        let r = by_id("road-rash");
        // D:\RoadRash is the folder of the files
        let at = r
            .program_in_game(&game, "D:\\RoadRash\\ROADRASH.EXE")
            .unwrap();
        assert_eq!(at, game.join("ROADRASH.EXE"));
        // a recipe without such a folder takes the path as it is
        std::fs::create_dir_all(game.join("Bin")).unwrap();
        std::fs::write(game.join("Bin/Play.exe"), "x").unwrap();
        let at = Recipe::generic()
            .program_in_game(&game, "D:\\bin\\play.exe")
            .unwrap();
        assert_eq!(at, game.join("Bin/Play.exe"));
        assert!(r
            .program_in_game(&game, "D:\\RoadRash\\Missing.EXE")
            .is_none());
        std::fs::remove_dir_all(game).unwrap();
    }
}
