# Kiến trúc vuretro

## Tổng quan

```
┌──────────────────────────── vuretro (Tauri app) ─────────────────────────────┐
│  UI (HTML/CSS/Svelte)  ⇄  Tauri commands (Rust)  ⇄  vuretro-library          │
│  Library · Cores & BIOS · Điều khiển      scan ROM, BIOS, metadata (SQLite)   │
└───────────────────────────────┬──────────────────────────────────────────────┘
                                │ spawn process + args (core, rom, dirs)
                                ▼
┌──────────────────────────── vuretro-runner ──────────────────────────────────┐
│  game loop ── retro_run() ──▶ libretro core (.dylib/.dll/.so)                 │
│     ▲  input (minifb + gilrs)      │ video_cb / audio_batch_cb / input_state_cb │
│     └──────────── frontend state (global Mutex) ◀───────────┘                 │
│  video → cửa sổ        audio → ring buffer → cpal (resample)                  │
└──────────────────────────────────────────────────────────────────────────────┘
```

Runner chạy **process riêng**: core là code C bên thứ ba, nếu crash chỉ mất game đang chơi, UI vẫn sống. UI và runner trao đổi qua tham số dòng lệnh (sau này thêm stdout JSON để báo thời gian chơi, slot save).

## Crates

| Crate | Vai trò |
|---|---|
| `libretro-sys` | Kiểu FFI từ `libretro.h`, chỉ phần đang dùng. Thêm hằng số khi core cần. |
| `vuretro-runner` | Binary chạy 1 game. `core.rs` load symbol, `frontend.rs` callback C, `audio.rs`, `input.rs`. |
| `vuretro-library` | `registry.rs` (hệ máy → core, đuôi file, BIOS + MD5), `scan.rs`, `bios.rs`. Binary `vuretro-scan` in JSON. |
| `vuretro-test-core` | Core giả (cdylib) cho test và CI. |

## Luồng chạy một game

1. `Core::load`: `dlopen` core, kiểm tra `retro_api_version`, đăng ký 6 callback, gọi `retro_init`.
2. `load_game`: nếu core `need_fullpath` chỉ truyền đường dẫn, ngược lại đọc file vào RAM. Lỗi thường gặp ở bước này là thiếu BIOS.
3. `retro_get_system_av_info` → kích thước khung hình, fps, sample rate.
4. Mỗi vòng: đọc input → `retro_run` (tua nhanh: chạy 4 lần) → core gọi `video_cb` (đổi pixel sang `0RGB` u32) và `audio_batch_cb` (đẩy vào ring buffer) → vẽ cửa sổ.
5. Audio thread của `cpal` lấy mẫu từ ring buffer, resample tuyến tính từ tần số core sang tần số thiết bị. Buffer giới hạn ~250 ms, tràn thì bỏ mẫu cũ.

## Callback và state toàn cục

libretro callback không có con trỏ user data, nên state frontend nằm trong `OnceLock<Mutex<Frontend>>`. Quy tắc: **không giữ lock khi gọi vào core**, vì core sẽ gọi ngược lại callback và khóa lại mutex.

## Environment đã hỗ trợ

`GET_CAN_DUPE`, `SET_PIXEL_FORMAT`, `GET_SYSTEM_DIRECTORY`, `GET_CORE_ASSETS_DIRECTORY`, `GET_SAVE_DIRECTORY`, `GET_VARIABLE` (trả về mặc định), `GET_VARIABLE_UPDATE`, `SET_VARIABLES`, `SET_INPUT_DESCRIPTORS`, `SET_PERFORMANCE_LEVEL`. Lệnh khác trả `false` và log ở mức `trace` (`RUST_LOG=trace`).

## Nhận diện hệ máy khi quét ROM

1. File nằm trong thư mục tên hệ máy (`roms/neogeo/…`, `roms/psx/…`) → hệ máy đó.
2. Không có thư mục: dùng đuôi file nếu chỉ thuộc một hệ máy (`.chd` → PS1).
3. File nén (`.zip`, `.7z`) bắt buộc nằm trong thư mục hệ máy, vì có thể chứa game của bất kỳ hệ nào.
4. File BIOS (`neogeo.zip`, `scph*.bin`) bị loại khỏi danh sách game.

## Thư mục dữ liệu

Lấy qua crate `directories` (`ProjectDirs::from("dev", "vuvn", "vuretro")`):

| OS | Data dir |
|---|---|
| macOS | `~/Library/Application Support/dev.vuvn.vuretro` |
| Windows | `%APPDATA%\vuvn\vuretro\data` |
| Linux | `~/.local/share/vuretro` |

Bên trong: `system/` (BIOS), `saves/` (SRAM, `<rom>.state1..4`).

## License

- Mã nguồn vuretro: GPL-3.0-or-later (tương thích với core GPLv2+ khi phân phối kèm).
- **FBNeo**: license phi thương mại → vuretro phải phát hành miễn phí nếu bundle FBNeo.
- **PCSX ReARMed**: GPLv2. **snes9x**: phi thương mại. **mGBA**: MPL-2.0.
- Không dùng DuckStation: license hiện tại không cho tái phân phối tự do.
- ROM và BIOS: người dùng tự cung cấp, app không đóng gói và không hướng dẫn tải.
