# Using vuretro

This guide is for playing games. To build or work on the code, see [DEVELOPMENT.md](DEVELOPMENT.md).

vuretro does **not** include ROMs or BIOS files and does not download them. Use copies of games and hardware you own.

## 1. Get it running

vuretro is run from a source checkout for now (see the Quick start in the [README](../README.md)):

```bash
make setup     # once: Rust toolchain and checks
make cores     # once: downloads the emulator cores (Neo Geo, PlayStation, SNES, GBA)
make dev       # opens the app
```

On first launch the app shows a welcome screen. **Try the demo** runs a built-in test program (a square you move with the arrow keys), so you can check video, sound and controls before adding any game.

## 2. Add games

Everything you add is **copied** into vuretro's own library. Your original files are never changed or deleted.

- **Drag and drop** files or folders onto the window.
- **File ▸ Import Games…** (`⌘O` / `Ctrl+O`) for files, **Import Folder…** (`⌘⇧O` / `Ctrl+Shift+O`) for a whole folder. Subfolders are searched.
- **Link ROM Folder…** reads a folder in place without copying. **Unlink ROM Folder** stops reading it. Games you imported stay.

How vuretro decides which system a file is for:

1. A folder named after the system wins (`neogeo`, `psx`, `snes`, `gba`, and common aliases like `Neo-Geo (MVS)`).
2. Otherwise a file extension that belongs to only one system decides (`.chd` is PlayStation, `.gba` is Game Boy Advance).
3. **`.zip` and `.7z` can hold games for any system.** Select the system in the sidebar before importing, or keep the file in a system-named folder.

`.cue` and `.m3u` files bring the `.bin` / disc files they point to. If one is missing, the import is refused and nothing is copied. BIOS files are recognised and refused here; add them as described next.

## 3. Add BIOS files

Open **Cores & BIOS**. Drop BIOS files on the window or click the drop zone. vuretro checks each file by its MD5, so a renamed dump still works, and installs it with the right name.

| System | File | Needed? |
|---|---|---|
| Neo Geo | `neogeo.zip` (MVS / AES BIOS set) | Required |
| PlayStation | `scph5501.bin` (NTSC-U) | Required |
| PlayStation | `scph5500.bin` (NTSC-J), `scph5502.bin` (PAL) | Optional |
| SNES | none | |
| Game Boy Advance | `gba_bios.bin` | Optional (mGBA has a built-in replacement) |

Each system card shows **Ready**, **BIOS missing**, **Core missing** or **Off**, and has a switch to turn the system off.

## 4. Play

Select a game and press **Play** (or double-click it, or press `Enter`). If **Play** is locked, the reason is shown: a missing BIOS or core, or the system is switched off.

The Library has **All games**, **Recently played** and **Favorites**, plus one entry per system. Search with `⌘K` / `Ctrl+K` (or `⌘F`). Sort by recent, A–Z or size, and switch between grid and list. Preview images are made in the background by running each game for a short while, and appear after a moment.

**Right-click** a game for *Play*, *Add to favorites*, *Show in folder*, *Select…* and *Remove from library…*.

### Removing games

Removing deletes the copy in vuretro's library together with its play history and preview. Games read from a linked folder cannot be removed from here.

- Select several: `⌘`/`Ctrl`-click, `Shift`-click for a range, `⌘A` / `Ctrl+A` for all imported games, or the **Select** button.
- `Delete` or `⌫` removes the selected game(s) after you confirm.

### Keyboard in the library

| Keys | Action |
|---|---|
| `←` `↑` `↓` `→` | Move between games |
| `Enter` | Play the selected game |
| `F` | Add or remove favorite |
| `⌘K` / `Ctrl+K`, `⌘F` | Search |
| `Esc` | Clear the search |
| `⌘1` … `⌘5` | Library, Recently played, Favorites, Cores & BIOS, Controls |
| `⌘,` | Settings |
| `⌘R` | Refresh the library |

On Windows and Linux use `Ctrl` in place of `⌘`.

## 5. In the game window

| Key | Action |
|---|---|
| `Esc` | Quit the game |
| `P` or `F3` | Pause / resume (dims the picture and mutes the sound) |
| `F5` / `F7` | Save state / load state |
| `F6` | Next save slot (there are 4) |
| `F2` | Reset |
| Hold `Tab` | Fast-forward |
| `F8` | Sound on / off |
| `F9` / `F10` | Smaller / bigger window (remembered for next time) |
| `F4` | CRT scanlines on or off (remembered) |
| `F11` | Change disc (multi-disc games, e.g. a PlayStation `.m3u`) |
| `Backspace` (hold) | Rewind, when switched on in Settings |
| `F12` | Screenshot |

On macOS and Windows the game window also has **Game**, **State** and **Size** menus with the same actions. A short message about what just happened appears in the title bar.

The default button layout is in the [README](../README.md#default-controls). Gamepads (Xbox, DualSense, Switch Pro and others) are detected automatically.

## 6. Controls

Open **Controls** to choose the device for Player 1 and Player 2 (automatic, keyboard only, one specific gamepad, or off), swap players, see which gamepad is which (press a button and its row lights up), and remap every button. Click **+ Set** / **+ Add**, then press the key or pad button. You can bind several keys or buttons to one action. You can also set the stick dead zone and whether the left stick acts as the D-pad.

Changes save at once and apply the next time a game starts. `Esc`, `Tab`, `P` and `F2`–`F10` are game hotkeys and cannot be used as buttons.

## 7. Macros

Open **Macros** to make one key or gamepad button do more than one thing:

- **Combo**: while you hold the trigger, several buttons are pressed together.
- **Turbo**: while you hold the trigger, the chosen buttons are tapped over and over (2 to 30 taps per second).
- **Sequence**: pressing the trigger plays a list of steps, each a set of buttons held for some milliseconds. Use **● Record** to capture a move as you play it, or add steps by hand. A sequence can repeat while the trigger is held.

Macros can be **global** or belong to **one game**. A game's own macros run together with the global ones. They load when a game starts.

## 8. Settings

Settings apply the next time a game starts.

- **Window size**: a multiple of the game's own resolution (1× to 8×).
- **Smooth motion**: match your screen's refresh rate. This removes the small periodic stutter on 60 Hz displays; the game may run up to 5% faster than the original.
- **Pause when the window loses focus**: stops the game while you are in another app.
- **Game sound** and **Volume**.

## 9. Where things are stored

Everything lives in one data folder:

| OS | Folder |
|---|---|
| macOS | `~/Library/Application Support/dev.vuvn.vuretro` |
| Windows | `%APPDATA%\vuvn\vuretro\data` |
| Linux | `~/.local/share/vuretro` |

| Inside it | What |
|---|---|
| `roms/` | Your imported games |
| `system/` | BIOS files |
| `saves/` | Battery saves (`<game>.srm`) and save states (`<game>.state1` … `.state4`) |
| `screenshots/` | Screenshots from `F12` |
| `covers/` | Preview images |
| `config/` | Controls, macros, game settings, per-core options |
| `logs/runner.log` | Output of the last game (shown when a game fails to start) |

**File ▸ Show Data Folder** and **Show BIOS Folder** open these in your file manager. Back up `saves/` to keep your progress.

## 10. Troubleshooting

| Message or problem | What to do |
|---|---|
| **BIOS missing** | Add the BIOS for that system in **Cores & BIOS** (see the table above). |
| **Core … not installed** | Open Cores & BIOS and press **Install** on the system card (or run `make cores`). |
| **… is switched off** | Turn the system on in **Cores & BIOS**. |
| Game does not start | The app shows the end of `logs/runner.log`. Most often it is a missing BIOS or a damaged file. |
| A `.zip` is not added ("cannot tell which system") | Select the system in the sidebar and import again, or put the file in a folder named after the system. |
| "Game runner not found" | Run `make build` once, then restart the app. |
| No preview image for a game | They are generated in the background; some games only show a title screen later or never produce a colourful frame. |
| Gamepad not working | Check **Controls ▸ Connected gamepads**; press a button to see it react. Player 1 and 2 may be set to a different pad. |
| Crackling sound or stutter | Try **Smooth motion** in **Settings**; run from a terminal with `RUST_LOG=debug` for details. |

Found a bug? Use **Help ▸ Report an Issue…**. For security problems, see [SECURITY.md](../SECURITY.md).

## Official names and collections

- **Official names.** On **Cores & BIOS**, press **Add DAT files…** and choose No-Intro or Redump DAT files (XML). vuretro hashes your games (CRC32) and shows their official titles. A game that matches another file is marked as a duplicate in its details. DAT files are not included; download them from the project that publishes them.
- **Collections.** Select a game and type a name under **Collections**. Collections are listed in the sidebar and filter the library.

## 3D systems (N64, PSP, GameCube, PS2, 3DS)

These systems need a fast computer and a graphics renderer. On **Cores & BIOS** their cards are marked **3D system** and are **off by default**. The card tells you how your computer compares with what the system needs (memory and processor cores; the graphics chip cannot be measured), and turning one on asks for confirmation with that verdict.

**This version cannot draw 3D systems yet**, so their games stay locked even when the system is on. The switch, the check and the slow-game warning are already in place for when it can.

While any game runs, vuretro watches its speed. If the computer cannot keep up, the window title shows **(slow: about N fps)** and a message appears; it goes away when the game runs smoothly again.

For PS2 you also need a PS2 BIOS inside the `system/pcsx2/bios` folder of the vuretro data directory.

## Settings and controls per system

On **Settings** and **Controls**, the **Applies to** box at the top chooses what you are editing: **All games** or one system.

- **Settings per system:** window size, CRT scanlines and rewind can be set for one system (a Game Boy wants a bigger window than a PlayStation). A customised row is tagged **customised** and has a **Use general** link. Anything you do not change follows **All games**, even if you change it there later. Sound and the other settings stay general.
- **Controls per system:** a system uses the general key bindings until you change a button while that system is selected; from then on it has bindings of its own. **Use the general ones** removes them again.
- **In a game:** the hotkeys for window size (F9 / F10) and CRT (F4) save to the place that already owns that setting: the system if it customised it, otherwise the general settings.
- **Settings per game:** select a game and open **Settings for this game** in its details. The same three settings can be set for that one game; it overrides its system, which overrides All games. A game is told apart by its file name, so two copies with the same name share their settings.

The files are `config/overrides.json` and `config/controls/<system>.json` in the data folder.
