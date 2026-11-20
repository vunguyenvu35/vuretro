use std::io::{Cursor, Read};
use std::path::PathBuf;
use vuretro_core::paths::Paths;
use vuretro_core::ports::CoreInstaller;
use vuretro_core::registry::core_filename;

const BASE: &str = "https://buildbot.libretro.com/nightly";
/// The biggest download or unpacked core accepted (the current MAME core is a few hundred MB).
const MAX_BYTES: u64 = 800 * 1024 * 1024;

/// Fetches `url` and reports `(received, total)` while doing so.
pub type Fetch =
    Box<dyn Fn(&str, &dyn Fn(u64, Option<u64>)) -> Result<Vec<u8>, String> + Send + Sync>;

/// Installs cores from the libretro buildbot into `<data dir>/cores`.
///
/// The buildbot publishes no checksums, so integrity rests on HTTPS plus structural
/// checks: the archive must hold the expected file, and the file must be a native
/// library of this OS.
pub struct BuildbotInstaller {
    paths: Paths,
    fetch: Fetch,
    base: String,
    /// Folders that may already hold the core's zip (an offline set), looked at first.
    offline: Vec<PathBuf>,
}

impl BuildbotInstaller {
    pub fn new(paths: Paths) -> Self {
        let offline = offline_dirs(&paths);
        Self::with_fetch(paths, BASE.to_string(), Box::new(http_get)).with_offline(offline)
    }

    pub fn with_fetch(paths: Paths, base: String, fetch: Fetch) -> Self {
        BuildbotInstaller {
            paths,
            fetch,
            base,
            offline: Vec::new(),
        }
    }

    /// Where an offline set may be (see [`offline_dirs`]); a core found there is installed
    /// without any download.
    pub fn with_offline(mut self, dirs: Vec<PathBuf>) -> Self {
        self.offline = dirs;
        self
    }

    /// The zip of a core in an offline folder, if one has it. Both `<dir>/<core>.zip` and
    /// `<dir>/<buildbot platform>/<core>.zip` (what `make cores-offline` writes) are found.
    fn offline_zip(&self, name: &str) -> Option<PathBuf> {
        let file = format!("{name}.zip");
        self.offline.iter().find_map(|d| {
            [d.join(&file), d.join(platform_dir()).join(&file)]
                .into_iter()
                .find(|p| p.is_file())
        })
    }

    fn url(&self, stem: &str) -> String {
        format!(
            "{}/{}/latest/{}.zip",
            self.base,
            platform_dir(),
            core_filename(stem)
        )
    }
}

/// Folders searched for an offline set of cores, so a computer with no internet can still
/// install them: `$VURETRO_CORES_OFFLINE`, `<data dir>/cores-offline`, and `cores/offline`
/// beside the app or the source it runs from (fill that one with `make cores-offline`).
pub fn offline_dirs(paths: &Paths) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(d) = std::env::var("VURETRO_CORES_OFFLINE") {
        dirs.push(PathBuf::from(d));
    }
    dirs.push(paths.data_dir().join("cores-offline"));
    let starts = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf)),
        std::env::current_dir().ok(),
    ];
    for start in starts.into_iter().flatten() {
        for dir in start.ancestors() {
            let d = dir.join("cores").join("offline");
            if !dirs.contains(&d) {
                dirs.push(d);
            }
        }
    }
    dirs
}

/// Buildbot folder for this OS and CPU.
fn platform_dir() -> &'static str {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "apple/osx/arm64"
        } else {
            "apple/osx/x86_64"
        }
    } else if cfg!(target_os = "windows") {
        "windows/x86_64"
    } else {
        "linux/x86_64"
    }
}

fn http_get(url: &str, progress: &dyn Fn(u64, Option<u64>)) -> Result<Vec<u8>, String> {
    // Without limits a stalled connection waits for ever, and the core stays "being
    // installed" until the app is restarted. The body has no limit of its own (it is the
    // time for the whole download, and a big core takes minutes): the global one covers it.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(15)))
        .timeout_recv_response(Some(std::time::Duration::from_secs(30)))
        .timeout_global(Some(std::time::Duration::from_secs(7200)))
        .build()
        .into();
    let mut resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let total = resp.body().content_length();
    if total.is_some_and(|t| t > MAX_BYTES) {
        return Err("download is too large to be a core".into());
    }
    let mut reader = resp.body_mut().as_reader();
    let mut out = Vec::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("download failed: {e}"))?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
        if out.len() as u64 > MAX_BYTES {
            return Err("download is too large to be a core".into());
        }
        progress(out.len() as u64, total);
    }
    Ok(out)
}

/// Does `bytes` start like a native library of this OS?
fn is_native_library(bytes: &[u8]) -> bool {
    if cfg!(target_os = "macos") {
        // Mach-O 32/64-bit (either byte order) or a universal ("fat") binary.
        matches!(
            bytes.get(..4),
            Some([0xcf, 0xfa, 0xed, 0xfe])
                | Some([0xce, 0xfa, 0xed, 0xfe])
                | Some([0xca, 0xfe, 0xba, 0xbe])
        )
    } else if cfg!(target_os = "windows") {
        bytes.starts_with(b"MZ")
    } else {
        bytes.starts_with(b"\x7fELF")
    }
}

/// Pull `name` out of a zip held in memory.
fn unpack(zip_bytes: Vec<u8>, name: &str) -> Result<Vec<u8>, String> {
    let mut zip =
        zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(|e| format!("bad archive: {e}"))?;
    let mut file = zip
        .by_name(name)
        .map_err(|_| format!("archive does not contain {name}"))?;
    if file.size() > MAX_BYTES {
        return Err("core is too large".into());
    }
    let mut out = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut out)
        .map_err(|e| format!("cannot unpack {name}: {e}"))?;
    Ok(out)
}

impl CoreInstaller for BuildbotInstaller {
    fn install(&self, stem: &str, progress: &dyn Fn(u8)) -> Result<PathBuf, String> {
        // The stem ends up in a URL and a file name: accept plain identifiers only.
        if stem.is_empty() || !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("invalid core name {stem:?}"));
        }
        let name = core_filename(stem);
        progress(0);
        let zipped = match self.offline_zip(&name) {
            Some(local) => {
                log::info!("installing {name} from {}", local.display());
                let bytes = std::fs::read(&local)
                    .map_err(|e| format!("cannot read {}: {e}", local.display()))?;
                progress(50);
                bytes
            }
            None => (self.fetch)(&self.url(stem), &|got, total| {
                if let Some(t) = total.filter(|t| *t > 0) {
                    progress((got * 95 / t).min(95) as u8);
                }
            })?,
        };
        let lib = unpack(zipped, &name)?;
        if !is_native_library(&lib) {
            return Err(format!("{name} is not a native library for this system"));
        }
        let dir = self.paths.cores_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        // Write beside the target and rename, so a crash never leaves half a core.
        let target = dir.join(&name);
        let tmp = dir.join(format!("{name}.part"));
        std::fs::write(&tmp, &lib).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &target)
            .map_err(|e| format!("cannot install {}: {e}", target.display()))?;
        progress(100);
        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    fn zip_of(name: &str, data: &[u8]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        w.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(data).unwrap();
        w.finish().unwrap().into_inner()
    }

    fn magic() -> Vec<u8> {
        let mut v = if cfg!(target_os = "macos") {
            vec![0xcf, 0xfa, 0xed, 0xfe]
        } else if cfg!(target_os = "windows") {
            b"MZ".to_vec()
        } else {
            b"\x7fELF".to_vec()
        };
        v.extend_from_slice(&[0; 64]);
        v
    }

    fn installer(
        dir: &std::path::Path,
        reply: Result<Vec<u8>, String>,
    ) -> (BuildbotInstaller, Arc<Mutex<Vec<String>>>) {
        let urls = Arc::new(Mutex::new(Vec::new()));
        let seen = urls.clone();
        let fetch: Fetch = Box::new(move |url, progress| {
            seen.lock().unwrap().push(url.to_string());
            if let Ok(b) = &reply {
                progress(b.len() as u64 / 2, Some(b.len() as u64));
            }
            reply.clone()
        });
        (
            BuildbotInstaller::with_fetch(Paths::new(dir), "https://example.test".into(), fetch),
            urls,
        )
    }

    /// Hits the real buildbot: `cargo test -p vuretro-library -- --ignored`.
    #[test]
    #[ignore]
    fn live_download_from_the_buildbot() {
        let dir = tempfile::tempdir().unwrap();
        let inst = BuildbotInstaller::new(Paths::new(dir.path()));
        let path = inst.install("gambatte_libretro", &|_| {}).unwrap();
        assert!(std::fs::metadata(path).unwrap().len() > 100_000);
    }

    #[test]
    fn installs_the_core_into_the_cores_folder() {
        let dir = tempfile::tempdir().unwrap();
        let name = core_filename("snes9x_libretro");
        let (inst, urls) = installer(dir.path(), Ok(zip_of(&name, &magic())));
        let seen = Mutex::new(Vec::new());
        let path = inst
            .install("snes9x_libretro", &|p| seen.lock().unwrap().push(p))
            .unwrap();
        assert_eq!(path, dir.path().join("cores").join(&name));
        assert_eq!(std::fs::read(&path).unwrap(), magic());
        assert!(!dir
            .path()
            .join("cores")
            .join(format!("{name}.part"))
            .exists());
        assert!(urls.lock().unwrap()[0].starts_with("https://example.test/"));
        assert!(urls.lock().unwrap()[0].ends_with(&format!("/latest/{name}.zip")));
        let seen = seen.into_inner().unwrap();
        assert_eq!((seen[0], *seen.last().unwrap()), (0, 100));
        assert!(seen.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn a_core_in_an_offline_folder_is_installed_without_any_download() {
        let dir = tempfile::tempdir().unwrap();
        let offline = tempfile::tempdir().unwrap();
        let name = core_filename("gambatte_libretro");
        // in the platform folder `make cores-offline` writes
        let sub = offline.path().join(platform_dir());
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join(format!("{name}.zip")), zip_of(&name, &magic())).unwrap();
        // the download would fail: it must not be tried
        let (inst, urls) = installer(dir.path(), Err("no internet".into()));
        let inst = inst.with_offline(vec![offline.path().to_path_buf()]);
        let path = inst.install("gambatte_libretro", &|_| {}).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), magic());
        assert!(urls.lock().unwrap().is_empty());
        // a core that is not in the offline folder is downloaded, and without internet fails
        assert_eq!(
            inst.install("mgba_libretro", &|_| {}).unwrap_err(),
            "no internet"
        );
        // a zip straight in the folder works too
        std::fs::write(
            offline.path().join(format!("{name}.zip")),
            zip_of(&name, &magic()),
        )
        .unwrap();
        std::fs::remove_dir_all(&sub).unwrap();
        assert!(inst.install("gambatte_libretro", &|_| {}).is_ok());
    }

    #[test]
    fn refuses_bad_downloads_and_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let name = core_filename("fceumm_libretro");
        // wrong content inside the archive
        let (inst, _) = installer(dir.path(), Ok(zip_of(&name, b"<html>not a core</html>")));
        assert!(inst
            .install("fceumm_libretro", &|_| {})
            .unwrap_err()
            .contains("not a native"));
        // archive without the file
        let (inst, _) = installer(dir.path(), Ok(zip_of("other.txt", b"x")));
        assert!(inst
            .install("fceumm_libretro", &|_| {})
            .unwrap_err()
            .contains("does not contain"));
        // not an archive at all
        let (inst, _) = installer(dir.path(), Ok(b"nope".to_vec()));
        assert!(inst
            .install("fceumm_libretro", &|_| {})
            .unwrap_err()
            .contains("bad archive"));
        // network error is passed on
        let (inst, _) = installer(dir.path(), Err("download failed: offline".into()));
        assert!(inst
            .install("fceumm_libretro", &|_| {})
            .unwrap_err()
            .contains("offline"));
        assert!(!dir.path().join("cores").join(&name).exists());
    }

    #[test]
    fn names_that_could_escape_are_rejected_before_any_request() {
        let dir = tempfile::tempdir().unwrap();
        let (inst, urls) = installer(dir.path(), Ok(Vec::new()));
        for bad in ["", "../evil", "a/b", "x y", "core.dylib"] {
            assert!(inst.install(bad, &|_| {}).is_err(), "{bad}");
        }
        assert!(urls.lock().unwrap().is_empty());
    }
}
