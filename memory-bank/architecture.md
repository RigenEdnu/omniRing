# Architecture: omniRing

## 1. Directory & File Structure

```
omniRing/
├── SKILL/
│   └── skill.md                   # Daftar skill yang digunakan dalam proyek
├── docs/
│   ├── rea-radialmenu.md          # Analisis Reverse Engineering RadialMenu Flutter
│   └── rea-logioptions.md         # Analisis Arsitektur Logi Options+ (Daemon/Gesture)
├── memory-bank/
│   ├── architecture.md            # Peta arsitektur dan struktur file saat ini
│   ├── design-document.md         # Spesifikasi arsitektur teknis dan detail komponen
│   ├── tech-stack.md              # Pilihan teknologi dan justifikasi
│   ├── implementation-plan.md     # Rencana langkah implementasi bertahap
│   └── progress.md                # Log eksekusi dan status verifikasi milestone
├── PRD.md                         # Product Requirements Document awal
├── AGENTS.md                      # Panduan dan aturan memori untuk agent AI
├── 14914FroltSoftware.RadialMenu* # Binary referensi 1 (MSIX)
└── logioptionsplus_installer.exe  # Binary referensi 2 (Installer)
```

## 2. Struktur Modul Source Code (Telah Terinisialisasi pada Langkah 1)

Struktur source code proyek saat ini:

```
omniRing/
├── .github/
│   └── workflows/
│       └── build.yml              # CI/CD Windows runner (Rust + Node)
├── src-tauri/
│   ├── Cargo.toml                 # Tauri v2 & Win32 dependencies
│   ├── tauri.conf.json            # Konfigurasi transparent overlay window
│   ├── build.rs                   # Tauri build hook
│   ├── icons/                     # Application bundle icons
│   └── src/
│       ├── main.rs                # Entry point Tauri
│       ├── lib.rs                 # Tauri application builder & setup
│       ├── hook/
│       │   ├── mod.rs                 # Re-ekspor HookWorker, HookEvent, HookDecoder, MouseButton
│       │   └── worker.rs              # Dedicated thread WH_MOUSE_LL dengan message loop Win32 & channel non-blocking
│       ├── gesture/
│       │   ├── mod.rs                 # Re-ekspor GestureDetector, GestureWorker, GestureEvent, Direction, dan helper replay_tap_click
│       │   └── detector.rs            # State machine 4 status (Idle, PendingHold, Flicking, RadialActive), kalkulasi atan2 4 kuadran, deteksi Flick & Tap
│       ├── window/
│       │   ├── mod.rs                 # Re-ekspor ScreenRect, MonitorGeometry, RadialPlacement, helper clamping & window controls
│       │   └── manager.rs         # Win32 WS_EX_NOACTIVATE, multi-monitor geometry (GetMonitorInfoW), GetCursorPos, screen clamping & relative placement
│       ├── profile/
│       │   ├── mod.rs                 # Re-ekspor ProfileConfig, SliceItem, ActionConfig, SubmenuConfig, ProfileManager, ForegroundWatcher, ProcessInfo
│       │   ├── watcher.rs             # Dedicated foreground window watcher (SetWinEventHook EVENT_SYSTEM_FOREGROUND, QueryFullProcessImageNameW)
│       │   └── config.rs              # JSON profile loader, matcher, resolver, fallback cache, dan dukungan submenus/rings
│       ├── action/
│       │   ├── mod.rs                 # Re-ekspor execute_action, execute_shortcut, execute_type_text, execute_open_url, execute_open_app, execute_sequence
│       │   └── dispatcher.rs          # SendInput keyboard injection, KEYEVENTF_UNICODE text, safe URL launcher, app launcher, macro sequence engine, dan VK parser (13 unit test)
│       └── tray/
│           └── mod.rs                 # Windows System Tray & Context Menu controls (Open Config Folder, Reload Profiles, Toggle Enable/Disable, Exit)
├── src/
│   ├── index.html                     # Overlay root container
│   ├── styles/
│   │   └── overlay.css                # Transparent container & slice styling
│   └── scripts/
│       ├── main.ts                    # IPC listener, UI coordinator, submenu navigation, dan action execution dispatcher
│       └── canvas_renderer.ts         # Canvas polar coordinate radial drawing, high-DPI scaling, submenu state tracking, dan dynamic ring switching
├── profiles/
│   ├── default.json                   # Profil default global
│   ├── code.json                      # Contoh profil khusus VS Code dengan submenu Git Tools
│   └── notepad.json                   # Contoh profil khusus Notepad
├── package.json                   # Vite & TypeScript manifest
├── tsconfig.json                  # TypeScript compiler configuration
└── vite.config.ts                 # Vite bundler configuration
```

## 3. Alur Data & Tanggung Jawab Modul

1. **Input Ingestion**: `hook::worker` menangkap event mouse tingkat rendah tanpa blocking.
2. **Gesture Classification**: `gesture::detector` memisahkan tap cepat vs flick 4 arah vs penahanan untuk radial menu.
3. **Context Binding**: `profile::watcher` menyediakan identitas proses foreground untuk memuat konfigurasi profil yang tepat.
4. **Visual Positioning**: `window::manager` menghitung batas monitor dan memposisikan window overlay tanpa mencuri fokus aktif (`WS_EX_NOACTIVATE`).
5. **Rendering & Hit-Testing**: `canvas_renderer.ts` melakukan kalkulasi koordinat polar (`r, theta`), menggambar irisan cincin radial (annular sectors) dengan high-DPI scaling, rendering ikon vector bawaan, garis pemisah, inner hub (dengan label "Back" jika di submenu), serta mendukung transisi cincin child ring (`switchToRing`).
6. **Interaction Coordinator**: `main.ts` menghubungkan event IPC Tauri (`radial-open`, `radial-move`, `radial-release`) dan DOM mouse fallback untuk memperbarui state hover, berpindah ke submenu tanpa menutup jendela, atau memicu `execute_action`.
7. **Action Dispatch**: `action::dispatcher` mengeksekusi aksi yang dipilih:
   - `Shortcut`: Injeksi kombinasi tombol via Win32 `SendInput` dengan penekanan terurut dan pelepasan terbalik serta penanganan extended keys.
   - `TypeText`: Injeksi deretan teks via `KEYEVENTF_UNICODE`.
   - `OpenUrl`: Membuka link web menggunakan browser default via `rundll32 url.dll,FileProtocolHandler` dengan validasi skema aman.
   - `OpenApp`: Menjalankan executable atau perintah sistem via detached process.
   - `Sequence`: Menjalankan urutan rantai aksi berantai dengan jeda `delay_ms`.
   - Jendela overlay disembunyikan secara otomatis sebelum aksi non-submenu dikirim ke Windows agar fokus aplikasi target langsung aktif.
8. **System Tray & User Controls**: `tray::mod` menyediakan kontrol akses melalui area notifikasi Windows:
   - `Open Config Folder`: Membuka direktori profil via Windows File Explorer untuk mempermudah kustomisasi pengguna.
   - `Reload Profiles`: Sinkronisasi ulang konfigurasi profil JSON dari disk ke runtime.
   - `Toggle Enable/Disable`: Mengontrol aktivasi hook mouse tingkat rendah secara instan tanpa perlu mematikan aplikasi.
   - `Exit`: Mematikan aplikasi dan melepaskan semua hook Win32 secara bersih.
