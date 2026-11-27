//! Windows games on Windows: no Wine. The disc image is mounted as a drive, which is what the
//! game looks for (Road Rash stops with "Could not find any CD-ROM drive" without a CD), and the
//! game runs as it is. Road Rash is set up as it is under Wine: its files are copied, a stand-in
//! replaces the driver of the Sound Blaster AWE32, cnc-ddraw draws its window, and the entries of
//! the registry that its installer writes are made.
//!
//! Written without a Windows computer at hand: what can be checked without one (the commands, the
//! reading of what they say) is tested; the rest has not been run.

use super::native_installer::run;
use super::wine_installer::{
    add_window_frame, aweman32_stub, copy_game, find_program, game_id, patch_road_rash, recipe_for,
    road_rash_registry, Recipe,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use vuretro_core::paths::Paths;
use vuretro_core::ports::NativeSpec;

/// A PowerShell command (a script as text), without its profile and without questions.
fn powershell(script: &str) -> Vec<String> {
    [
        "powershell.exe",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        script,
    ]
    .map(String::from)
    .to_vec()
}

/// `text` as a text in single quotes for PowerShell.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The script that mounts a disc image and says the letter of its drive.
fn mount_script(iso: &Path) -> String {
    format!(
        "$i = Mount-DiskImage -ImagePath {} -PassThru; ($i | Get-Volume).DriveLetter",
        quoted(&iso.to_string_lossy())
    )
}

/// What ends the mount when the game is over or stopped.
fn dismount_command(iso: &Path) -> Vec<String> {
    powershell(&format!(
        "Dismount-DiskImage -ImagePath {} | Out-Null",
        quoted(&iso.to_string_lossy())
    ))
}

/// The letter of a drive from what the mount script printed.
fn drive_letter(output: &str) -> Result<char, String> {
    let text = output.trim();
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Ok(c.to_ascii_uppercase()),
        _ => Err(format!(
            "Windows did not say which drive the disc is: \"{text}\""
        )),
    }
}

/// Mount the disc image and give the letter of its drive. One already mounted is found again.
fn mount(iso: &Path) -> Result<char, String> {
    let argv = powershell(&mount_script(iso));
    let out = Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|e| format!("cannot start PowerShell: {e}"))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "Windows could not mount this disc image: {}",
            why.trim()
        ));
    }
    drive_letter(&String::from_utf8_lossy(&out.stdout))
}

/// A path on the drive of the disc from the path that `find_program` gives on the drive `D:`.
fn on_drive(program: &str, letter: char) -> PathBuf {
    PathBuf::from(format!("{letter}:{}", program.trim_start_matches("D:")))
}

/// Make the game on `disc` ready to start and say how to start it. The first time Road Rash is
/// set up (its files are copied and the registry is written); every time the image is mounted.
pub fn prepare(
    paths: &Paths,
    disc: &Path,
    icon: Option<&[u8]>,
    log: &dyn Fn(&str),
) -> Result<NativeSpec, String> {
    if !disc.is_file() {
        return Err(
            "On Windows the game needs its disc image (it looks for the CD): choose the .iso"
                .into(),
        );
    }
    let recipe = recipe_for(disc)?;
    let root = paths.native_dir().join("pc").join(game_id(disc));
    let game = root.join("game");
    let done = root.join("ready");

    let (exe, cwd) = match recipe {
        Recipe::RoadRash => {
            if !done.is_file() {
                std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
                log("Copying the game's files from the disc");
                if game.exists() {
                    std::fs::remove_dir_all(&game).map_err(|e| e.to_string())?;
                }
                copy_game(disc, &game, log)?;
                std::fs::write(game.join("AWEMAN32.DLL"), aweman32_stub())
                    .map_err(|e| e.to_string())?;
                patch_road_rash(&game.join("ROADRASH.EXE"), icon, false, log);
                add_window_frame(paths, &game, "640x480", log);
                log("Writing the settings of the game");
                let here = format!("{}\\", game.display());
                for (i, command) in road_rash_registry(&here).iter().enumerate() {
                    // the first half is for the current user and must work; the machine's
                    // half needs an administrator and is a convenience
                    let hkcu = !command[2].starts_with("HKLM");
                    let done = run(Command::new(&command[0]).args(&command[1..]), &|_| {});
                    if hkcu && done.is_err() {
                        return Err(format!(
                            "could not write the settings of the game (step {i}): {}",
                            done.unwrap_err()
                        ));
                    }
                }
                std::fs::write(&done, "windows\n").map_err(|e| e.to_string())?;
            }
            (game.join("ROADRASH.EXE"), game.clone())
        }
        Recipe::Generic => (PathBuf::new(), PathBuf::new()),
    };

    log("Mounting the disc image");
    let letter = mount(disc)?;
    let (exe, cwd) = match recipe {
        Recipe::RoadRash => (exe, cwd),
        Recipe::Generic => {
            let drive = PathBuf::from(format!("{letter}:\\"));
            // the same search as under Wine, on the drive that the disc is mounted as
            let program = find_program(&drive)?;
            (on_drive(&program, letter), drive)
        }
    };
    Ok(NativeSpec {
        exe,
        args: Vec::new(),
        cwd: Some(cwd),
        env: Vec::new(),
        stop: Some(dismount_command(disc)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_disc_is_mounted_by_a_script_that_says_its_drive_letter() {
        let s = mount_script(Path::new("C:\\Games\\Road Rash's disc.iso"));
        assert!(s.contains("Mount-DiskImage -ImagePath 'C:\\Games\\Road Rash''s disc.iso'"));
        assert!(s.ends_with("DriveLetter"));
    }

    #[test]
    fn the_mount_ends_with_a_command_that_dismounts_that_image() {
        let argv = dismount_command(Path::new("C:\\a.iso"));
        assert_eq!(argv[0], "powershell.exe");
        assert!(argv.contains(&"-NoProfile".to_string()));
        assert!(argv
            .last()
            .unwrap()
            .contains("Dismount-DiskImage -ImagePath 'C:\\a.iso'"));
    }

    #[test]
    fn the_drive_letter_is_read_from_what_the_script_printed() {
        assert_eq!(drive_letter("e\r\n").unwrap(), 'E');
        assert_eq!(drive_letter(" F ").unwrap(), 'F');
        assert!(drive_letter("").is_err());
        assert!(drive_letter("Error: access denied").is_err());
        assert!(drive_letter("EF").is_err());
    }

    #[test]
    fn a_program_found_on_the_stand_in_drive_is_on_the_real_one() {
        assert_eq!(
            on_drive("D:\\Bin\\GAME.EXE", 'G'),
            PathBuf::from("G:\\Bin\\GAME.EXE")
        );
    }

    #[test]
    fn a_folder_is_not_enough_for_a_game_that_looks_for_its_cd() {
        let dir = std::env::temp_dir();
        let err = prepare(&Paths::new(dir.to_str().unwrap()), &dir, None, &|_| {}).unwrap_err();
        assert!(err.contains("disc image"), "{err}");
    }
}
