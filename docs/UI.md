# Thiết kế UI

Bản thiết kế tương tác (canvas): https://claude.ai/artifact/TFnBEM1GyQF9oLw4edGJAn

## Phong cách

- Nền tối `#090B10`, surface `#0F121A` / `#131724`, viền `#1C2130`
- Accent amber `#FFB347` (đổi được), màu riêng mỗi hệ máy: Neo Geo amber, PS cyan `#5AD1FF`, SNES tím `#B79CFF`, GBA xanh lá `#9BE36B`
- Font: Space Grotesk (tiêu đề), Inter (nội dung), JetBrains Mono (tên file, phím), Press Start 2P (điểm nhấn pixel)
- Hiệu ứng CRT: scanline `repeating-linear-gradient` trên cover và màn preview

## Màn hình

### Library
Sidebar (Thư viện: Tất cả / Gần đây / Yêu thích; Hệ máy kèm số game; thẻ thư mục ROM) · thanh tìm kiếm · hero "Tiếp tục chơi" (CRT preview, thời gian chơi, slot save) · lưới cover sắp xếp Gần đây / A–Z / Năm · panel chi tiết (save slot, core, file, BIOS, nút Chơi khóa khi thiếu BIOS).

### Cores & BIOS
Thẻ mỗi hệ máy (core, đuôi file, bật/tắt, trạng thái, số game, BIOS) · danh sách BIOS đã có / thiếu · vùng kéo thả BIOS (kiểm tra MD5 rồi chép vào `system/`).

### Điều khiển
Thiết bị (bàn phím, tay cầm, gán P1/P2, deadzone, rung) · sơ đồ nút theo hệ máy (tủ arcade Neo Geo, tay PS) bấm để gán lại · bảng gán phím · phím tắt chung.

## Việc cần chỉnh khi dựng thật (đa nền tảng)

- Bỏ chi tiết riêng macOS trong mock: "Renderer: Metal", nhãn "arm64", `⌘K` / `⌘F`
- Phím tắt hiển thị theo OS: `⌘` trên macOS, `Ctrl` trên Windows/Linux
- Đường dẫn BIOS lấy từ data dir của từng OS (xem ARCHITECTURE.md)
