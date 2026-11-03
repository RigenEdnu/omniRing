# Reverse Engineering Analysis: Logi Options+ Ecosystem & Architecture

- Target: `logioptionsplus_installer.exe`
- Arsitektur Teranalisis: Logitech Options+ Client Daemon & Overlay Engine
- Tipe Target: PE32+ Executable (Installer Bootstrapper .NET/WPF + Native Hook & Agent Runtime)

## 1. Ikhtisar Arsitektur Logi Options+

Logi Options+ dirancang sebagai utilitas produktivitas sistem yang memadukan input hook tingkat rendah dengan antarmuka grafis overlay yang kaya:

1. **Background Service / Agent**:
   - Berjalan permanen sebagai proses background (`logioptionsplus_agent.exe`).
   - Bertanggung jawab memonitor event input perangkat keras (mouse buttons, gesture trigger, smart crown/wheel).
   - Menjalankan IPC lokal (biasanya via named pipes atau WebSocket lokal) untuk berkomunikasi dengan proses frontend/overlay.
2. **Input Hook & Gesture Engine**:
   - Memonitor tombol khusus mouse (misalnya Gesture Button / Thumb Button).
   - **State Machine Gesture**:
     - *State 1 (Idle)*: Menunggu tombol pemicu ditekan.
     - *State 2 (Press / Down)*: Mengunci input tombol pemicu dan merekam koordinat awal kursor `(X0, Y0)`.
     - *State 3 (Hold + Drag)*: Jika kursor bergeser melebihi ambang batas delta (threshold `dX` / `dY`), gesture terdeteksi sesuai vektor arah dominan:
       - Up: `dY < -threshold`
       - Down: `dY > threshold`
       - Left: `dX < -threshold`
       - Right: `dX > threshold`
     - *State 4 (Release)*: Jika tombol dilepas tanpa ada perpindahan signifikan sebelum timeout tertentu, aksi tap standar dieksekusi (atau memunculkan menu radial).
3. **App-Specific Profile Switching**:
   - Menggunakan hook Windows foreground (`SetWinEventHook` dengan event `EVENT_SYSTEM_FOREGROUND`) atau polling event `GetForegroundWindow`.
   - Mengambil nama executable proses aktif melalui `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`.
   - Menyesuaikan tabel pemetaan aksi (action profile) secara instan berdasarkan ID proses atau nama executable aktif (misal `chrome.exe`, `code.exe`, `photoshop.exe`, atau profil fallback `All Applications`).
4. **Non-Activating OSD / Overlay**:
   - Menu overlay atau notifikasi visual gesture ditampilkan menggunakan window bergaya `WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW`.
   - Jendela aktif pengguna (aplikasi kerja yang sedang dipakai) tidak kehilangan fokus input.
   - Shortcut keyboard atau injeksi event input (`SendInput`) langsung mendarat pada jendela target yang benar tanpa jeda switching fokus.

## 2. Fitur Inti yang Diadopsi untuk omniRing

1. **Trigger Berbasis Gesture + Click (Dual-Mode Trigger)**:
   - Tombol mouse (misal Mouse Button 4/5 atau Middle Button) dapat memiliki dua fungsi:
     - Tahan + Gerakkan mouse: Mode Gesture Cepat (misal 4 arah untuk aksi navigasi instan).
     - Tahan tanpa geser / Tap: Membuka menu radial `omniRing` di posisi kursor.
2. **App-Profile Reactive Matching**:
   - omniRing akan memiliki konfigurasi profil per-aplikasi:
     - `default.json`: Pengaturan global untuk semua aplikasi.
     - `<process_name>.json`: Pengaturan khusus aplikasi tertentu (misal `code.exe`, `blender.exe`, `figma.exe`).
   - Setiap kali window target berubah, omniRing langsung mengaitkan menu radial dengan profil yang sesuai.
3. **Hardware Agnostic**:
   - Logi Options+ mengunci fitur ini khusus hardware Logitech.
   - omniRing mengimplementasikan logika gesture dan profile switching murni di level Win32 API standar (`SetWindowsHookExW` / `WH_MOUSE_LL`), sehingga dapat berjalan pada semua merk mouse dan keyboard standar.

## 3. Kelemahan Logi Options+ yang Dihindari

1. **Overhead Sumber Daya (Bloat)**:
   - Logi Options+ mengonsumsi beberapa ratus megabyte memori karena arsitektur multi-proses Electron + background agent + updater.
   - omniRing menggunakan Rust core (Tauri v2) dengan hidden lightweight webview atau native rendering, menjaga penggunaan RAM minimal (<50 MB) dan CPU idle 0%.
2. **Telemetri & Keterikatan Akun**:
   - Logi Options+ mewajibkan login akun cloud dan mengirimkan telemetri berkala.
   - omniRing bekerja 100% lokal dan offline dengan file konfigurasi JSON transparan yang mudah di-backup atau diedit manual.
