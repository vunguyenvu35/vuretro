# vuretro

Frontend chơi game retro đa nền tảng (macOS, Windows, Linux), chạy game qua các **libretro core**.
Bắt đầu với Neo Geo (FBNeo) và PlayStation (PCSX ReARMed), mở rộng dần sang SNES, GBA…

> vuretro **không** đóng gói ROM hay BIOS. Người dùng tự cung cấp từ bản họ sở hữu.

## Trạng thái

Phase 1 (runner) đang làm. Xem [docs/PLAN.md](docs/PLAN.md).

| Thành phần | Trạng thái |
|---|---|
| `vuretro-runner`: load core, video, audio, bàn phím, tay cầm, save state | ✅ chạy được |
| `vuretro-library`: quét ROM, nhận diện hệ máy, kiểm tra BIOS (MD5) | ✅ có test |
| `vuretro-test-core`: core giả để test không cần ROM thật | ✅ |
| UI Tauri (Library, Cores & BIOS, Điều khiển) | ⏳ Phase 2 |

## Chạy thử

Yêu cầu: Rust stable mới nhất (`rustup update`). Linux cần thêm:
`sudo apt install libasound2-dev libudev-dev libx11-dev libxkbcommon-dev`.

```bash
# 1. Test nhanh bằng core giả (không cần ROM)
#    Linux: libvuretro_test_core.so · Windows: vuretro_test_core.dll
cargo build -p vuretro-test-core
echo test > /tmp/game.test
cargo run -p vuretro-runner -- \
  --core target/debug/libvuretro_test_core.dylib \
  --rom /tmp/game.test

# 2. Chạy game thật
./cores/fetch-cores.sh                      # tải fbneo + pcsx_rearmed vào cores/bin/
cp /path/to/neogeo.zip "<data dir>/system/" # BIOS, xem đường dẫn trong log khi chạy
cargo run --release -p vuretro-runner -- \
  --core cores/bin/fbneo_libretro.dylib \
  --rom ~/Games/roms/neogeo/mslug.zip

# 3. Quét thư mục ROM
cargo run -p vuretro-library --bin vuretro-scan -- ~/Games/roms "<data dir>/system"
```

`<data dir>` mặc định:
macOS `~/Library/Application Support/dev.vuvn.vuretro`,
Windows `%APPDATA%\vuvn\vuretro\data`,
Linux `~/.local/share/vuretro`.

## Điều khiển mặc định

| Phím | RetroPad | Neo Geo |
|---|---|---|
| Mũi tên | D-pad | Cần gạt |
| `Z` / `X` / `A` / `S` | B / A / Y / X | A / B / C / D |
| `Enter` hoặc `1` | Start | Start |
| `Right Shift` hoặc `5` | Select | Coin |
| `Q` / `E` / `W` / `R` | L / R / L2 / R2 | |

Phím tắt: `Esc` thoát · `F5` save state · `F7` load state · `F6` đổi slot · giữ `Tab` tua nhanh · `F2` reset.
Tay cầm (Xbox, DualSense, Switch Pro…) nhận tự động, tay thứ 2 là người chơi 2.

## Cấu trúc

```
crates/
  libretro-sys/   FFI types của libretro.h (phần dùng tới)
  runner/         vuretro-runner: cửa sổ, game loop, audio, input
  library/        registry hệ máy, quét ROM, kiểm tra BIOS
  test-core/      core giả: color bars + tiếng 440 Hz + ô vuông điều khiển được
cores/            script tải core từ libretro buildbot
docs/             kế hoạch, kiến trúc, thiết kế UI
```

## License

Mã nguồn vuretro: GPL-3.0-or-later.
Các core có license riêng (FBNeo: phi thương mại, PCSX ReARMed: GPLv2), xem [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#license).
