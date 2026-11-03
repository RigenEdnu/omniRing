# Implementation Plan: omniRing

Rencana implementasi disusun secara bertahap, modular, dan setiap langkah dapat diverifikasi secara independen sebelum melangkah ke tahap berikutnya.

---

### Langkah 1: Scaffolding Proyek Tauri v2, Struktur Workspace & CI/CD Pipeline
- **Tujuan**: Menyiapkan struktur dasar proyek Tauri v2 dengan backend Rust, frontend web ringan (Vanilla TypeScript/HTML/CSS), serta workflow GitHub Actions untuk build otomatis di cloud.
- **Aktivitas**:
  - Konfigurasi `Cargo.toml` dengan dependensi inti (`tauri`, `windows-sys` / `windows`, `serde`, `serde_json`, `crossbeam-channel`).
  - Menyiapkan struktur folder `src-tauri` dan folder frontend `src`.
  - Mengonfigurasi `tauri.conf.json` dengan window tunggal `overlay` bertipe transparan, borderless, dan always-on-top.
  - Membuat workflow `.github/workflows/build.yml` (Windows runner, setup Node.js, setup Rust toolchain, `cargo check`, dan `cargo tauri build`).
- **Validasi / Kriteria Sukses**:
  - Struktur manifest `Cargo.toml`, `package.json`, dan `tauri.conf.json` valid dan konsisten.
  - File workflow GitHub Actions `.github/workflows/build.yml` siap dieksekusi oleh runner Windows.

---

### Langkah 2: Win32 Non-Activating Window & Hidden Persistent Window
- **Tujuan**: Mengimplementasikan strategi jendela overlay yang tidak mencuri fokus keyboard (`WS_EX_NOACTIVATE`) dan standby di memori.
- **Aktivitas**:
  - Menerapkan hook subclassing atau styling Win32 pada window Tauri saat inisialisasi: memasang `WS_EX_NOACTIVATE`, `WS_EX_TOPMOST`, dan `WS_EX_TOOLWINDOW`.
  - Mengatur window agar langsung tersembunyi (`ShowWindow(SW_HIDE)`) setelah pembuatan untuk eliminasi cold-start latency.
  - Menyediakan handler Tauri command internal untuk memindahkan koordinat window dan menampilkannya secara instan (`SW_SHOWNOACTIVATE`).
- **Validasi / Kriteria Sukses**:
  - Jendela overlay dapat muncul di layar tanpa mengubah foreground window aktif (aplikasi seperti Notepad atau terminal tetap mempertahankan fokus kursor teksnya).

---

### Langkah 3: Dedicated Low-Level Input Hook Worker Thread
- **Tujuan**: Membangun listener tombol mouse dan keyboard tingkat rendah yang aman dari pembatalan (timeout) oleh Windows.
- **Aktivitas**:
  - Membuat thread terisolasi di Rust yang menjalankan Windows Message Loop (`GetMessageW`).
  - Memasang `SetWindowsHookExW` (`WH_MOUSE_LL`).
  - Menangkap event tombol pemicu (default: Mouse Button 4/5 atau tombol yang dikonfigurasi).
  - Melakukan supresi klik (`LRESULT(1)`) saat tombol pemicu berada dalam fase evaluasi/penahanan.
  - Mengirimkan raw event ke receiver internal melalui channel `crossbeam-channel` non-blocking.
- **Validasi / Kriteria Sukses**:
  - Log pengujian mendeteksi aksi tekan (down), lepas (up), dan koordinat kursor secara konsisten.
  - Penahanan tombol pemicu di atas jendela browser atau teks editor tidak memicu aksi klik bawaan aplikasi tersebut.

---

### Langkah 4: Gesture Engine & Flick Directional Detection
- **Tujuan**: Membedakan input klik standar, gestur cepat 4 arah (flick), dan penahanan untuk radial menu.
- **Aktivitas**:
  - Membangun state machine dengan 4 status: `Idle`, `PendingHold`, `Flicking`, dan `RadialActive`.
  - Merekam titik awal `(X0, Y0)` dan timestamp saat tombol ditekan.
  - Menghitung delta perpindahan `(dX, dY)` dan sudut vektor arah `atan2`.
  - Mengirim event gestur terarah jika jarak geser melebihi batas (contoh: 40px) sebelum radial timer terpicu.
- **Validasi / Kriteria Sukses**:
  - Geser kursor cepat ke atas menghasilkan event `GestureUp`, geser ke bawah menghasilkan `GestureDown`, tanpa memunculkan menu radial.
  - Klik tanpa geser (< 150ms) meneruskan klik normal ke sistem.

---

### Langkah 5: Multi-Monitor Geometry & Screen Boundary Clamping
- **Tujuan**: Memastikan menu radial selalu muncul di posisi kursor dan 100% berada di dalam batas monitor aktif.
- **Aktivitas**:
  - Mengambil posisi kursor saat ini via `GetCursorPos`.
  - Mengambil informasi monitor tempat kursor berada via `MonitorFromPoint` + `GetMonitorInfoW`.
  - Melakukan kalkulasi batas (clamping): jika `cursor.x - radius < monitor.left`, posisikan `window.x = monitor.left + radius`, dan seterusnya untuk keempat sisi layar.
- **Validasi / Kriteria Sukses**:
  - Menekan trigger di pojok paling kanan-bawah layar monitor tidak menyebabkan menu terpotong ke luar layar monitor.

---

### Langkah 6: Frontend Radial Menu Polar Renderer
- **Tujuan**: Merender antarmuka cincin melingkar yang mulus, responsif, dan interaktif.
- **Aktivitas**:
  - Mengimplementasikan kalkulasi pembagian sudut `slice_angle = 360 / n` pada Canvas/SVG.
  - Menggambar cincin dalam (inner radius), cincin luar (outer radius), garis pemisah, dan teks/icon irisan.
  - Menangani mouse move untuk kalkulasi sudut kursor relatif terhadap titik pusat lingkaran guna mengaktifkan status hover irisan.
  - Mendukung transisi animasi cepat (fade-in / scale-up < 100ms).
- **Validasi / Kriteria Sukses**:
  - Irisan menu merespon gerakan mouse dengan highlighting yang presisi sesuai sudut irisan yang ditunjuk kursor.

---

### Langkah 7: Dynamic App Profile Watcher & Config Manager
- **Tujuan**: Mengaitkan set irisan menu dengan aplikasi yang sedang aktif di layar pengguna.
- **Aktivitas**:
  - Mengimplementasikan `SetWinEventHook` untuk event `EVENT_SYSTEM_FOREGROUND`.
  - Membaca executable path aplikasi foreground via `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`.
  - Memuat file profil JSON (`profiles/<app_name>.json` atau `default.json`).
  - Mengirim data struktur menu yang telah disesuaikan ke frontend via Tauri event.
- **Validasi / Kriteria Sukses**:
  - Saat berpindah antara dua aplikasi berbeda (misal VS Code dan Notepad), data irisan menu di frontend otomatis diperbarui sesuai profil masing-masing.

---

### Langkah 8: Action Dispatcher & Macro Execution
- **Tujuan**: Mengeksekusi tindakan yang dipilih dari irisan radial ke jendela aplikasi target.
- **Aktivitas**:
  - Implementasi eksekusi aksi:
    - `Shortcut`: Injeksi kombinasi tombol via `SendInput`.
    - `TypeText`: Injeksi deretan teks via `KEYEVENTF_UNICODE`.
    - `OpenApp` / `OpenUrl`: Membuka executable atau link web.
    - `Submenu`: Mengganti cincin aktif ke child ring tanpa menutup jendela.
    - `Sequence`: Menjalankan urutan aksi dengan delay terukur.
- **Validasi / Kriteria Sukses**:
  - Memilih irisan shortcut (contoh `Ctrl+S` atau `Ctrl+V`) berhasil tersimpan atau ter-paste pada aplikasi target yang sedang aktif di bawah menu radial.

---

### Langkah 9: System Tray & Tray Controls
- **Tujuan**: Menyediakan akses kontrol bagi pengguna melalui taskbar Windows.
- **Aktivitas**:
  - Mendaftarkan icon tray Windows dengan menu: Open Config Folder, Reload Profiles, Toggle Enable/Disable, dan Exit.
- **Validasi / Kriteria Sukses**:
  - Icon tray muncul di notification area Windows dan seluruh opsi menu berfungsi sesuai peruntukannya.
