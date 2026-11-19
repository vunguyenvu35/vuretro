//! The native menu bar and what its items do.

use crate::platform::open_external;
use crate::state::Shell;
use tauri::{Emitter, Manager};

pub fn build(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
    let item = |id: &str, text: &str, accel: Option<&str>| {
        let b = MenuItemBuilder::with_id(id, text);
        match accel {
            Some(a) => b.accelerator(a).build(app),
            None => b.build(app),
        }
    };

    let settings = item("view-settings", "Settings…", Some("CmdOrCtrl+,"))?;
    let app_menu = SubmenuBuilder::new(app, "Vuretro")
        .about(None)
        .separator()
        .item(&settings)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file = SubmenuBuilder::new(app, "File")
        .item(&item("import-roms", "Import Games…", Some("CmdOrCtrl+O"))?)
        .item(&item(
            "import-folder",
            "Import Folder…",
            Some("CmdOrCtrl+Shift+O"),
        )?)
        .item(&item(
            "import-bios",
            "Import BIOS Files…",
            Some("CmdOrCtrl+B"),
        )?)
        .separator()
        .item(&item("link-folder", "Link ROM Folder…", None)?)
        .item(&item("unlink-folder", "Unlink ROM Folder", None)?)
        .separator()
        .item(&item("run-demo", "Run Demo", None)?)
        .separator()
        .item(&item("open-data", "Show Data Folder", None)?)
        .item(&item("open-bios", "Show BIOS Folder", None)?)
        .separator()
        .close_window()
        .build()?;

    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&item("find", "Find Game", Some("CmdOrCtrl+F"))?)
        .build()?;

    let view = SubmenuBuilder::new(app, "View")
        .item(&item("view-library", "Library", Some("CmdOrCtrl+1"))?)
        .item(&item(
            "view-recent",
            "Recently Played",
            Some("CmdOrCtrl+2"),
        )?)
        .item(&item("view-favorites", "Favorites", Some("CmdOrCtrl+3"))?)
        .separator()
        .item(&item("view-cores", "Cores & BIOS", Some("CmdOrCtrl+4"))?)
        .item(&item("view-controls", "Controls", Some("CmdOrCtrl+5"))?)
        .separator()
        .item(&item("refresh", "Refresh Library", Some("CmdOrCtrl+R"))?)
        .separator()
        .fullscreen()
        .build()?;

    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;

    let help = SubmenuBuilder::new(app, "Help")
        .item(&item("help-docs", "Vuretro Documentation", None)?)
        .item(&item("help-issue", "Report an Issue…", None)?)
        .separator()
        .item(&item("help-about", "About Vuretro", None)?)
        .item(&item("help-support", "Support Vuretro…", None)?)
        .build()?;

    MenuBuilder::new(app)
        .items(&[&app_menu, &file, &edit, &view, &window, &help])
        .build()
}

pub fn handle(app: &tauri::AppHandle, id: &str) {
    const REPO: &str = "https://github.com/vunguyenvu35/vuretro";
    let paths = app.state::<Shell>().app.paths().clone();
    match id {
        "open-data" => {
            let _ = std::fs::create_dir_all(paths.data_dir());
            open_external(&paths.data_dir().to_string_lossy());
        }
        "open-bios" => {
            let dir = paths.system_dir();
            let _ = std::fs::create_dir_all(&dir);
            open_external(&dir.to_string_lossy());
        }
        "help-docs" => open_external(&format!("{REPO}#readme")),
        "help-issue" => open_external(&format!("{REPO}/issues/new")),
        // Everything else is the page's business.
        _ => {
            let _ = app.emit("menu", id.to_string());
        }
    }
}
