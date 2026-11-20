use super::bundle;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use vuretro_core::model::CoreOptionDef;
use vuretro_core::paths::Paths;
use vuretro_core::ports::{CoverJob, Exit, GameProcess, LaunchSpec, Launcher};
use vuretro_core::protocol::{self, Event};

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
    /// What the runner reported, read from its standard output by a thread of its own: the
    /// runner must never wait for the app to read it.
    reports: Receiver<Event>,
    reader: Option<JoinHandle<()>>,
}

/// Read the runner's output line by line and pass on the events among them. Everything else
/// (what a core prints) is skipped. Ends when the output does.
fn read_events(output: impl Read, to: Sender<Event>) {
    for line in BufReader::new(output).lines() {
        let Ok(line) = line else { break };
        if let Some(event) = protocol::parse(&line) {
            if to.send(event).is_err() {
                break;
            }
        }
    }
}

impl GameProcess for Runner {
    fn poll(&mut self) -> Result<Option<Exit>, String> {
        let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let detail = if status.success() {
            String::new()
        } else if let Some(message) = self.reported_error() {
            message
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
        // the last things it said are still on their way: wait for the end of its output
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }

    fn events(&mut self) -> Vec<Event> {
        self.reports.try_iter().collect()
    }
}

impl Runner {
    /// The reason the runner gave for failing, if it gave one. The reader may still be
    /// finishing when the process has just ended, so it is waited for.
    fn reported_error(&mut self) -> Option<String> {
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        self.reports.try_iter().find_map(|e| match e {
            Event::Error { message } => Some(message),
            _ => None,
        })
    }
}

impl ProcessLauncher {
    /// Where to start the runner from. On macOS a bare executable gets the generic "exec" Dock
    /// icon at first, so it is started from a small app bundle that carries Vuretro's icon
    /// (unless it already sits inside an app). Elsewhere, or when the bundle cannot be made,
    /// the runner itself.
    fn start_path(&self, runner: &std::path::Path) -> PathBuf {
        if !cfg!(target_os = "macos") || bundle::inside_bundle(runner) {
            return runner.to_path_buf();
        }
        match bundle::ensure(&self.paths.runner_app(), runner, bundle::ICON_PNG) {
            Ok(exe) => exe,
            Err(e) => {
                log::warn!("cannot make the runner's app bundle ({e}); starting it directly");
                runner.to_path_buf()
            }
        }
    }
}

impl Launcher for ProcessLauncher {
    fn spawn(&self, spec: &LaunchSpec) -> Result<Box<dyn GameProcess>, String> {
        let log_path = self.paths.runner_log();
        if let Some(dir) = log_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
        let runner = self.start_path(&spec.runner);
        let mut child = Command::new(&runner)
            .arg("--core")
            .arg(&spec.core)
            .arg("--rom")
            .arg(&spec.rom)
            .arg("--system-dir")
            .arg(&spec.system_dir)
            .arg("--system")
            .arg(&spec.system)
            .args(
                spec.load_slot
                    .iter()
                    .flat_map(|n| ["--load-slot".to_string(), n.to_string()]),
            )
            .arg("--events")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(log)
            .spawn()
            .map_err(|e| format!("cannot start runner: {e}"))?;
        let (to, reports) = channel();
        let reader = child
            .stdout
            .take()
            .map(|out| std::thread::spawn(move || read_events(out, to)));
        Ok(Box::new(Runner {
            child,
            log: log_path,
            reports,
            reader,
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

    fn list_core_options(
        &self,
        runner: &Path,
        core: &Path,
        rom: Option<&Path>,
    ) -> Result<Vec<CoreOptionDef>, String> {
        let mut cmd = Command::new(runner);
        cmd.arg("--core").arg(core).arg("--options-json");
        if let Some(rom) = rom {
            cmd.arg("--rom").arg(rom);
        }
        let out = cmd
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("cannot start runner: {e}"))?;
        if !out.status.success() {
            return Err("the core could not be loaded".into());
        }
        // a core may print to stdout too: the list is the last line that looks like JSON
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text
            .lines()
            .rev()
            .find(|l| l.trim_start().starts_with('['))
            .ok_or("the core gave no option list")?;
        serde_json::from_str(line).map_err(|e| format!("unreadable option list: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vuretro_core::protocol::StateAction;

    fn line(e: &Event) -> String {
        format!("{}\n", protocol::encode(e))
    }

    #[test]
    fn events_are_picked_out_of_what_a_runner_and_its_core_print() {
        let ready = Event::Exit { played_secs: 5 };
        let saved = Event::State {
            action: StateAction::Saved,
            slot: 1,
        };
        let text = format!(
            "core says hello\n{}\n   \n{{not json\n{}partial{}",
            protocol::encode(&ready),
            line(&saved),
            line(&Event::Progress { played_secs: 30 }), // on the end of a half line: lost
        );
        let (to, from) = channel();
        read_events(text.as_bytes(), to);
        assert_eq!(from.try_iter().collect::<Vec<_>>(), vec![ready, saved]);
    }

    #[test]
    fn a_closed_receiver_ends_the_reading() {
        let (to, from) = channel();
        drop(from);
        let many = line(&Event::Progress { played_secs: 1 }).repeat(1000);
        read_events(many.as_bytes(), to); // returns instead of reading on for nothing
    }

    /// A real process that says things and fails, run through the same code as the runner.
    #[cfg(unix)]
    fn process(script: &str) -> Runner {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let (to, reports) = channel();
        let out = child.stdout.take().unwrap();
        Runner {
            child,
            log: PathBuf::from("/nonexistent/runner.log"),
            reports,
            reader: Some(std::thread::spawn(move || read_events(out, to))),
        }
    }

    #[cfg(unix)]
    fn finished(r: &mut Runner) -> Exit {
        for _ in 0..200 {
            if let Some(exit) = r.poll().unwrap() {
                return exit;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the process did not end");
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_runner_is_explained_by_what_it_reported_not_by_the_log() {
        let say = protocol::encode(&Event::Error {
            message: "neogeo.zip is missing".into(),
        });
        let mut r = process(&format!("echo noise; printf '\\n%s\\n' '{say}'; exit 1"));
        let exit = finished(&mut r);
        assert!(!exit.success);
        assert_eq!(exit.detail, "neogeo.zip is missing");
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_runner_that_said_nothing_falls_back_to_the_log_or_the_status() {
        let mut r = process("exit 3");
        let exit = finished(&mut r);
        assert!(!exit.success);
        assert!(exit.detail.contains("exited with"), "{}", exit.detail);
    }

    #[cfg(unix)]
    #[test]
    fn waiting_gives_everything_the_runner_said_before_it_ended() {
        let say = |e: &Event| protocol::encode(e);
        let progress = Event::Progress { played_secs: 30 };
        let exit = Event::Exit { played_secs: 41 };
        let mut r = process(&format!(
            "printf '\\n%s\\n\\n%s\\n' '{}' '{}'",
            say(&progress),
            say(&exit)
        ));
        r.wait();
        assert_eq!(r.events(), vec![progress, exit]);
        assert!(r.events().is_empty(), "taken once");
    }
}
