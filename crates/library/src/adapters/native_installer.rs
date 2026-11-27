use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use vuretro_core::paths::Paths;
use vuretro_core::ports::{NativeInstaller, NativeSpec};

/// Where the source of the native port is fetched from, and nowhere else.
const REPO: &str = "https://github.com/arkits/road-rash";

/// Where a program started from the dock does not look: the folders of Homebrew and the like.
const EXTRA_PATH: &[&str] = &[
    "/opt/homebrew/bin",
    "/opt/homebrew/sbin",
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
];

/// The search path of the programs we start: the one we have and the usual extra ones.
pub(super) fn search_path() -> OsString {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.extend(EXTRA_PATH.iter().map(PathBuf::from));
    // every folder once, in the order they were first named
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    std::env::join_paths(dirs).unwrap_or_default()
}

/// A program of that name on the search path.
pub(super) fn find_tool(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&search_path())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// Whether the SDL 3 library the port draws and plays with is installed.
fn has_sdl3() -> bool {
    let libs = [
        "/opt/homebrew/lib",
        "/usr/local/lib",
        "/usr/lib",
        "/usr/lib64",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
    ];
    libs.iter().any(|d| {
        std::fs::read_dir(d).is_ok_and(|rd| {
            rd.flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with("libSDL3."))
        })
    })
}

/// The text of the `game.json` that makes the installed port show up as a game.
fn manifest(cover: Option<&str>) -> String {
    let mut m = serde_json::json!({
        "name": "Road Rash",
        "exe": "rr-game",
        "args": ["--assets", "{dir}/ROADRASH"],
    });
    if let Some(c) = cover {
        m["cover"] = serde_json::Value::String(c.to_string());
    }
    m.to_string()
}

/// Run a command, passing on every line it prints (output and errors alike) to `log`. On
/// failure the error holds the last lines it printed.
pub(super) fn run(cmd: &mut Command, log: &dyn Fn(&str)) -> Result<(), String> {
    let program = cmd.get_program().to_string_lossy().into_owned();
    let mut child = cmd
        .env("PATH", search_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start {program}: {e}"))?;
    let (to, lines) = channel::<String>();
    let readers: Vec<_> = [
        child
            .stdout
            .take()
            .map(|o| (Box::new(o) as Box<dyn std::io::Read + Send>, to.clone())),
        child
            .stderr
            .take()
            .map(|o| (Box::new(o) as Box<dyn std::io::Read + Send>, to.clone())),
    ]
    .into_iter()
    .flatten()
    .map(|(out, to)| {
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if to.send(line).is_err() {
                    break;
                }
            }
        })
    })
    .collect();
    drop(to);
    let mut last: Vec<String> = Vec::new();
    for line in lines {
        let line = line.trim_end().to_string();
        if !line.is_empty() {
            log(&line);
            last.push(line);
            if last.len() > 6 {
                last.remove(0);
            }
        }
    }
    for r in readers {
        let _ = r.join();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed ({status}): {}", last.join(" / ")))
    }
}

/// Builds the native Road Rash port from its source into `<data dir>/native/road-rash`.
pub struct RoadRashInstaller {
    paths: Paths,
    /// The Dock icon of Windows games: 256x256 red, green, blue, alpha bytes.
    pc_icon: Option<Vec<u8>>,
}

impl RoadRashInstaller {
    pub fn new(paths: Paths) -> Self {
        RoadRashInstaller {
            paths,
            pc_icon: None,
        }
    }

    /// The icon that Windows games get in the Dock (256x256 pixels of red, green, blue and alpha
    /// bytes), instead of one drawn from the icon inside the game's program.
    pub fn with_pc_icon(mut self, rgba: Vec<u8>) -> Self {
        self.pc_icon = Some(rgba);
        self
    }
}

impl NativeInstaller for RoadRashInstaller {
    fn missing_tools(&self) -> Vec<String> {
        let mut missing = Vec::new();
        if find_tool("git").is_none() {
            missing.push("git".to_string());
        }
        if find_tool("cmake").is_none() {
            missing.push("cmake".to_string());
        }
        if ["c++", "clang++", "g++"]
            .iter()
            .all(|c| find_tool(c).is_none())
        {
            missing.push("a C++ compiler".to_string());
        }
        if !has_sdl3() {
            missing.push("SDL3".to_string());
        }
        missing
    }

    fn install_tools(&self, log: &dyn Fn(&str)) -> Result<(), String> {
        if cfg!(windows) {
            return Err("Setting the tools up is not done for Windows yet".into());
        }
        if !cfg!(target_os = "macos") {
            return Err("Install git, cmake, a C++ compiler and SDL 3 with your package manager (SDL 3 is recent: a newer release of your distribution may be needed)".into());
        }
        let missing = self.missing_tools();
        if missing.iter().any(|m| m == "git" || m == "a C++ compiler") {
            return Err("Install the command line tools of Apple first: run `xcode-select --install` in a terminal".into());
        }
        let packages: Vec<&str> = missing
            .iter()
            .filter_map(|m| match m.as_str() {
                "cmake" => Some("cmake"),
                "SDL3" => Some("sdl3"),
                _ => None,
            })
            .collect();
        if packages.is_empty() {
            return Ok(());
        }
        let brew =
            find_tool("brew").ok_or("Homebrew is needed to install them: see https://brew.sh")?;
        log(&format!("brew install {}", packages.join(" ")));
        run(Command::new(brew).arg("install").args(&packages), log)
    }

    fn pc_ready(&self) -> bool {
        super::wine_installer::ready(&self.paths)
    }

    fn install_pc_runtime(&self, log: &dyn Fn(&str)) -> Result<(), String> {
        super::wine_installer::install_runtime(&self.paths, log)
    }

    fn run_pc_installer(&self, disc: &Path, log: &dyn Fn(&str)) -> Result<String, String> {
        super::wine_installer::run_installer(&self.paths, disc, self.pc_icon.as_deref(), log)
    }

    fn prepare_pc_game(&self, disc: &Path, log: &dyn Fn(&str)) -> Result<NativeSpec, String> {
        super::wine_installer::prepare(&self.paths, disc, self.pc_icon.as_deref(), log)
    }

    fn install_road_rash(&self, disc: &Path, log: &dyn Fn(&str)) -> Result<(), String> {
        if cfg!(windows) {
            return Err("Building the port is not done for Windows yet".into());
        }
        let missing = self.missing_tools();
        if !missing.is_empty() {
            return Err(format!("these are missing first: {}", missing.join(", ")));
        }
        let (git, cmake) = (
            find_tool("git").ok_or("git is missing")?,
            find_tool("cmake").ok_or("cmake is missing")?,
        );
        let out = self.paths.native_dir().join("road-rash");
        let src = self.paths.native_dir().join("road-rash-src");
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;

        log("1/4 Fetching the source of the port");
        if src.join(".git").is_dir() {
            run(
                Command::new(&git)
                    .arg("-C")
                    .arg(&src)
                    .args(["pull", "--ff-only"]),
                log,
            )?;
        } else {
            run(
                Command::new(&git)
                    .args(["clone", "--depth", "1", REPO])
                    .arg(&src),
                log,
            )?;
        }

        log("2/4 Setting the build up");
        let build = src.join("build");
        let mut configure = Command::new(&cmake);
        configure
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .arg("-DCMAKE_BUILD_TYPE=Release");
        let prefixes: Vec<&str> = ["/opt/homebrew", "/usr/local"]
            .into_iter()
            .filter(|p| Path::new(p).is_dir())
            .collect();
        if !prefixes.is_empty() {
            configure.arg(format!("-DCMAKE_PREFIX_PATH={}", prefixes.join(";")));
        }
        run(&mut configure, log)?;

        log("3/4 Building the game (a few minutes)");
        let jobs = std::thread::available_parallelism().map_or(4, |n| n.get());
        run(
            Command::new(&cmake)
                .arg("--build")
                .arg(&build)
                .args(["--target", "rr-game", "rr-import", "--parallel"])
                .arg(jobs.to_string()),
            log,
        )?;
        let program = out.join("rr-game");
        std::fs::copy(build.join("rr-game"), &program)
            .map_err(|e| format!("the build made no rr-game: {e}"))?;

        log("4/4 Reading the game's data from your disc");
        let data = out.join("ROADRASH");
        if data.exists() {
            std::fs::remove_dir_all(&data).map_err(|e| e.to_string())?;
        }
        run(
            Command::new(build.join("rr-import")).arg(disc).arg(&data),
            log,
        )?;
        let cover = "ROADRASH/IMAGES/SPLASH.RRI";
        let cover = out.join(cover).is_file().then_some(cover);
        std::fs::write(out.join("game.json"), manifest(cover)).map_err(|e| e.to_string())?;
        log("Done");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_usual_extra_folders_are_searched_even_when_the_path_is_short() {
        let path = search_path();
        let dirs: Vec<PathBuf> = std::env::split_paths(&path).collect();
        assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(dirs.contains(&PathBuf::from("/usr/local/bin")));
        let mut seen = std::collections::HashSet::new();
        assert!(
            dirs.iter().all(|d| seen.insert(d.clone())),
            "no folder twice"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_tool_is_found_by_name_and_a_missing_one_is_not() {
        assert!(find_tool("sh").is_some());
        assert!(find_tool("no-such-tool-for-vuretro").is_none());
    }

    #[test]
    fn the_manifest_names_the_program_and_the_data_and_the_picture_when_there_is_one() {
        let with: serde_json::Value = serde_json::from_str(&manifest(Some("a/b.jpg"))).unwrap();
        assert_eq!(with["exe"], "rr-game");
        assert_eq!(with["args"][1], "{dir}/ROADRASH");
        assert_eq!(with["cover"], "a/b.jpg");
        let without: serde_json::Value = serde_json::from_str(&manifest(None)).unwrap();
        assert!(without.get("cover").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn the_lines_a_command_prints_are_passed_on_and_a_failure_says_what_it_last_said() {
        let seen = std::sync::Mutex::new(Vec::new());
        let log = |l: &str| seen.lock().unwrap().push(l.to_string());
        run(
            Command::new("sh").args(["-c", "echo one; echo two >&2"]),
            &log,
        )
        .unwrap();
        let mut got = seen.lock().unwrap().clone();
        got.sort();
        assert_eq!(got, ["one", "two"]);
        let err = run(
            Command::new("sh").args(["-c", "echo about to fail; exit 3"]),
            &|_| {},
        )
        .unwrap_err();
        assert!(
            err.contains("about to fail") && err.contains("failed"),
            "{err}"
        );
        assert!(run(&mut Command::new("/no/such/program"), &|_| {}).is_err());
    }

    #[test]
    fn a_missing_tool_stops_the_install_before_anything_is_started() {
        let installer = RoadRashInstaller::new(Paths::new("/nonexistent-vuretro"));
        if installer.missing_tools().is_empty() {
            return; // a machine that has everything: nothing to check here
        }
        assert!(installer
            .install_road_rash(Path::new("/nowhere.iso"), &|_| {})
            .unwrap_err()
            .contains("missing"));
    }
}
