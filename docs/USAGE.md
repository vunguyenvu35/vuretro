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

A core is downloaded **the first time a game needs it**: press **Play** on a game whose core is not installed and the app fetches it (a progress box shows how far it is), then starts the game. You can also install cores yourself in **Cores & BIOS**.

**No internet?** Prepare the cores once on a computer that has it, with `make cores-offline` (add `ARGS=--all` for the alternative cores too). It saves the core zips in `cores/offline/<platform>/`. Copy that folder next to the app (or to `<data dir>/cores-offline`, or point `VURETRO_CORES_OFFLINE` at it) and the cores install from it with no connection. The zips are not committed to the repository: they are large (the MAME core is ~100 MB), differ per OS and CPU, and some cores have licences that do not allow passing them on.

On first launch the app shows a welcome screen. **Try the demo** runs a built-in test program (a square you move with the arrow keys), so you can check video, sound and controls before adding any game.

## 2. Add games

Everything you add is **copied** into vuretro's own library. Your original files are never changed or deleted.

- **Drag and drop** files or folders onto the window.
- **File ▸ Import Games…** (`⌘O` / `Ctrl+O`) for files, **Import Folder…** (`⌘⇧O` / `Ctrl+Shift+O`) for a whole folder. Subfolders are searched.
- **Link ROM Folder…** reads a folder in place without copying. **Unlink ROM Folder** stops reading it. Games you imported stay.

How vuretro decides which system a file is for:

1. A folder named after the system wins (`neogeo`, `psx`, `snes`, `gba`, and common aliases like `Neo-Geo (MVS)`).
2. Otherwise a file extension that belongs to only one system decides (`.chd` is PlayStation, `.gba` is Game Boy Advance).
3. **`.zip`, `.7z` and `.rar` can hold games for any system.** Select the system in the sidebar before importing, or keep the file in a system-named folder.

### Patches (translations and ROM hacks)

Fan translations and ROM hacks come as small **IPS**, **UPS** or **BPS** patch files. **Right-click a game ▸ Apply patch…**, pick the patch, and vuretro adds a patched copy to the library next to the game, named `<game> [<patch>].<ext>`. **The original game is not changed**, and the patched copy is a game like any other. Patches check that they fit your copy of the game; if they do not, you are told and nothing is added. Cartridge games only: a game inside a `.zip`, or a disc image, has to be unpacked and patched outside vuretro. Applying the same patch twice does not add a second copy.

A disc that comes in a `.zip`, `.7z` or `.rar` (a `.cue` with its `.bin`, or a `.chd`, `.iso`…) is unpacked for you: import the archive as it is. If its `.cue` names a different `.bin` than the one packed (common in shared dumps), the sheet is mended. Archives of arcade or Neo Geo ROM sets are kept as they are. `.cue` and `.m3u` files bring the `.bin` / disc files they point to. If one is missing, the import is refused and nothing is copied. If a BIOS file is among what you import (a file you picked, or one inside a folder), vuretro installs it as a BIOS and says so, so a whole collection can be imported in one go. You can also add BIOS files yourself as described next.


### PC (Windows) games

Put the disc image (`.iso`) of a Windows game in a folder named `pc` (or `windows`) in your ROM folder; it is listed as **PC (Windows)**. The first Play fetches Wine (about 230 MB, from `github.com/Gcenx`, checked against a fixed SHA-256; an existing Wine on the computer is used instead) and sets the game up from your disc, which takes a few minutes; after that Play starts it at once. On an Apple Silicon Mac it needs Rosetta 2. Vuretro's save states, rewind and macros do not reach these games. Any disc is tried: the whole disc is copied, and the program named in its `autorun.inf` (or its only program) is started. Games that draw with DirectDraw open in an ordinary window with the Mac's title bar and buttons, drawn by cnc-ddraw (`github.com/FunkyFr3sh/cnc-ddraw`, MIT; fetched when the game is set up and checked against a fixed SHA-256); the mouse is not kept inside the window (Ctrl+Tab frees it if a game grabs it); if it cannot be fetched the game runs on a Wine desktop without a title bar. Road Rash (1996) is known and gets what it needs (a stand-in for the Sound Blaster AWE32 driver, its registry entries). On Windows there is no Wine: the disc image is mounted as a drive (the game looks for its CD) and the game runs as it is; Road Rash's files are copied and its settings written to the registry of the current user (the same as its installer does). This part is written but has not been run on a Windows computer. A game that needs more than copying is described by a recipe in `crates/library/data/pc_games.json`: how to recognise its disc, the folder of the disc that is the game, stand-ins for drivers, entries for the registry, the title of its window, libraries to install with `winetricks` (the `winetricks` on your computer is used: `brew install winetricks`), DLL overrides, and `"renderer": "dxvk"` for games that use Direct3D 10 or 11 (DXVK for macOS is fetched and checked against a fixed SHA-256; Apple's D3DMetal is not included). A disc whose program cannot be told (several programs, no `autorun.inf`) has **Run the disc's installer…** in its details: the installer opens in its own window, and when it is closed the program it left is what Play starts. That last part has been tested with its pieces but not with a real installer.

Many Windows games need more than this, so other discs are best-effort; one with several programs and no `autorun.inf` says it cannot tell which to start. While a game is open the app shows **Playing … Stop** in the corner; Stop ends the game, and so does **⌘⌥Q** (Ctrl+Alt+Q elsewhere) from any program: the keys are taken only while a game is open, and if another program holds them the Stop button still works. The Dock shows the icon inside the game's program: for Road Rash the copy of the program on your computer gets that icon redrawn as a Mac icon (a rounded tile; the disc and the original file are not touched); for other games the copy of the game's program gets the same icon when it can be patched (a packed or protected program is left as it was and keeps its own, which may look garbled). The Dock still names the program `wine`, and the title of Road Rash's window is "Road Rash — Wine" to say what runs it.

### Native games (programs of their own)

Some games are not run by a core but are programs of their own, such as the native port of Road Rash. They are in **Native games** in the sidebar, and are started as they are: they keep their own saves and controls (Vuretro's save states, rewind and macros do not reach them).

- **Add a game from a folder…** takes the folder of a game and finds the program in it. A folder with a `game.json` (`{"name": "My game", "exe": "play", "args": ["--data", "{dir}/data"]}`) is read as it says; `{dir}` stands for the folder.
- Games kept under `<data dir>/native/<name>/` with a `game.json` are offered by themselves.
- **Road Rash:** the page has an **Install from my disc image…** button. It needs your own Road Rash PC disc (the `.iso`, or the folder it was unpacked to). It downloads the source of the native port from `github.com/arkits/road-rash`, builds it on your computer (a few minutes) and reads the game's data from your disc, then adds the game with the title picture of the disc as its cover. It needs `git`, `cmake`, a C++ compiler and SDL 3: on a Mac the page offers to install `cmake` and `sdl3` with Homebrew (`xcode-select --install` gives the compiler and git). Vuretro includes none of the game or its data, and the port has no licence that lets it be passed on, so it is built on your computer rather than shipped.
- The same, from a terminal: `cargo run --release -p vuretro-library --bin vuretro-install-road-rash -- /path/to/ROADRASH.iso`.

## 3. Add BIOS files

Open **Cores & BIOS**. Drop BIOS files on the window or click the drop zone. vuretro checks each file by its MD5, so a renamed dump still works, and installs it with the right name.

| System | File | Needed? |
|---|---|---|
| Neo Geo | `neogeo.zip` (MVS / AES BIOS set) | Required |
| PlayStation | `scph5501.bin` (NTSC-U), `scph5500.bin` (NTSC-J), `scph5502.bin` (PAL) | Optional (without them the core uses a built-in BIOS that is less compatible) |
| Arcade (MAME) | `pgm.zip` (PolyGame Master BIOS) | Only for PGM games such as Oriental Legend |
| SNES | none | |
| Game Boy Advance | `gba_bios.bin` | Optional (mGBA has a built-in replacement) |

Each system card shows **Ready**, **BIOS missing**, **Core missing** or **Off**, and has a switch to turn the system off.

## 4. Play

Select a game and press **Play** (or double-click it, or press `Enter`). If **Play** is locked, the reason is shown: a missing BIOS or core, or the system is switched off.

**One game at a time.** While a game is open, pressing **Play** on another one shows "<game> is already open" instead of opening a second window. Close the game first (`Esc`, then `Esc` again to confirm).

The Library has **All games**, **Recently played** and **Favorites**, plus one entry per system. Search with `⌘K` / `Ctrl+K` (or `⌘F`). Sort by recent, A–Z or size, and switch between grid and list. **Pictures.** A game shows the picture it already has: cover art you chose or downloaded, or a preview made before. Only a game with none gets a preview made, by running it for a short while in the background, and only while its card is on screen (or about to be): scroll to other games and those are made next, the ones you scrolled past are dropped. Closing a game does **not** replace its picture with the last frame.

**Right-click** a game for *Play*, *Add to favorites*, *Show in folder*, *Select…* and *Remove from library…*.

### Choosing the core of one game

A system runs on one core (chosen on its card in **Cores & BIOS**), but some systems have several, for example Arcade: `mame2003_plus`, `mame2003` and `mame`. A ROM set fits one version, so a game that does not run on the system's core can have its own: select the game and open **Core ▸ Runs on** in its details. Only that game changes; the others keep the system's core. The core has to be installed first (**Cores & BIOS**).

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
| `Esc` | Ask "Quit the game?": the game waits; `Esc` again quits, `Enter`, `Space` or `N` go on playing |
| `P` or `F3` | Pause / resume (dims the picture and mutes the sound) |
| `F5` / `F7` | Save state / load state, in the current slot |
| `F6` | Next save slot (there are 10) |
| `F2` | Reset |
| Hold `Tab` | Fast-forward (2×, 3×, 4× or 8×, set in Settings) |
| `F1` | Fast-forward on / off: it stays on until you press it again |
| `Shift`+`F1` | Slow motion on / off (half speed, no sound) |
| `Backspace` (hold) | Rewind, when switched on in Settings |
| `F8` | Sound on / off |
| `F9` / `F10` | Smaller / bigger window (remembered for next time) |
| `Alt`+`Enter` | Full screen on / off |
| `F4` | CRT scanlines on or off (remembered) |
| `Shift`+`F4` | LCD pixel grid on or off (remembered). It replaces CRT scanlines, and the other way round |
| `Shift`+`F9` | Smooth pixels (Scale2x) on or off (remembered). It combines with CRT and LCD |

| `F11` | Change disc (multi-disc games, e.g. a PlayStation `.m3u`) |
| `F12` | Screenshot |

**Speed.** While fast-forwarding, the sound is muted by default; turn on **Sound while fast-forwarding** in Settings to hear it faster and higher. The **Speed** menu of the game window (macOS and Windows) changes the fast-forward speed for next time.

**Carry on where I left off.** Switch it on in Settings and vuretro saves the game when you close it, and loads it again the next time you start that game. Press `F2` to start over, or start from a save slot in the game's details. It needs a core with save states.

**Sharper PlayStation (GPU).** PlayStation games are drawn by `pcsx_rearmed` on the processor, at the console's own 320×240. Two other cores draw with the graphics card and can render at 2× to 16× that size, which is much sharper: **Beetle PSX HW** (`mednafen_psx_hw`) and **SwanStation** (`swanstation`). In **Cores & BIOS** choose the core of PlayStation (or, in a game's details, **Runs on**) and install it, then open its **Options**: for Beetle PSX HW set **Renderer** to `hardware` and raise **Internal GPU Resolution** (2× is a good start); for SwanStation raise **Internal Resolution Scale**. Turn **Scale2x** off for PlayStation: it is skipped for large pictures anyway, but it works on the processor. A save state belongs to the core that made it, so states made with one PlayStation core do not load in another.

Tested on a MacBook Pro (Intel Core i9, Intel UHD 630 and AMD Radeon Pro 5500M) with Road Rash: **Beetle PSX HW** drew the intro, menus and the race correctly. **SwanStation's OpenGL renderer** drew polygons that have a texture as white (washed-out menus and a white road and sky) while flat colours were right; set its **GPU Renderer** to `Software` to get the right picture from it (it ran at about 30 fps in busy scenes). Another Mac or graphics driver may behave differently.

**Full screen** is a borderless window the size of your display. It is available on macOS and Windows; on Linux the game stays in a window for now. **Whole-number scaling** (Settings) enlarges the picture by 2×, 3× … only, with black borders, so pixels stay sharp. **Smooth when enlarged (bilinear)** (Settings, general or per system) does the opposite: it softens the picture instead of showing big square pixels, which suits 3D and video games such as PlayStation. It applies the next time a game starts.

### Gamepad shortcuts

Hold **Select** and press another button. While you do, the game does not see those presses.

| With Select held | Action |
|---|---|
| Start | Quit the game |
| R1 / L1 | Save state / load state |
| D-pad → / ← | Next / previous save slot |
| R2 (hold) | Fast-forward |
| L2 (hold) | Rewind |
| ○ (East) | Fast-forward on / off |
| △ (North) | Pause / resume |
| □ (West) | Screenshot |
| ✕ (South) | Sound on / off |

**Arcade coins.** Neo Geo and MAME games ignore Start until a coin is in (`5` or `Right Shift`, the Select button), so Start alone shows "INSERT COIN". By default Start is just Start and the coin is the Select button (`5` or `Right Shift`). If you would rather not insert coins by hand, switch on **Insert a coin when Start is pressed** (Settings, off by default): every press of Start then puts a coin in first, waits a moment (about a quarter of a second) and lets Start through, so Start alone begins a game. If you press Start at a moment the game is not ready, a coin may be spent without the game starting; press Start again. It does nothing on consoles, and can be set for all games, one system or one game.

**Vibration.** Games that ask for rumble make your gamepad vibrate (Settings ▸ **Gamepad vibration**). It works on Windows and Linux with gamepads that support it; macOS does not offer it to apps yet.

On macOS and Windows the game window also has **Game**, **State** and **Size** menus with the same actions. A short message about what just happened appears in the title bar.

The default button layout is in the [README](../README.md#default-controls). Gamepads (Xbox, DualSense, Switch Pro and others) are detected automatically.

## 6. Controls

Open **Controls** to choose the device for Player 1 and Player 2 (automatic, keyboard only, one specific gamepad, or off), swap players, see which gamepad is which (press a button and its row lights up), and remap every button. Click **+ Set** / **+ Add**, then press the key or pad button. You can bind several keys or buttons to one action. You can also set the stick dead zone and whether the left stick acts as the D-pad.

Changes save at once and apply the next time a game starts. `Esc`, `Tab`, `Backspace`, `P` and `F1`–`F12` are game hotkeys and cannot be used as buttons.

**Diagonal keys.** Below the buttons is a **Diagonal keys** section (keyboard only): one key for ↖, ↗, ↙ or ↘ (see [MACROS.md](MACROS.md)). It presses both directions at once, so you can play diagonals on a keyboard without a numpad (a TKL keyboard) by choosing four keys you can reach with your arrow hand. Holding two arrow keys together still works as well. In a macro, **Record** understands these keys too.

## 7. Macros

Open **Macros** to make one key or gamepad button do more than one thing:

- **Combo**: while you hold the trigger, several buttons are pressed together.
- **Turbo**: while you hold the trigger, the chosen buttons are tapped over and over (2 to 30 taps per second).
- **Sequence**: pressing the trigger plays a list of steps, each a set of buttons held for some milliseconds. Use **● Record** to capture a move as you play it, or add steps by hand. A sequence can repeat while the trigger is held.

Macros can be **global** or belong to **one game**. A game's own macros run together with the global ones. They load when a game starts.

### Moves and combos (fighting games)

Choose **+ Move** to play a special move or a whole combo with one key: type it in fighting-game notation (`236P`, `j.D, cl.C > 236P`), set a trigger key, and the app turns it into a sequence. Moves can be set per character with **profiles**. The full guide, with the notation, facing, diagonal keys and troubleshooting, is in [MACROS.md](MACROS.md).

## 8. Settings

Settings apply the next time a game starts.

- **Window size**: a multiple of the game's own resolution (1× to 8×).
- **Smooth motion**: match your screen's refresh rate. This removes the small periodic stutter on 60 Hz displays; the game may run up to 5% faster than the original.
- **Start in full screen**, **Whole-number scaling**, **CRT scanlines**, **LCD pixel grid**, **Smooth pixels (Scale2x)**, **Smooth when enlarged (bilinear)**.
- **Pause when the window loses focus**: stops the game while you are in another app.
- **Rewind**, **Fast-forward speed**, **Sound while fast-forwarding**, **Carry on where I left off**, **Insert a coin when Start is pressed** (arcade games), **Gamepad vibration**.
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
| No preview image for a game | Previews are made one at a time, only for the games on screen; wait a moment or scroll a little. A game with a missing BIOS or core gets none. Some games never produce a good frame: choose a picture with **Set cover image…**. |
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

- **Settings per system:** window size, CRT scanlines, LCD grid, whole-number scaling, rewind, fast-forward speed and carrying on can be set for one system (a Game Boy wants a bigger window than a PlayStation). A customised row is tagged **customised** and has a **Use general** link. Anything you do not change follows **All games**, even if you change it there later. Sound can be turned off for one system or one game with the **Sound** switch; the volume and the other settings stay general.
- **Recommended look:** with a system selected, Settings shows **Use the recommended look for <system>**. It turns on whole-number scaling (sharp pixels) and, depending on the system, the **LCD pixel grid** (Game Boy, Game Boy Color, Game Boy Advance, Game Gear, Lynx, Neo Geo Pocket, WonderSwan, Nintendo DS) or **CRT scanlines** (Neo Geo, SNES, NES, Genesis, Master System, PC Engine, Atari 2600 and 7800, 32X, Saturn, PlayStation, arcade). Nothing changes until you press the button; only those settings are touched, and **Follow the settings for All games again** undoes it. Full screen is a general setting and is not part of it. 3D systems have no recommended look.
- **Controls per system:** a system uses the general key bindings until you change a button while that system is selected; from then on it has bindings of its own. **Use the general ones** removes them again.
- **Controls per game:** select a game and press **Key bindings for this game…** in its details (or pick the game in **Applies to** on Controls). Changing a button gives that game its own bindings. Order of use: the game's own, then its system's, then the general ones; **Use … ones** removes the game's own bindings again.
- **In a game:** the hotkeys for window size (F9 / F10), CRT (F4), LCD (Shift+F4) and fast-forward speed save to the place that already owns that setting: the system if it customised it, otherwise the general settings.
- **Settings per game:** select a game and open **Settings for this game** in its details. The same settings can be set for that one game; it overrides its system, which overrides All games. A game is told apart by its file name, so two copies with the same name share their settings.

The files are `config/overrides.json` and `config/controls/<system>.json` and `config/controls/games/<game>.json` in the data folder.

## Backup, restore and export

On **Settings** (All games), the **Backup and export** section has three buttons.

- **Back up…** saves one `.zip` file with your settings, favorites, collections, play time, controls, macros, cheats, core options, per-system and per-game choices and DAT files. Tick **Game saves and save states** to include saves (on by default) and **Cover pictures** to include previews and downloaded or chosen art (off by default: large, and previews can be made again). **Your games, BIOS files and cores are never included.**
- **Restore…** brings a backup back, for example on a new computer. It shows what the file holds and asks first. Before anything is replaced, a copy of your current state is saved in the `backups` folder of the data folder (`before-restore-<time>.zip`). Games, BIOS and cores are not touched. The app reloads afterwards.
- **Export…** writes the game list as a CSV spreadsheet: title, system, file, size, minutes played, last played, favorite, duplicate and collections.

Restoring only writes files of the kinds a backup holds, inside the data folder; anything else in the zip is ignored.

## Neo Geo: BIOS mode and coins

Neo Geo games do not start in the arcade (MVS) mode, which shows a long system check. Vuretro picks the mode from your `neogeo.zip`: **AES** (home console) if the set has the AES BIOS, otherwise the **Universe BIOS**. In AES mode there is no coin at all: press **Start** (`Enter` or `1`) and play. In the Universe BIOS, and in the arcade modes, a coin is needed first; **Insert a coin when Start is pressed** (Settings, on by default) does that for you.

To choose a mode yourself, open **Cores & BIOS → Core options** on the Neo Geo card and change **Neo-Geo mode** (`MVS_USA`, `MVS_EUR`, `MVS_JAP`, `AES_EUR`, `AES_JAP`, `UNIBIOS` or `DIPSWITCH`). The screen reads the options of the core by loading one of your Neo Geo games, so it needs at least one in the library. The value marked *default* is the one Vuretro picked for your BIOS; a value you choose is kept.

## Two players

The game must have a two-player mode. Open **Controls** and set the device of each player, then start the game (changes apply at the next start).

| You have | Player 1 | Player 2 |
|---|---|---|
| Two gamepads | Automatic | Automatic |
| One gamepad and a keyboard | the gamepad (choose it by name) | Keyboard only |
| Only a keyboard | Automatic | Keyboard only |
| One gamepad, the other person on the gamepad too | not possible: one gamepad is one player |

**Automatic** gives Player 1 the keyboard plus the first free gamepad, and every other player the next free gamepad. It never gives Player 2 the keyboard, so for a keyboard player set **Keyboard only**. A gamepad named explicitly is taken first; Automatic players share the rest in the order they were plugged in. **Controls → Connected gamepads** shows which pad is which (press a button and its row lights up), and **Swap players** exchanges the two.

**Two people on one keyboard** (default keys)

| | Player 1 | Player 2 |
|---|---|---|
| Move | arrow keys | `I` up, `K` down, `J` left, `L` right |
| B / A | `Z` / `X` | `N` / `M` |
| Y / X | `A` / `S` | `H` / `G` |
| Coin (Select) | `5` or `Right Shift` | `6` |
| Start | `1` or `Enter` | `2` |

Most keyboards register only a few keys at once, so two players pressing many buttons together (fighting games) can lose presses. A gamepad for at least one player avoids it. In arcade games, press Start `2` to join; **Insert a coin when Start is pressed** (Settings) takes care of the coin.

You can give a system or a single game its own buttons in the **Applies to** box on Controls.

## Help, about and support

- **Help ▸ About Vuretro** (also **Settings ▸ Help and support**) shows the version and links: GitHub, Report a bug, License and Ko-fi.
- **Help ▸ Support Vuretro…** explains how to support the project (Ko-fi) and other ways to help. Vuretro is free; nothing in the app is locked behind a donation.
- **Help ▸ Report an Issue…** opens the issue form on GitHub. Please say which game and system it is, and never attach ROMs or BIOS files.

The app only opens the project's own links (its GitHub page and its Ko-fi page); any other address is refused.
