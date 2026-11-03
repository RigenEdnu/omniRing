# Product Requirements Document (PRD): omniRing

## 1. Product Overview
- **Nama Produk**: omniRing
- **Kategori**: Windows Productivity Utility (Radial Menu & Gesture Controller)
- **Target Platform**: Windows 10 / Windows 11 (64-bit)
- **Target User**: Power users, developers, digital artists/designers, dan multitaskers yang membutuhkan eksekusi shortcut dan makro secara instan di posisi kursor mouse tanpa memindahkan tangan dari mouse atau menghafal puluhan hotkey keyboard.

## 2. Problem Statement
- **Keterbatasan Solusi Eksisting**:
  - Aplikasi radial menu berbasis Flutter (seperti RadialMenu referensi) memiliki ukuran distribusi besar (>70MB), dependensi eksternal berlebih, dan penggunaan RAM idle yang tinggi (80-150MB).
  - Utilitas gesture seperti Logi Options+ terkunci hanya untuk perangkat keras proprietary Logitech, memiliki overhead proses tinggi, dan mewajibkan telemetri/akun online.
- **Kebutuhan Pengguna**:
  - Utilitas ringan (<50MB RAM), beroperasi 100% lokal dan offline, kompatibel dengan mouse/keyboard standar apapun.
  - Akses instan di posisi kursor tanpa jeda (zero cold-start latency).
  - Tidak mengganggu fokus jendela aktif (*non-activating window*) sehingga injeksi shortcut tidak memerlukan re-fokus.

## 3. Primary User Flows

### Flow 1: Pemanggilan Radial Menu (Instant Summon)
1. Pengguna menahan tombol pemicu (misal Mouse Button 4/5, Middle Click hold, atau hotkey keyboard).
2. omniRing mendeteksi posisi kursor saat ini `(X, Y)`.
3. Jendela radial menu yang sudah standby (hidden persistent window) diposisikan di tengah kursor dan ditampilkan seketika (<25ms).
4. Pengguna mengarahkan kursor ke irisan (slice) yang diinginkan (indikator visual hover aktif).
5. Pengguna melepas tombol pemicu atau mengklik irisan.
6. Aksi dieksekusi langsung pada jendela aplikasi di bawahnya, dan radial menu kembali sembunyi.

### Flow 2: Quick Directional Gesture (Flick Action)
1. Pengguna menahan tombol pemicu mouse dan langsung menggeser mouse melebihi ambang batas jarak (threshold misal 40px) ke arah tertentu (Atas, Bawah, Kiri, Kanan).
2. Gesture engine langsung mendeteksi vektor arah sebelum menu radial sempat terbuka penuh.
3. Aksi terarah langsung dieksekusi (misal: Atas = Volume Up, Bawah = Volume Down, Kiri = Switch Tab Kiri, Kanan = Switch Tab Kanan).
4. Menu radial dibatalkan tanpa kedipan visual.

### Flow 3: Auto-Switch Profil Aplikasi Aktif (Context Awareness)
1. Pengguna bekerja di Visual Studio Code (`code.exe`).
2. omniRing mendeteksi bahwa foreground window adalah `code.exe`.
3. Saat trigger ditekan, irisan menu memuat aksi khusus VS Code (Format Document, Run Test, Toggle Terminal, Split Editor).
4. Saat pengguna berpindah ke browser (`chrome.exe`), susunan irisan otomatis menyesuaikan dengan aksi web (New Tab, Reload, Bookmark, Close Tab).

### Flow 4: Kustomisasi & Submenu
1. Pengguna membuka editor konfigurasi melalui icon tray atau klik tengah menu radial.
2. Pengguna dapat menambah, mengedit, dan menghapus irisan.
3. Mendukung pembuatan submenu (irisan yang membuka cincin level 2 atau sub-ring).
4. Pengguna dapat memilih aksi: Jalankan Program, Buka URL, Shortcut Keyboard, Injeksi Teks, atau Rantai Makro.

## 4. Core Features & Capabilities

1. **High-Performance Rust Input Engine**:
   - Menjalankan low-level hook (`WH_MOUSE_LL` / `WH_KEYBOARD_LL`) di thread terisolasi.
   - Mampu melakukan supresi klik pemicu agar klik asli tidak tembus ke aplikasi di bawahnya saat trigger aktif.
2. **Zero-Focus Stealing Overlay (Non-Activating Window)**:
   - Window overlay dikonfigurasi dengan flag Win32: `WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW`.
   - Menghindari pergeseran fokus Windows (`WM_ACTIVATE`) sehingga penargetan target window tetap 100% presisi.
3. **Hidden Persistent Window Strategy**:
   - WebView2 / UI window diinisialisasi saat boot aplikasi dan disembunyikan.
   - Eliminasi delay rendering saat pemanggilan pertama.
4. **Adaptive Polar Coordinate Layout**:
   - Kalkulasi irisan cincin berbasis sudut dan radius polar.
   - Pengecekan batas layar (bounding box monitor) agar menu tidak keluar dari viewport monitor.
5. **Dynamic App Profile Matching**:
   - Deteksi otomatis proses aktif melalui `GetForegroundWindow` + `QueryFullProcessImageNameW`.
   - Format penyimpanan profil berbasis JSON lokal (`profiles/default.json`, `profiles/<process_name>.json`).
6. **Built-in Macro Runner**:
   - Dukungan blok tindakan: shortcut key combo, delay terukur, pengetikan teks, pembukaan path/URL.
7. **Offline & Privacy-First**:
   - Tidak ada pengiriman telemetri, tidak membutuhkan akun, data konfigurasi transparan dalam folder data lokal pengguna.

## 5. Technical Constraints & Non-Goals
- **Operating System**: Windows 10 (1809+) dan Windows 11.
- **Footprint Target**: Ukuran installer < 15MB, RAM idle < 50MB, CPU idle 0%.
- **Non-Goals (Fase 1)**:
  - Sinkronisasi cloud / akun pengguna (out of scope).
  - Integrasi driver hardware khusus vendor (out of scope).
  - Emulator game controller / direct input injection khusus game anti-cheat (out of scope).
