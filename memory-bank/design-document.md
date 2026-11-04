# Design Document: omniRing

## 1. Arsitektur Sistem

omniRing mengadopsi arsitektur terpisah antara Engine Native Berkecepatan Tinggi (Rust) dan Antarmuka Grafis Dinamis (Tauri v2 / WebView2 multi-window):

```
+---------------------------------------------------------------------------------+
|                               Windows OS (Win32)                                |
+---------------------------------------------------------------------------------+
      ^                                                            ^
      | Low-Level Input Hooks                                      | SendInput / Window API
      v (WH_MOUSE_LL / WH_KEYBOARD_LL / GetAsyncKeyState)          v
+---------------------------------------------------------------------------------+
|                          omniRing Native Engine (Rust)                          |
|  - Hook Manager (Dedicated Worker: Hold, Combo, Outside-Click, Esc suppress)     |
|  - Gesture State Machine (Delta threshold, Flick 4 kuadran, Tap, CornerHover)   |
|  - Window Manager (WS_EX_NOACTIVATE, multi-monitor clamp, multi-window lifecycle)|
|  - Profile Manager (Active foreground process watcher, JSON disk persistence)   |
|  - Macro & Action Dispatcher (Keyboard injection, Unicode, Shell, Sequence)     |
+---------------------------------------------------------------------------------+
      |                                              |
      | IPC (Tauri Event Bridge / Local Channel)     | IPC Commands & Event Sync
      v                                              v
+----------------------------------------+ +--------------------------------------+
|       Radial Overlay UI (Webview2)     | |    Desktop Configurator (Webview2)   |
|  - Bubble Nodes Polar Renderer         | |  - Top Profile Switcher Strip        |
|  - Center Dismiss Hub ('×', < 26px)    | |  - Live Interactive 8-Slot Wheel     |
|  - 8 Circular Action Nodes (r = 24px)  | |  - Searchable Action Library Sidebar |
|  - Floating Outer Pill Badges          | |  - Trigger Mode Settings Tab         |
|  - Fitts's Law 45-degree Hit-Testing   | |  - Undo/Redo & Real-Time Sync        |
+----------------------------------------+ +--------------------------------------+
```

## 2. Rincian Komponen Teknis

### A. Input Hook Manager (`hook::worker`)
- Berjalan di dedicated thread dengan Windows Message Loop (`GetMessageW` / `DispatchMessageW`).
- Menangkap event mouse tingkat rendah via `SetWindowsHookExW` (`WH_MOUSE_LL`) dan keyboard (`WH_KEYBOARD_LL`).
- **Sistem Multi-Mode Trigger**:
  - `TriggerMode::Hold { button, hold_ms }`: Menahan tombol mouse pemicu (default Button 4 / X1) hingga melebihi batas waktu (default 150ms).
  - `TriggerMode::Combo { modifiers, button }`: Memeriksa tombol modifier keyboard (Ctrl, Shift, Alt, Win) secara instan via Win32 `GetAsyncKeyState` saat tombol mouse ditekan. Jika kombinasi cocok, menu radial langsung aktif tanpa jeda hold timer. Jika modifier tidak ditekan, klik mouse diteruskan langsung ke sistem (`CallNextHookEx`) tanpa supresi.
  - `TriggerMode::CornerHover { corner, dwell_ms }`: Memantau pergerakan kursor yang berdiam (dwell) di sudut layar yang ditentukan.
- **Outside-Click Auto-Close**:
  - Saat status radial aktif (`RadialActive`), jika mouse down terdeteksi di luar batas menu (`dist > outer_radius + tolerance`), overlay otomatis ditutup seketika (`HookEvent::OutsideClick`), dan klik mouse tetap diteruskan ke aplikasi di latar belakang (`CallNextHookEx`).
- **Supresi Pembatalan Tombol Escape**:
  - Hook `WH_KEYBOARD_LL` memantau `VK_ESCAPE` (0x1B). Jika menu radial sedang aktif, event pembatalan dikirimkan untuk menutup overlay, dan tombol Esc disupresi (`LRESULT(1)`) sehingga tidak membatalkan kerja pada aplikasi aktif.
- **Supresi Input Bersih & Timeout Protection**:
  - Seluruh evaluasi hook bersifat non-blocking dan diproteksi dari event sintetis (`LLMHF_INJECTED`). Event dikirim melalui bounded crossbeam channel ke thread controller.

### B. Gesture Engine & State Machine (`gesture::detector`)
- **State Machine 4 Status**:
  - `Idle`: Status siaga menunggu penekanan tombol pemicu.
  - `PendingHold`: Tombol pemicu ditekan, menunggu evaluasi apakah pengguna melakukan tap cepat, flick, atau hold.
  - `Flicking`: Kursor bergeser melebihi ambang batas (> 40px) sebelum hold timer habis.
  - `RadialActive`: Menu radial visual aktif ditampilkan di layar.
- **Fast-Path Directional Flick**:
  - Jika kursor digeser cepat sebelum timer hold habis (> 40px dalam < 150ms), kalkulasi arah gestur:
    - `theta = atan2(dY, dX)`
    - Dibagi 4 kuadran: Atas (-90 deg), Kanan (0 deg), Bawah (90 deg), Kiri (180 deg).
    - Membatalkan kemunculan menu visual dan langsung memicu aksi terarah yang terkonfigurasi.
- **Fast-Path Tap Replay**:
  - Jika tombol dilepaskan sebelum timer habis (< 150ms) dan jarak pergeseran kursor <= 10px, event direplay sebagai klik normal melalui sintetis `mouse_event` dengan flag `LLMHF_INJECTED`.
- **Mode Combo & Corner Hover**:
  - Mode Combo langsung mentransisikan state dari `Idle` ke `RadialActive` seketika.
  - Mode Corner Hover menghitung akumulasi waktu diam kursor di zona sudut layar (dwell timer).

### C. Window Manager & Overlay Strategy (`window::manager`)
- **Strategi Multi-Window**:
  1. **Radial Overlay Window (`index.html`)**:
     - Window transparan, borderless, persistent, dan hidden (`SW_HIDE`) saat startup untuk mengeliminasi cold-start WebView2.
     - Window Flags:
       - `WS_EX_NOACTIVATE`: Mencegah window mengambil fokus keyboard dari aplikasi yang sedang aktif.
       - `WS_EX_TOPMOST`: Menjaga overlay selalu berada di lapisan visual teratas.
       - `WS_EX_TOOLWINDOW`: Mencegah kemunculan jendela di Taskbar dan dialog Alt+Tab.
     - Multi-Monitor Clamping: Menghitung batas monitor via `MonitorFromPoint` + `GetMonitorInfoW` (`rcWork`) agar seluruh irisan radial dan floating pill label tetap berada di dalam viewport layar.
  2. **Desktop Configurator Window (`config.html`)**:
     - Window desktop standar (980x680) dengan dekorasi window chrome bawaan Windows.
     - Dikelola oleh Tauri v2 dan dipanggil melalui opsi tray "Configure OmniRing" atau event double-click pada tray icon.

### D. Frontend Bubble Nodes UI Architecture (`canvas_renderer.ts`)
- Antarmuka visual mengadopsi bahasa desain Logitech Options+ Bubble Nodes (terinspirasi `docs/bubbles.webp`):
  1. **Inner Dismiss Hub**:
     - Lingkaran tengah pembatalan berukuran kecil (`r = 22px`) dengan simbol silang '×' (`stroke = #475569`, `lineWidth = 2px`).
     - Dead-zone pembatalan: pelepasan kursor pada jarak `< 26px` membatalkan menu tanpa memicu aksi apapun (`isCenterHub`).
  2. **8 Circular Action Bubble Nodes**:
     - Lingkaran aksi gelap (`r = 24px`, warna `#18181B`) ditempatkan pada radius orbital 95px.
     - Dilengkapi ikon vektor putih presisi (stroke 2px) yang digambar secara native via canvas 2D path tanpa dependensi font eksternal.
  3. **Outer Floating Pill Badges**:
     - Label mengambang berbentuk badge rounded rectangle putih (`#FFFFFF`, border-radius 6px, bayangan halus `0 4px 12px rgba(0,0,0,0.12)`) pada radius orbital 175px.
     - Diposisikan sejajar sepanjang vektor sudut radial masing-masing irisan.
  4. **Active Hover Micro-Interactions**:
     - Node aktif membesar 1.12x (`r = 26.88px`) disertai pendaran aksen glow (`#5E48E8`, blur 16px).
     - Pill badge aktif beralih ke tema kontras tinggi berlatar gelap (`#18181B`) dengan teks putih dan garis tepi aksen ungu.
  5. **Hit-Testing Polar Fitts's Law**:
     - Pembagian 8 sektor sudut 45 derajat yang merata, dimulai dari posisi jam 12 (-90 derajat).
     - Menghitung jarak Euclidean `r = hypot(dx, dy)` dan sudut `theta = atan2(dy, dx)` terhadap titik pusat relative menu.
  6. **Dukungan Cincin Bertingkat (Submenus)**:
     - Mendukung navigasi cincin hierarkis (`switchToRing`, `resetToRoot`).
     - Hub tengah menampilkan label 'Back' saat berada di level submenu.

### E. Desktop Configurator & Action Library (`config.html`, `config.ts`, `config.css`)
- Antarmuka desktop konfigurasi interaktif terinspirasi Loupedeck & Logitech Options+ (`docs/menu.png`):
  1. **Top Profile Bar**:
     - Tab profil aplikasi (`Default Global`, `VS Code`, `Notepad`, `+ Add Profile`).
     - View switcher antara "Ring Editor" dan "Trigger Settings".
     - Tombol cepat "Open Folder" dan "Reload Profiles".
  2. **Interactive Workspace (70%)**:
     - Live preview radial wheel 8-slot yang mencerminkan render Bubble Nodes overlay.
     - Ring seleksi slot aktif berpendar ungu (`#5E48E8`) yang merespons klik pengguna.
     - Dock kontrol bawah: Undo, Redo, Clear Slot, dan Reset Slices.
  3. **Action Library Sidebar (30%)**:
     - Kolom pencarian instan dengan pemfilteran kartu aksi secara real-time.
     - 4 Kategori Accordion terorganisir:
       - `System Actions` (Lock Screen, Volume Mute, Play/Pause, Screenshot, dsb).
       - `Device Navigation & Folders` (Open Explorer, Switch Window, Back, Desktop, dsb).
       - `Keyboard Shortcuts` (Copy, Paste, Undo, Redo, Custom Shortcut).
       - `App Launchers & URLs` (Terminal, Browser, Custom App, Web Link).
     - Alokasi aksi instan: klik slot pada wheel lalu klik kartu aksi untuk menugaskan aksi.
  4. **Panel Pengaturan Trigger**:
     - Pilihan interaktif mode pemicu: `Hold` (durasi dan tombol), `Combo` (kombinasi modifier dan tombol mouse), serta `CornerHover`.
     - Visualisasi pemetaan gestur flick 4 arah.
  5. **Sinkronisasi Dua Arah Real-Time**:
     - Setiap perubahan langsung disimpan ke berkas JSON melalui perintah Tauri `save_profile`.
     - Memancarkan event `omniring://profile-changed` sehingga overlay langsung memperbarui tata letak aksi tanpa perlu me-restart aplikasi.

### F. Profile & Foreground Watcher (`profile::watcher`, `profile::config`)
- Menggunakan WinEvent hook `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, ...)` untuk menerima notifikasi pergantian jendela aktif.
- Ekstraksi nama proses target via `GetForegroundWindow` + `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`.
- Pemindaian folder `profiles/` dengan pencocokan nama file proses (misal `code.exe`, `notepad.exe`) dan fallback aman ke `default.json`.
- Serialisasi dan penulisan konfigurasi profil ke disk JSON via method `save_profile`.

### G. Macro & Action Dispatcher (`action::dispatcher`)
- **Instant Auto-Close**: Jendela overlay disembunyikan (`hide_overlay`) seketika sebelum aksi terminal dikirimkan ke Windows, memastikan jendela target menerima fokus secara penuh.
- **Tipe Aksi yang Didukung**:
  1. `Shortcut`: Injeksi kombinasi tombol via Win32 `SendInput` dengan penekanan terurut, pelepasan terbalik, dan penanganan extended keys.
  2. `TypeText`: Injeksi deretan teks karakter Unicode via `KEYEVENTF_UNICODE`.
  3. `OpenUrl`: Membuka link web menggunakan browser default sistem via `rundll32.exe url.dll,FileProtocolHandler`.
  4. `OpenApp`: Menjalankan executable atau perintah sistem via detached process.
  5. `Submenu`: Mengganti set irisan aktif di UI ke cincin level berikutnya tanpa menutup jendela.
  6. `Sequence`: Menjalankan urutan rantai aksi berantai dengan jeda milidetik terukur.

## 3. Struktur Data Konfigurasi (JSON Schema)

```json
{
  "profile_name": "Visual Studio Code",
  "process_targets": ["code.exe"],
  "inner_radius": 22,
  "outer_radius": 210,
  "slices": [
    {
      "id": "slice_0",
      "label": "Format Code",
      "icon": "code",
      "action": {
        "type": "Shortcut",
        "keys": ["Shift", "Alt", "F"]
      }
    },
    {
      "id": "slice_1",
      "label": "Git Tools",
      "icon": "folder",
      "action": {
        "type": "Submenu",
        "target_ring_id": "git_tools"
      }
    },
    {
      "id": "slice_2",
      "label": "Toggle Terminal",
      "icon": "terminal",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "`"]
      }
    },
    {
      "id": "slice_3",
      "label": "Quick Search",
      "icon": "search",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "Shift", "F"]
      }
    },
    {
      "id": "slice_4",
      "label": "New File",
      "icon": "new-note",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "N"]
      }
    },
    {
      "id": "slice_5",
      "label": "Command Palette",
      "icon": "options",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "Shift", "P"]
      }
    },
    {
      "id": "slice_6",
      "label": "Copy",
      "icon": "copy",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "C"]
      }
    },
    {
      "id": "slice_7",
      "label": "Paste",
      "icon": "paste",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "V"]
      }
    }
  ],
  "submenus": {
    "git_tools": {
      "title": "Git Tools",
      "slices": [
        {
          "id": "git_status",
          "label": "Status Cmd",
          "icon": "terminal",
          "action": {
            "type": "TypeText",
            "text": "git status\n"
          }
        },
        {
          "id": "git_back",
          "label": "Back",
          "icon": "undo",
          "action": {
            "type": "Submenu",
            "target_ring_id": "root"
          }
        }
      ]
    }
  }
}
```

## 4. Acceptance Criteria & Edge Cases

1. **Supresi Input Bersih & Focus Stealing Prevention**:
   - Menahan tombol pemicu tidak memicu aksi default tombol mouse pada aplikasi di latar belakang.
   - Window overlay menerapkan `WS_EX_NOACTIVATE | WS_EX_TOPMOST` sehingga kursor teks pada editor tidak kehilangan fokus saat menu muncul.
2. **Supresi Tombol Escape Bersih**:
   - Menekan tombol `VK_ESCAPE` saat menu radial aktif menutup overlay secara instan dan mengembalikan `LRESULT(1)` sehingga tombol Esc tidak membatalkan kerja pada aplikasi yang berada di bawahnya.
3. **Outside-Click Auto-Close Passthrough**:
   - Klik mouse di luar batas luar menu radial (`outer_radius + tolerance`) otomatis menutup overlay seketika dan klik diteruskan langsung ke sistem (`CallNextHookEx`).
4. **Akurasi Mode Combo**:
   - Pada mode Combo (misal Ctrl + Right Click), klik kanan biasa tanpa tombol Ctrl tetap diteruskan secara normal ke sistem untuk memunculkan context menu Windows tanpa hambatan.
5. **High-DPI Alignment & Scalability**:
   - Seluruh elemen grafis Canvas 2D (hub tengah, node bubble 24px, pill badges, teks, dan ikon vektor) diskalakan menggunakan `window.devicePixelRatio` agar tampil tajam tanpa distorsi atau blur pada penskalaan 100%, 125%, 150%, dan 200%.
6. **Multi-Monitor Boundary Clamping**:
   - Pemanggilan menu pada monitor sekunder di posisi koordinat negatif atau tepi layar otomatis digeser (*clamped*) agar seluruh 8 bubble node dan floating pill badges tetap berada di dalam area kerja monitor aktif.
