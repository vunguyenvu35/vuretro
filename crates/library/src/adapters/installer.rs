use std::io::{Cursor, Read};
use std::path::PathBuf;
use vuretro_core::paths::Paths;
use vuretro_core::ports::CoreInstaller;
use vuretro_core::registry::core_filename;

const BASE: &str = "https://buildbot.libretro.com/nightly";
/// Where libretro keeps the data some cores need beside them (`system/` folder contents).
const ASSETS: &str = "https://buildbot.libretro.com/assets/system";
/// Cores that do not run without data in the system folder: `(core, zip on ASSETS, a folder of
/// it that tells it is there already)`. blueMSX needs its `Machines` and `Databases`.
const SUPPORT_DATA: [(&str, &str, &str); 1] = [("bluemsx_libretro", "blueMSX", "Machines")];
/// The most that a data zip may unpack to.
const MAX_SUPPORT_BYTES: u64 = 300 * 1024 * 1024;

/// The biggest download or unpacked core accepted (the current MAME core is a few hundred MB).
const MAX_BYTES: u64 = 800 * 1024 * 1024;

/// Fetches `url` and reports `(received, total)` while doing so.
pub type Fetch =
    Box<dyn Fn(&str, &dyn Fn(u64, Option<u64>)) -> Result<Vec<u8>, String> + Send + Sync>;

/// When the server says the file at `url` was last changed (a HEAD request), if it says.
pub type Head = Box<dyn Fn(&str) -> Result<Option<std::time::SystemTime>, String> + Send + Sync>;

/// Installs cores from the libretro buildbot into `<data dir>/cores`.
///
/// The buildbot publishes no checksums, so integrity rests on HTTPS plus structural
/// checks: the archive must hold the expected file, and the file must be a native
/// library of this OS.
pub struct BuildbotInstaller {
    paths: Paths,
    fetch: Fetch,
    head: Head,
    base: String,
    /// Folders that may already hold the core's zip (an offline set), looked at first.
    offline: Vec<PathBuf>,
}

impl BuildbotInstaller {
    pub fn new(paths: Paths) -> Self {
        let offline = offline_dirs(&paths);
        Self::with_fetch(paths, BASE.to_string(), Box::new(http_get))
            .with_head(Box::new(http_modified))
            .with_offline(offline)
    }

    /// How the date of a core on the server is asked (nothing by default: no update is known).
    pub fn with_head(mut self, head: Head) -> Self {
        self.head = head;
        self
    }

    pub fn with_fetch(paths: Paths, base: String, fetch: Fetch) -> Self {
        BuildbotInstaller {
            paths,
            fetch,
            head: Box::new(|_| Ok(None)),
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

/// The `Last-Modified` of the file at `url`, without downloading it.
fn http_modified(url: &str) -> Result<Option<std::time::SystemTime>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(10)))
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    let resp = agent
        .head(url)
        .call()
        .map_err(|e| format!("cannot reach the server: {e}"))?;
    Ok(resp
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| httpdate::parse_http_date(v).ok()))
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

impl BuildbotInstaller {
    /// Fetch the data a core needs in the system folder, if it needs any and it is not there yet.
    fn install_support_data(&self, stem: &str) -> Result<(), String> {
        let Some((_, asset, marker)) = SUPPORT_DATA.iter().find(|(c, _, _)| *c == stem) else {
            return Ok(());
        };
        let system = self.paths.system_dir();
        if system.join(marker).is_dir() {
            return Ok(());
        }
        let url = format!("{ASSETS}/{asset}.zip");
        let bytes = (self.fetch)(&url, &|_, _| {})?;
        let mut zip =
            zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("bad archive: {e}"))?;
        let mut total = 0u64;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
            // only paths that stay inside the system folder
            let Some(rel) = file.enclosed_name() else {
                continue;
            };
            let target = system.join(rel);
            if file.is_dir() {
                std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
                continue;
            }
            total += file.size();
            if total > MAX_SUPPORT_BYTES {
                return Err(format!("{asset}.zip is bigger than it should be"));
            }
            if let Some(dir) = target.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            let mut out = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut out).map_err(|e| e.to_string())?;
            std::fs::write(&target, out).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

impl CoreInstaller for BuildbotInstaller {
    /// The server's build is newer than the file that is installed (which dates from when it
    /// was installed, so a build made before that is not an update).
    fn update_available(&self, stem: &str) -> Result<bool, String> {
        if stem.is_empty() || !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("invalid core name {stem:?}"));
        }
        let installed = self.paths.cores_dir().join(core_filename(stem));
        let Ok(have) = std::fs::metadata(&installed).and_then(|m| m.modified()) else {
            return Ok(false);
        };
        Ok((self.head)(&self.url(stem))?.is_some_and(|built| built > have))
    }

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
        // a core that needs data of libretro's: the core is installed either way, and a failure
        // here is told in the log (the core then says what it lacks when a game starts)
        if let Err(e) = self.install_support_data(stem) {
            log::warn!("data for {stem} not installed: {e}");
        }
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
    fn a_newer_build_on_the_server_is_an_update() {
        use std::time::{Duration, SystemTime};
        let dir = tempfile::tempdir().unwrap();
        let name = core_filename("snes9x_libretro");
        let (inst, _) = installer(dir.path(), Ok(zip_of(&name, &magic())));
        // not installed: nothing to update
        let later = Box::new(|_: &str| Ok(Some(SystemTime::now() + Duration::from_secs(3600))));
        let inst = inst.with_head(later);
        assert_eq!(inst.update_available("snes9x_libretro"), Ok(false));
        inst.install("snes9x_libretro", &|_| {}).unwrap();
        assert_eq!(inst.update_available("snes9x_libretro"), Ok(true));
        // built before it was installed, or a server that does not say: no update
        let earlier = Box::new(|_: &str| Ok(Some(SystemTime::now() - Duration::from_secs(3600))));
        assert_eq!(
            inst.with_head(earlier).update_available("snes9x_libretro"),
            Ok(false)
        );
    }

    #[test]
    fn an_unreachable_server_is_an_error_not_an_update() {
        let dir = tempfile::tempdir().unwrap();
        let name = core_filename("snes9x_libretro");
        let (inst, _) = installer(dir.path(), Ok(zip_of(&name, &magic())));
        inst.install("snes9x_libretro", &|_| {}).unwrap();
        let inst = inst.with_head(Box::new(|_| Err("offline".into())));
        assert!(inst.update_available("snes9x_libretro").is_err());
        assert!(inst.update_available("bad name").is_err());
    }

    #[test]
    fn a_core_that_needs_data_gets_it_in_the_system_folder_once() {
        let dir = tempfile::tempdir().unwrap();
        let name = core_filename("bluemsx_libretro");
        let data = {
            let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
            let o = zip::write::SimpleFileOptions::default();
            w.add_directory("Machines/", o).unwrap();
            w.start_file("Machines/MSX/config.ini", o).unwrap();
            w.write_all(b"x").unwrap();
            w.start_file("../escape.txt", o).unwrap();
            w.write_all(b"x").unwrap();
            w.finish().unwrap().into_inner()
        };
        let core = zip_of(&name, &magic());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let seen = calls.clone();
        let fetch: Fetch = Box::new(move |url, _| {
            seen.lock().unwrap().push(url.to_string());
            Ok(if url.ends_with("blueMSX.zip") {
                data.clone()
            } else {
                core.clone()
            })
        });
        let inst = BuildbotInstaller::with_fetch(
            Paths::new(dir.path()),
            "https://example.test".into(),
            fetch,
        );
        inst.install("bluemsx_libretro", &|_| {}).unwrap();
        let system = Paths::new(dir.path()).system_dir();
        assert!(system.join("Machines/MSX/config.ini").is_file());
        assert!(
            !dir.path().join("escape.txt").exists(),
            "nothing outside the system folder"
        );
        // there already: not fetched again
        let before = calls.lock().unwrap().len();
        inst.install("bluemsx_libretro", &|_| {}).unwrap();
        let after = calls.lock().unwrap().len();
        assert_eq!(after - before, 1, "only the core");
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
