use super::bundle;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use vuretro_core::model::CoreOptionDef;
use vuretro_core::paths::Paths;
use vuretro_core::ports::{CoverJob, Exit, GameProcess, LaunchSpec, Launcher, NativeSpec};
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
    /// What ends the program's own helpers (see `NativeSpec::stop`), with the environment to run
    /// it in.
    stop: Option<(Vec<String>, Vec<(String, String)>)>,
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

    fn stop(&mut self) {
        self.cleanup();
        let _ = self.child.kill();
    }

    fn wait(&mut self) {
        let _ = self.child.wait();
        // what the program started besides itself (Wine's server, a mounted disc image) is
        // ended with it, however it ended
        self.cleanup();
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
    /// Run the command that ends what the program started besides itself, once.
    fn cleanup(&mut self) {
        let Some((argv, env)) = self.stop.take() else {
            return;
        };
        if let Some((program, args)) = argv.split_first() {
            let _ = Command::new(program)
                .args(args)
                .envs(env.iter().map(|(k, v)| (k, v)))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }

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
            stop: None,
        }))
    }

    fn spawn_native(&self, spec: &NativeSpec) -> Result<Box<dyn GameProcess>, String> {
        let log_path = self.paths.native_log();
        if let Some(dir) = log_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
        // Start it from the folder it asks for, or its own: games of this kind look for their
        // data next to them.
        let mut cmd = Command::new(&spec.exe);
        let cwd = spec
            .cwd
            .as_deref()
            .or_else(|| spec.exe.parent().filter(|d| !d.as_os_str().is_empty()));
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.envs(spec.env.iter().map(|(k, v)| (k, v)));
        let child = cmd
            .args(&spec.args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone().map_err(|e| e.to_string())?))
            .stderr(log)
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", spec.exe.display()))?;
        // it speaks no protocol: nothing to read, and the log explains a failure
        let (_to, reports) = channel();
        Ok(Box::new(Runner {
            child,
            log: log_path,
            reports,
            reader: None,
            stop: spec.stop.clone().map(|argv| (argv, spec.env.clone())),
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
            stop: None,
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

    #[cfg(unix)]
    #[test]
    fn a_native_game_that_ends_by_itself_has_its_stop_command_run_once() {
        let dir = std::env::temp_dir().join(format!("vuretro-end-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log");
        let launcher = ProcessLauncher::new(Paths::new(dir.to_str().unwrap()));
        let mut p = launcher
            .spawn_native(&NativeSpec {
                exe: "/bin/sh".into(),
                args: vec!["-c".into(), "exit 0".into()],
                cwd: None,
                env: Vec::new(),
                stop: Some(vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    format!("echo ran >> '{}'", log.display()),
                ]),
            })
            .unwrap();
        p.wait();
        p.stop(); // asked after it ended: the command is not run again
        p.wait();
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "ran\n");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_native_game_starts_in_the_folder_and_with_the_environment_it_asked_for() {
        let dir = std::env::temp_dir().join(format!("vuretro-spawn-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("out.txt");
        let launcher = ProcessLauncher::new(Paths::new(dir.to_str().unwrap()));
        let script = format!(
            "pwd > '{}'; echo \"$WINEPREFIX\" >> '{}'",
            out.display(),
            out.display()
        );
        let mut process = launcher
            .spawn_native(&NativeSpec {
                exe: PathBuf::from("/bin/sh"),
                args: vec!["-c".into(), script],
                cwd: Some(dir.clone()),
                env: vec![("WINEPREFIX".into(), "/some/prefix".into())],
                stop: None,
            })
            .unwrap();
        process.wait();
        let text = std::fs::read_to_string(&out).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            std::fs::canonicalize(lines[0]).unwrap(),
            std::fs::canonicalize(&dir).unwrap()
        );
        assert_eq!(lines[1], "/some/prefix");
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// With a real Wine and the game set up in the app's data folder (`VURETRO_DATA`): the
    /// game starts, and stopping it ends it and everything Wine started for it.
    #[test]
    #[ignore]
    fn stopping_the_real_game_ends_it() {
        let data = std::env::var("VURETRO_DATA").unwrap();
        let paths = Paths::new(&data);
        let disc = PathBuf::from("/x/roadrash.iso");
        let spec = crate::adapters::wine_installer::prepare(&paths, &disc, None, &|_| {}).unwrap();
        println!("{spec:?}");
        let launcher = ProcessLauncher::new(paths);
        let mut p = launcher.spawn_native(&spec).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(20));
        assert!(p.poll().unwrap().is_none(), "the game is running");
        p.stop();
        for _ in 0..20 {
            if p.poll().unwrap().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        assert!(p.poll().unwrap().is_some(), "the game ended");
    }
}
