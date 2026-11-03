# Design Document: omniRing

## 1. Arsitektur Sistem

omniRing mengadopsi arsitektur terpisah antara Engine Native Berkecepatan Tinggi (Rust) dan Antarmuka Grafis Dinamis (Tauri v2 / WebView2):

```
+-------------------------------------------------------------+
|                     Windows OS (Win32)                      |
+-------------------------------------------------------------+
      ^                                                ^
      | Low-Level Input Hook                           | SendInput / Window API
      v (WH_MOUSE_LL / WH_KEYBOARD_LL)                 v
+-------------------------------------------------------------+
|                   omniRing Native Engine (Rust)             |
|  - Hook Manager (Dedicated Worker Thread)                   |
|  - Gesture State Machine (Delta threshold, Flick detector)  |
|  - Window Manager (WS_EX_NOACTIVATE, position clamp)        |
|  - Profile Manager (Active foreground process watcher)      |
|  - Macro & Action Dispatcher                                |
+-------------------------------------------------------------+
      | IPC (Tauri Event Bridge / Local Channel)
      v
+-------------------------------------------------------------+
|                 Radial Overlay UI (Webview2)                |
|  - Canvas / SVG Polar Renderer (Inner & Outer Rings)        |
|  - Hover & Active Slice State Machine                       |
|  - Smooth Enter/Exit Micro-animations                       |
|  - Dynamic Icon Cache & Slice Labeling                      |
+-------------------------------------------------------------+
```

## 2. Rincian Komponen Teknis

### A. Input Hook Manager (`hook_manager.rs`)
- Berjalan di dedicated thread dengan Windows Message Loop (`GetMessageW` / `DispatchMessageW`).
- Menggunakan `SetWindowsHookExW` dengan `WH_MOUSE_LL` untuk menangkap tombol mouse pemicu (contoh default: Mouse Button 4/5 atau Middle Click).
- Supresi input: Jika tombol pemicu sedang aktif dalam mode penahanan (hold), hook mengembalikan nilai `1` (`LRESULT(1)`) untuk mencegah klik tembus ke jendela di bawahnya.
- Timeout protection: Seluruh evaluasi hook dilakukan secara non-blocking; data event dikirim melalui bounded crossbeam/mpsc channel ke thread controller.

### B. Gesture Engine & State Machine (`gesture.rs`)
- **Hold vs Tap**:
  - Tombol pemicu ditekan (Down): Catat waktu `T0` dan koordinat kursor `(X0, Y0)`. Mulai timer hold (misal 150ms).
  - Tombol dilepas sebelum timer habis dan jarak pergeseran `< 10px`: Perlakukan sebagai klik standar (klik asli diteruskan).
- **Flick Directional Gesture (Fast Path)**:
  - Sebelum timer radial menu terbuka penuh, jika kursor bergeser lebih dari ambang batas (misal `40px`), kalkulasi sudut pergeseran:
    - `theta = atan2(dY, dX)`
    - Sudut dibagi 4 kuadran: Atas (-90 deg), Kanan (0 deg), Bawah (90 deg), Kiri (180 deg).
    - Batalkan pemunculan menu radial dan langsung picu aksi terarah yang telah terkonfigurasi.
- **Radial Menu Mode (Visual Path)**:
  - Jika tombol ditahan melebihi timer dan kursor tetap berada dalam radius kecil (`< 15px`), sinyal pembukaan menu radial dikirim ke Window Manager.

### C. Window Manager & Overlay Strategy (`window.rs`)
- **Hidden Persistent Window**:
  - Satu window overlay dibuat saat aplikasi pertama kali dijalankan dengan style transparan, borderless, dan disembunyikan (`ShowWindow(SW_HIDE)`).
  - Eliminasi cold-start WebView2 yang biasanya memakan waktu 150-300ms.
- **Window Flags (Zero Focus Stealing)**:
  - `WS_EX_NOACTIVATE`: Mencegah window mengambil fokus keyboard dari aplikasi yang sedang aktif.
  - `WS_EX_TOPMOST`: Menjaga menu selalu berada di lapisan visual paling atas.
  - `WS_EX_TOOLWINDOW`: Mencegah kemunculan jendela di Taskbar dan dialog Alt+Tab.
- **Screen Geometry & Bounding Clamping**:
  - Mengambil batas monitor tempat kursor berada menggunakan `MonitorFromPoint` + `GetMonitorInfoW`.
  - Jika radius menu menyebabkan menu keluar dari tepi layar monitor, koordinat pusat menu digeser (clamped) agar seluruh irisan tetap berada di dalam viewport monitor.

### D. Profile & Foreground Watcher (`profile_manager.rs`)
- Menggunakan WinEvent hook `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, ...)` untuk menerima notifikasi setiap kali jendela aktif pengguna berganti.
- Mengidentifikasi proses foreground:
  1. `GetForegroundWindow()`
  2. `GetWindowThreadProcessId(hwnd, &pid)`
  3. `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, ...)`
  4. `QueryFullProcessImageNameW(...)` -> mengekstrak nama file executable (misal: `code.exe`, `blender.exe`).
- Mencari file konfigurasi profil di direktori profil:
  - Jika ditemukan `profiles/<process_name>.json`, gunakan profil tersebut.
  - Jika tidak ditemukan, fallback ke `profiles/default.json`.

### E. Macro & Action Dispatcher (`action_dispatcher.rs`)
- **Tipe Aksi yang Didukung**:
  1. `Shortcut`: Mengirim kombinasi tombol via `SendInput` ke aplikasi aktif.
  2. `OpenApp`: Menjalankan proses executable baru.
  3. `OpenUrl`: Membuka link browser via shell command.
  4. `TypeText`: Menginjeksi deretan teks Unicode via `SendInput` dengan flag `KEYEVENTF_UNICODE`.
  5. `Submenu`: Mengganti set irisan aktif di UI ke cincin level berikutnya tanpa menutup window.
  6. `Sequence`: Menjalankan rantai aksi dengan jeda `delay_ms` di antara masing-masing aksi.

## 3. Struktur Data Konfigurasi (JSON Schema)

```json
{
  "profile_name": "Visual Studio Code",
  "process_targets": ["code.exe"],
  "inner_radius": 50,
  "outer_radius": 140,
  "center_action": {
    "type": "Submenu",
    "target_ring_id": "root"
  },
  "slices": [
    {
      "id": "slice_1",
      "label": "Format Code",
      "icon": "code-outline",
      "action": {
        "type": "Shortcut",
        "keys": ["Shift", "Alt", "F"]
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
      "label": "Git Tools",
      "icon": "git-branch",
      "action": {
        "type": "Submenu",
        "target_ring_id": "git_submenu"
      }
    }
  ]
}
```

## 4. Acceptance Criteria & Edge Cases
1. **Supresi Input Bersih**: Menahan tombol pemicu tidak memicu aksi default tombol mouse pada aplikasi di latar belakang.
2. **Penargetan Fokus Presisi**: Aksi shortcut yang diinjeksikan harus mendarat tepat pada jendela aplikasi yang tadinya aktif sebelum menu dipanggil.
3. **Multi-Monitor Awareness**: Pemanggilan menu pada monitor kedua atau ketiga harus terpusat di layar yang benar dengan koordinat DPI-aware.
4. **Resilience Terhadap Hook Timeout**: Operasi I/O berat tidak boleh dijalankan di dalam callback hook; semua event harus diproses secara asynchronous.
