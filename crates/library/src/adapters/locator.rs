use std::path::{Path, PathBuf};
use vuretro_core::paths::Paths;
use vuretro_core::ports::Locator;
use vuretro_core::registry::core_filename;

/// Looks for the runner and cores next to the app, in a source checkout, and in the data folder.
pub struct FsLocator {
    paths: Paths,
}

impl FsLocator {
    pub fn new(paths: Paths) -> Self {
        FsLocator { paths }
    }
}

/// Directories to search for sibling files: the executable's folder and, when
/// running from a source checkout, every parent of it and of the working dir.
fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let starts = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
        std::env::current_dir().ok(),
    ];
    for start in starts.into_iter().flatten() {
        for dir in start.ancestors() {
            if !roots.contains(&dir.to_path_buf()) {
                roots.push(dir.to_path_buf());
            }
        }
    }
    roots
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn newest(paths: impl Iterator<Item = PathBuf>) -> Option<PathBuf> {
    paths
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

impl Locator for FsLocator {
    fn runner(&self) -> Option<PathBuf> {
        if let Ok(p) = std::env::var("VURETRO_RUNNER") {
            return Some(PathBuf::from(p));
        }
        let name = exe_name("vuretro-runner");
        let candidates = search_roots().into_iter().flat_map(|r| {
            [
                r.join(&name),
                r.join("target/release").join(&name),
                r.join("target/debug").join(&name),
            ]
        });
        newest(candidates.filter(|p| p.is_file()))
    }

    fn core(&self, stem: &str) -> Option<PathBuf> {
        let file = core_filename(stem);
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Ok(d) = std::env::var("VURETRO_CORES_DIR") {
            dirs.push(d.into());
        }
        dirs.push(self.paths.cores_dir());
        dirs.extend(
            search_roots()
                .into_iter()
                .flat_map(|r| [r.join("cores"), r.join("cores/bin")]),
        );
        dirs.into_iter()
            .map(|d| d.join(&file))
            .find(|p| p.is_file())
    }

    fn test_core(&self) -> Option<PathBuf> {
        let name = core_filename("vuretro_test_core");
        let name = if cfg!(windows) {
            name
        } else {
            format!("lib{name}")
        };
        newest(
            search_roots()
                .into_iter()
                .flat_map(|r| {
                    [
                        r.join("target/debug").join(&name),
                        r.join("target/release").join(&name),
                    ]
                })
                .filter(|p| p.is_file()),
        )
    }
}
