# Kế hoạch vuretro

Mục tiêu: frontend chơi game retro có UI, chạy trên macOS, Windows và Linux.
Game chạy qua libretro core, app lo phần giao diện, quản lý thư viện, điều khiển.

## Phase 0: Khởi tạo ✅

- [x] Cargo workspace, CI matrix 3 OS (fmt, clippy, test, smoke test)
- [x] FFI libretro tối thiểu (`crates/libretro-sys`)
- [x] Script tải core theo OS (`cores/fetch-cores.sh`)
- [x] Core giả để test không cần ROM (`crates/test-core`)

## Phase 1: Runner chạy được game (đang làm)

- [x] Load core động (`libloading`), đăng ký đủ callback
- [x] Video: XRGB8888 / RGB565 / 0RGB1555 → cửa sổ (`minifb`)
- [x] Audio: `cpal` + resample tuyến tính, giới hạn độ trễ buffer
- [x] Input: bàn phím + tay cầm (`gilrs`), stick trái thay D-pad, 2 người chơi
- [x] Save state 4 slot (F5/F7/F6), reset, tua nhanh
- [x] Chế độ headless + screenshot cho CI
- [ ] Chạy thử thật: Metal Slug (FBNeo), Crash Bandicoot (PCSX ReARMed) trên cả 3 OS
- [ ] Core options (`GET_VARIABLE`) đọc từ file cấu hình theo core
- [ ] Đồng bộ theo audio (dynamic rate control) thay cho `set_target_fps`
- [ ] Lưu SRAM (`retro_get_memory_data`) khi thoát

## Phase 2: UI Tauri

- [ ] Khởi tạo Tauri v2 + Svelte trong `ui/` và `src-tauri/`
- [ ] Màn **Library** theo thiết kế (sidebar hệ máy, hero "Tiếp tục chơi", lưới cover, panel chi tiết)
- [ ] Màn **Cores & BIOS**: bật/tắt core, kiểm tra BIOS, kéo thả file BIOS
- [ ] Màn **Điều khiển**: chọn thiết bị, sơ đồ nút, gán phím, phím tắt
- [ ] Tauri command gọi `vuretro-library` (scan, bios) và spawn `vuretro-runner`
- [ ] Lưu metadata vào SQLite (thời gian chơi, lần cuối, yêu thích, slot save)

## Phase 3: Trải nghiệm chơi

- [ ] Renderer `wgpu` thay `minifb`: giữ tỷ lệ, integer scale, fullscreen
- [ ] Shader CRT (scanline, curvature) bật/tắt được
- [ ] Menu trong game (Esc): save/load, đổi slot, thoát về Library
- [ ] Remap phím lưu JSON theo `system → device → action`, ghi đè theo game
- [ ] Ảnh thumbnail cho save state

## Phase 4: Thêm hệ máy

- [ ] Bật SNES (snes9x), GBA (mGBA), Genesis (Genesis Plus GX)
- [ ] Đa đĩa PS1 qua `.m3u` + đổi đĩa (`SET_DISK_CONTROL_INTERFACE`)
- [ ] Cover art: người dùng tự thêm, hoặc lấy từ nguồn metadata (ScreenScraper…)

## Phase 5: Phát hành

- [ ] macOS: `.dmg` ký + notarize (universal arm64/x86_64)
- [ ] Windows: MSI/NSIS ký code
- [ ] Linux: AppImage, Flatpak
- [ ] Auto-update (Tauri updater) qua GitHub Releases
- [ ] Rà license từng core trước khi bundle

## Quyết định đã chốt

| Hạng mục | Chọn | Ghi chú |
|---|---|---|
| Ngôn ngữ | Rust | Không GC, FFI C gọn, cross-platform |
| UI | Tauri v2 | Dựng lại thiết kế bằng HTML/CSS, bundle nhỏ |
| Chạy game | Process riêng `vuretro-runner` | Core crash không kéo sập UI |
| Window/video (Phase 1) | `minifb` | Pure Rust, không cần SDL; thay bằng `wgpu` ở Phase 3 |
| Audio | `cpal` | CoreAudio / WASAPI / ALSA |
| Tay cầm | `gilrs` | Dùng SDL GameController DB để map tay cầm |
| Core Neo Geo | FBNeo | License phi thương mại → app phải miễn phí |
| Core PS1 | PCSX ReARMed | Nhẹ, chạy tốt arm64; tránh DuckStation (license) |
