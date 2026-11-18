use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use vuretro_core::model::CoreOptionDef;
use vuretro_core::paths::Paths;
use vuretro_core::ports::{CoverJob, Exit, GameProcess, LaunchSpec, Launcher};

/// Starts `vuretro-runner` as a child process, so a crashing core never takes the UI down.
pub struct ProcessLauncher {
    paths: Paths,
}

impl ProcessLauncher {
    pub fn new(paths: Paths) -> Self {
        ProcessLauncher { paths }
    }
}

struct Runner {
    child: Child,
    log: PathBuf,
}

impl GameProcess for Runner {
    fn poll(&mut self) -> Result<Option<Exit>, String> {
        let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let detail = if status.success() {
            String::new()
        } else {
            let log = std::fs::read_to_string(&self.log).unwrap_or_default();
            let tail: Vec<&str> = log.lines().rev().take(4).collect();
            let msg = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
            if msg.is_empty() {
                format!("runner exited with {status}")
            } else {
                msg
            }
        };
        Ok(Some(Exit {
            success: status.success(),
            detail,
        }))
    }

    fn wait(&mut self) {
        let _ = self.child.wait();
    }
}

impl Launcher for ProcessLauncher {
    fn spawn(&self, spec: &LaunchSpec) -> Result<Box<dyn GameProcess>, String> {
        let log_path = self.paths.runner_log();
        if let Some(dir) = log_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
        let child = Command::new(&spec.runner)
            .arg("--core")
            .arg(&spec.core)
            .arg("--rom")
            .arg(&spec.rom)
            .arg("--system-dir")
            .arg(&spec.system_dir)
            .arg("--cover")
            .arg(&spec.cover)
            .arg("--system")
            .arg(&spec.system)
            .args(
                spec.load_slot
                    .iter()
                    .flat_map(|n| ["--load-slot".to_string(), n.to_string()]),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .map_err(|e| format!("cannot start runner: {e}"))?;
        Ok(Box::new(Runner {
            child,
            log: log_path,
        }))
    }

    fn capture_cover(&self, job: &CoverJob) -> bool {
        let status = Command::new(&job.runner)
            .arg("--core")
            .arg(&job.core)
            .arg("--rom")
            .arg(&job.rom)
            .arg("--system-dir")
            .arg(&job.system_dir)
            .arg("--save-dir")
            .arg(&job.scratch_dir)
            .arg("--headless-frames")
            .arg("1500")
            .arg("--cover")
            .arg(&job.out)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        matches!(status, Ok(s) if s.success()) && job.out.exists()
    }

    fn list_core_options(&self, runner: &Path, core: &Path) -> Result<Vec<CoreOptionDef>, String> {
        let out = Command::new(runner)
            .arg("--core")
            .arg(core)
            .arg("--options-json")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("cannot start runner: {e}"))?;
        if !out.status.success() {
            return Err("the core could not be loaded".into());
        }
        serde_json::from_slice(&out.stdout).map_err(|e| format!("unreadable option list: {e}"))
    }
}
