# Reverse Engineering Analysis: Radial Menu (Frolt Software)

- Target: `14914FroltSoftware.RadialMenu_1.33.4.0_x64__v5vt3srnrv4et.Msix`
- Versi: 1.33.4.0 (x64)
- Penerbit: Frolt Software
- Tipe Target: Windows MSIX Desktop Package (Flutter Windows Desktop Engine)

## 1. Ikhtisar Arsitektur Runtime

Aplikasi ini dibangun menggunakan Flutter Windows Desktop x64 dengan serangkaian plugin Win32 native C++ dan utility bantu:
1. **Host Executable**: `radial_menu.exe` (entry point `Windows.FullTrustApplication`).
2. **Background / Startup Stub**: `startup_stub.exe` dengan parameter `StartHidden` yang didaftarkan melalui extension `windows.startupTask` pada `AppxManifest.xml`.
3. **Flutter Engine Core**: `flutter_windows.dll` (~21.2 MB) dan logic binary `data/app.so` (~11.1 MB).
4. **Native Plugin Layer (Win32 Interop)**:
   - `flutter_acrylic_plugin.dll`: Efek blur/acrylic/mica pada background window.
   - `hotkey_manager_windows_plugin.dll`: Registrasi global hotkey sistemik (`RegisterHotKey`).
   - `mouse_event_plugin.dll`: Menangkap koordinat dan event mouse di luar jendela aplikasi.
   - `window_manager_plugin.dll`: Pengendalian posisi window, borderless, always-on-top, show/hide tanpa focus stealing.
   - `screen_retriever_windows_plugin.dll`: Deteksi multi-monitor, koordinat DPI, dan geometri layar aktif.
   - `tray_manager_plugin.dll`: Integrasi system tray Windows dan menu taskbar.
   - `desktop_drop_plugin.dll`: Drag-and-drop file/shortcut ke dalam menu.
   - `super_native_extensions.dll`: Integrasi clipboard tingkat lanjut.
5. **Auxiliary Utility**:
   - `Thumbico/thumbicocli.exe`: Utilitas .NET CLI untuk mengekstrak ikon resolusi tinggi dari executable (`.exe`), shortcut (`.lnk`), dan file dokumen.
   - OpenCV (`opencv_core4.dll`, `opencv_imgproc4.dll`, `opencv_imgcodecs4.dll`): Pemrosesan gambar latar belakang dan masking lingkaran.

## 2. Struktur Modul & Model Data (Dari `data/app.so`)

Berdasarkan inspeksi simbol dan path Dart internal, struktur menu dimodelkan secara modular:

### A. Geometri & Layout Radial
- **Radial Shell (`radial_shell.dart`, `shell_layout_data.dart`)**:
  - Konsep cincin bertingkat (shell / rings): mendukung inner radius, outer radius, dan kalkulasi sudut (polar coordinate math).
  - Slices / Wedges: Pembagian lingkaran secara proporsional sesuai jumlah item aktif.
  - Center Controls: Tombol di tengah lingkaran untuk aksi meta (`add_button`, `edit_button`, `move_button`, `remove_button`, `settings_button`).
- **Screen Positioning**:
  - Fungsi `getMonitorCenterPosByPrimaryScreenPx` dan cursor clamping agar menu tidak terpotong tepi layar monitor saat dipanggil di dekat sudut monitor.

### B. Taksonomi Tipe Item Menu
1. **Shortcut Item (`shortcut_item_data.dart`)**:
   - Menjalankan file target, URL, atau simulasi shortcut keyboard.
2. **Submenu Item (`submenu_button_content.dart`)**:
   - Masuk ke cincin turunan (child shell) secara mulus tanpa membuat window baru.
3. **Clipboard Item (`clipboard_item_data.dart`)**:
   - Menampilkan riwayat clipboard untuk di-paste langsung.
4. **Macro Item (`macro_item_data.dart`)**:
   - Eksekusi serangkaian blok tindakan berurutan.
5. **Empty Item (`empty_button_content.dart`)**:
   - Slot kosong untuk penataan estetika lingkaran.

### C. Mesin Makro (Macro Blocks)
Ditemukan 14 blok tindakan modular dalam engine makro RadialMenu:
- `macro_block_press_key` / `macro_block_release_key`: Penekanan tombol individual.
- `macro_block_press_shortcut`: Kombinasi shortcut keyboard.
- `macro_block_type_text`: Injeksi teks instan.
- `macro_block_paste_text`: Paste isi clipboard ke aplikasi aktif.
- `macro_block_move_mouse`: Pemindahan koordinat kursor.
- `macro_block_scroll`: Simulasi scroll wheel.
- `macro_block_open_uri`: Membuka URL atau protokol Windows.
- `macro_block_run_command`: Eksekusi command line.
- `macro_block_activate_window`: Pemindahan fokus ke jendela aplikasi target.
- `macro_block_http_request`: Pemanggilan API HTTP eksternal.
- `macro_block_wait`: Delay/sleep terukur dalam milidetik.
- `macro_block_wait_for_input`: Pause sampai ada input pengguna.
- `macro_block_split`: Percabangan alur.

## 3. Evaluasi untuk omniRing

### Kelebihan yang Diadopsi:
- Pemodelan polar coordinate (sudut dan jari-jari) yang fleksibel untuk multi-ring/submenu.
- Konsep startup tersembunyi (`StartHidden`) untuk eliminasi cold-start latency.
- Taksonomi blok makro yang rapi dan terukur.
- Ekstraksi icon aplikasi secara dinamis dari file `.exe` / `.lnk`.

### Kelemahan yang Dihindari:
- Ukuran binary sangat besar (~75 MB MSIX, 184 MB unpacked) akibat runtime Flutter dan dependency OpenCV/Firebase.
- Konsumsi memori tinggi (Flutter desktop engine idle membutuhkan 80-150 MB RAM).
- Kompleksitas interop plugin C++ yang terpisah-pisah via IPC channel Flutter.
