# Architecture: omniRing

## 1. Directory & File Structure

```
omniRing/
├── SKILL/
│   └── skill.md                   # Daftar skill yang digunakan dalam proyek
├── docs/
│   ├── rea-radialmenu.md          # Analisis Reverse Engineering RadialMenu Flutter
│   ├── rea-logioptions.md         # Analisis Arsitektur Logi Options+ (Daemon/Gesture)
│   ├── bubbles.webp               # Referensi visual Logitech Options+ Bubble Nodes UI
│   ├── menu.png                   # Referensi visual Desktop Configurator & Action Library
│   └── superpowers/plans/         # Dokumen rencana implementasi berkala
├── memory-bank/
│   ├── architecture.md            # Peta arsitektur dan struktur file saat ini
│   ├── design-document.md         # Spesifikasi arsitektur teknis dan detail komponen
│   ├── tech-stack.md              # Pilihan teknologi dan justifikasi
│   ├── implementation-plan.md     # Rencana langkah implementasi bertahap
│   └── progress.md                # Log eksekusi dan status verifikasi milestone
├── PRD.md                         # Product Requirements Document awal
├── README.md                      # Dokumentasi penggunaan dan panduan pengguna
├── AGENTS.md                      # Panduan dan aturan memori untuk agent AI
├── config.html                    # Entry point desktop configurator window
├── index.html                     # Entry point radial overlay window
├── package.json                   # Vite & TypeScript manifest
├── tsconfig.json                  # TypeScript compiler configuration
├── vite.config.ts                 # Multi-page build configuration (main & config)
├── 14914FroltSoftware.RadialMenu* # Binary referensi 1 (MSIX)
└── logioptionsplus_installer.exe  # Binary referensi 2 (Installer)
```

## 2. Struktur Modul Source Code

Struktur source code proyek saat ini:

```
omniRing/
├── .github/
│   └── workflows/
│       └── build.yml              # CI/CD Windows runner (Rust + Node, auto upload log artifact)
├── src-tauri/
│   ├── Cargo.toml                 # Tauri v2 & Win32 dependencies (windows crate 0.62)
│   ├── tauri.conf.json            # Konfigurasi multi-window (overlay transparent & desktop configurator)
│   ├── build.rs                   # Tauri build hook
│   ├── icons/                     # Application bundle icons
│   └── src/
│       ├── main.rs                # Entry point Tauri
│       ├── lib.rs                 # Tauri application builder, state management, & IPC command registry
│       ├── hook/
│       │   ├── mod.rs             # Re-ekspor HookWorker, HookEvent, HookDecoder, MouseButton, TriggerMode
│       │   └── worker.rs          # Dedicated thread WH_MOUSE_LL & WH_KEYBOARD_LL, GetAsyncKeyState combo, outside-click auto-close, Esc suppression
│       ├── gesture/
│       │   ├── mod.rs             # Re-ekspor GestureDetector, GestureWorker, GestureEvent, Direction, helper replay_tap_click
│       │   └── detector.rs        # State machine (Idle, PendingHold, Flicking, RadialActive), kalkulasi atan2 4 kuadran, deteksi Flick, Tap, CornerHover
│       ├── window/
│       │   ├── mod.rs             # Re-ekspor ScreenRect, MonitorGeometry, RadialPlacement, helper clamping & window controls
│       │   └── manager.rs         # Win32 WS_EX_NOACTIVATE, multi-monitor geometry (GetMonitorInfoW), GetCursorPos, screen clamping & relative placement
│       ├── profile/
│       │   ├── mod.rs             # Re-ekspor ProfileConfig, SliceItem, ActionConfig, SubmenuConfig, ProfileManager, ForegroundWatcher, ProcessInfo
│       │   ├── watcher.rs         # Dedicated foreground window watcher (SetWinEventHook EVENT_SYSTEM_FOREGROUND, QueryFullProcessImageNameW)
│       │   └── config.rs          # JSON profile loader, matcher, resolver, fallback cache, submenus/rings, dan save_profile disk writer
│       ├── action/
│       │   ├── mod.rs             # Re-ekspor execute_action, execute_shortcut, execute_type_text, execute_open_url, execute_open_app, execute_sequence
│       │   └── dispatcher.rs      # SendInput keyboard injection, KEYEVENTF_UNICODE text, safe URL launcher, app launcher, macro sequence engine, dan VK parser
│       └── tray/
│           └── mod.rs             # Windows System Tray & Context Menu controls (Configure OmniRing, Open Config Folder, Reload Profiles, Toggle Enable/Disable, Exit)
├── src/
│   ├── index.html                 # Radial overlay root container (bubbles.webp UI)
│   ├── config.html                # Configurator desktop root container (menu.png UI)
│   ├── styles/
│   │   ├── overlay.css            # Transparent container, hardware-accelerated transform, & pointer events
│   │   └── config.css             # Desktop configurator window styling (light theme, accordion, selection glow, action cards)
│   └── scripts/
│       ├── main.ts                # Overlay IPC coordinator, center hub dismiss, outside-click fallback, & instant auto-close
│       ├── config.ts              # Desktop configurator coordinator, profile switcher, interactive wheel editor, action library, & trigger settings
│       └── canvas_renderer.ts     # Canvas polar coordinate Bubble Nodes renderer (hub tengah, 8 orbital bubble nodes, floating pill badges, high-DPI scaling)
├── profiles/
│   ├── default.json               # Profil default global (8 action bubble nodes)
│   ├── code.json                  # Profil khusus VS Code dengan submenu Git Tools
│   └── notepad.json               # Profil khusus Notepad
├── package.json                   # Vite & TypeScript manifest
├── tsconfig.json                  # TypeScript compiler configuration
└── vite.config.ts                 # Multi-page build configuration (main & config)
```

## 3. Alur Data & Tanggung Jawab Modul

1. **Input Ingestion & Hook Worker**:
   - `hook::worker` menangkap event mouse (`WH_MOUSE_LL`) dan keyboard (`WH_KEYBOARD_LL`) tingkat rendah pada thread terisolasi dengan message loop Win32.
   - Mendukung multi-mode trigger:
     - `Hold`: Menahan tombol mouse (default Button 4 / X1) melebihi batas waktu (default 150ms).
     - `Combo`: Deteksi instan penekanan tombol modifier keyboard (Ctrl, Shift, Alt, Win) via Win32 `GetAsyncKeyState` bersamaan dengan klik mouse tanpa menunggu hold timer. Klik biasa tanpa modifier diteruskan langsung ke sistem tanpa supresi.
     - `CornerHover`: Deteksi pergerakan kursor yang berdiam (dwell) di sudut layar yang ditentukan.
   - Penutupan otomatis saat klik di luar menu radial (Outside-Click Auto-Close): mendeteksi klik mouse di luar batas `outer_radius + tolerance`, menutup overlay secara instan, dan meneruskan klik ke jendela aplikasi di bawahnya (`CallNextHookEx`).
   - Supresi pembatalan tombol `VK_ESCAPE`: menangkap penekanan tombol Escape saat overlay aktif, menutup menu, dan mengembalikan `LRESULT(1)` sehingga tombol Esc tidak membatalkan kerja pada aplikasi aktif di latar belakang.

2. **Gesture Classification & State Machine**:
   - `gesture::detector` memproses state machine (`Idle`, `PendingHold`, `Flicking`, `RadialActive`) untuk memisahkan tap cepat vs flick 4 arah vs penahanan radial hold vs pembukaan instan mode combo vs dwell di sudut layar (`CornerHover`).
   - Flick cepat (> 40px sebelum batas hold timer) memicu aksi terarah instan tanpa memunculkan UI overlay.
   - Tap cepat (< 150ms dan pergeseran <= 10px) mereplay klik asli via sintetis `mouse_event` dengan proteksi `LLMHF_INJECTED`.

3. **Context Binding & Foreground Watcher**:
   - `profile::watcher` memantau pergantian jendela aktif via `SetWinEventHook` (`EVENT_SYSTEM_FOREGROUND`) dan mengekstrak nama proses via `QueryFullProcessImageNameW`.
   - Mengabaikan proses omniRing itu sendiri (`own_pid`) dan mendeduplikasi event berulang.
   - Memuat profil yang sesuai dari folder `profiles/` dengan fallback aman ke `default.json`.

4. **Visual Positioning & Boundary Clamping**:
   - `window::manager` menghitung batas monitor tempat kursor berada menggunakan `MonitorFromPoint` dan `GetMonitorInfoW`.
   - Menghitung relative clamped placement agar seluruh irisan dan floating pill label tetap berada di dalam viewport layar.
   - Menerapkan flag Win32 `WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW` sehingga overlay tidak pernah mencuri fokus keyboard dari aplikasi aktif.

5. **Rendering & Hit-Testing (Bubble Nodes UI)**:
   - `canvas_renderer.ts` melakukan kalkulasi koordinat polar (`r, theta`) dan merender antarmuka Bubble Nodes melingkar Logitech Options+ (terinspirasi `docs/bubbles.webp`):
     - Hub pembatalan tengah: lingkaran kecil (`r = 22px`) dengan simbol silang '×', bergaris tepi halus dan dead-zone pembatalan `< 26px`.
     - 8 Action Bubble Nodes: lingkaran gelap (`r = 24px`, `#18181B`) pada radius orbital 95px dengan ikon vektor putih beresolusi tinggi (stroke 2px).
     - Floating Pill Badges: badge persegi panjang melengkung putih (`#FFFFFF`, border-radius 6px, bayangan halus `rgba(0,0,0,0.12)`) pada radius orbital 175px sepanjang vektor sudut irisan.
     - Efek hover aktif: node membesar 1.12x dengan pendaran aksen glow (`#5E48E8`, blur 16px), dan pill badge beralih ke kontras tinggi latar gelap dengan teks putih.
     - Algoritma hit-testing polar Fitts's Law dengan 8 sektor sudut 45 derajat (dimulai dari posisi jam 12 / -90 derajat).
     - Navigasi submenu cincin bertingkat (`switchToRing`, `resetToRoot`, dan rendering label 'Back' pada inner hub).

6. **Overlay Interaction Coordinator**:
   - `main.ts` menghubungkan event IPC Tauri (`radial-open`, `radial-move`, `radial-release`, `cancelled`) dan DOM pointer fallback.
   - Mengarahkan pelepasan kursor pada center dismiss hub (`r < 26px`) untuk membatalkan menu tanpa aksi.
   - Instant auto-close: menyembunyikan overlay (`hide_overlay`) seketika sebelum aksi terminal dieksekusi agar jendela aplikasi target langsung menerima input secara utuh.
   - Penanganan fallback keyboard Escape dan outside pointer click.

7. **Action Dispatcher & Macro Execution**:
   - `action::dispatcher` mengeksekusi aksi yang dipilih:
     - `Shortcut`: Injeksi kombinasi tombol via Win32 `SendInput` dengan urutan penekanan terencana dan pelepasan terbalik serta penanganan extended keys.
     - `TypeText`: Injeksi deretan teks karakter Unicode via `KEYEVENTF_UNICODE`.
     - `OpenUrl`: Membuka link web menggunakan browser default via `rundll32.exe url.dll,FileProtocolHandler` dengan validasi skema aman.
     - `OpenApp`: Menjalankan executable atau perintah sistem via detached process.
     - `Sequence`: Menjalankan urutan rantai aksi berantai dengan jeda milidetik terukur.

8. **System Tray & User Controls**:
   - `tray::mod` menyediakan kontrol akses melalui area notifikasi Windows:
     - `Configure OmniRing`: Menampilkan, unminimize, dan memfokuskan jendela configurator desktop (`open_configurator_window`).
     - `Open Config Folder`: Membuka direktori profil via Windows File Explorer untuk mempermudah kustomisasi.
     - `Reload Profiles`: Sinkronisasi ulang konfigurasi profil JSON dari disk ke runtime.
     - `Toggle Enable/Disable`: Mengontrol aktivasi hook mouse tingkat rendah secara instan tanpa perlu mematikan aplikasi.
     - `Exit`: Mematikan aplikasi dan melepaskan seluruh hook Win32 secara bersih.
     - Double-click pada tray icon otomatis membuka jendela configurator.

9. **Desktop Configurator Window**:
   - `config.html`, `config.ts`, dan `config.css` mengimplementasikan window konfigurasi desktop modern (terinspirasi Loupedeck & Logitech Options+, `docs/menu.png`):
     - Header bar: strip tab profil aplikasi (`Default Global`, `VS Code`, `Notepad`, `+ Add Profile`), view switcher (Ring Editor vs Trigger Settings), serta aksi `Open Folder` dan `Reload Profiles`.
     - Workspace kiri (70%): Live interactive preview radial wheel 8-slot dengan bubble nodes, outer floating pill labels, ring seleksi slot aktif berpendar (`#5E48E8`), dan dock kontrol (Undo, Redo, Clear Slot, Reset Slices).
     - Sidebar kanan (30%): Action Library dengan pencarian instan dan 4 accordion kategori (System Actions, Device Navigation & Folders, Keyboard Shortcuts, App Launchers & URLs) untuk penugasan aksi ke slot terpilih secara real-time.
     - Panel Pengaturan Trigger: konfigurasi interaktif mode trigger (`Hold`, `Combo`, `CornerHover`) dan visualisasi pemetaan flick gesture 4 arah.
     - Modal dialog untuk penambahan profil aplikasi baru dan pembuatan aksi kustom.
     - Sinkronisasi dua arah real-time via perintah Tauri `save_profile` dan event listener `omniring://profile-changed` yang langsung menghubungkan configurator ke live overlay runtime.
