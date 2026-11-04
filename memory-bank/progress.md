# Progress Log: omniRing

## Status Proyek Saat Ini
- **Fase**: Langkah 9 Selesai (System Tray & Tray Controls) + Penyelarasan Dependensi Windows Crate v0.62 untuk CI
- **Status Eksekusi**: Siap masuk ke Langkah 10 (Settings & Persistence / End-to-End Release Validation)

---

## Log Eksekusi Milestone

### [Peningkatan CI / GitHub Actions] Pengunggahan Otomatis Log Error saat Build Gagal (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Penambahan direktori `logs/` dan pengaliran output (`2>&1 | tee`) untuk seluruh tahap build:
     - `logs/npm-install.log`
     - `logs/typecheck.log`
     - `logs/frontend-build.log`
     - `logs/cargo-check.log`
     - `logs/tauri-build.log`
  2. Penambahan step `Upload Error Logs` menggunakan `actions/upload-artifact@v4` dengan kondisi `if: failure()`, sehingga ketika terjadi kegagalan di salah satu tahap build, seluruh log error otomatis diarsipkan ke artifact `build-error-logs`.
  3. Konfigurasi `RUST_BACKTRACE: 1` pada environment job untuk backtrace lengkap jika terjadi panic pada toolchain Rust.
- **Hasil Verifikasi**:
  - Konfigurasi workflow `.github/workflows/build.yml` tervalidasi bersih.
  - Verifikasi pipeline lokal bash dan `npm run build` berjalan sukses.

### [Perbaikan CI / Cargo Check] Penyelarasan Dependensi `windows` Crate ke `0.62` (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Penyelarasan versi crate `windows` di `src-tauri/Cargo.toml` dari `0.58` ke `0.62` agar sesuai dengan dependensi `windows` (`0.62.2`) yang ditarik oleh Tauri v2.12+ pada CI GitHub Actions runner.
  2. Mengatasi mismatch tipe `expected HWND, found windows::Win32::Foundation::HWND` pada `src-tauri/src/lib.rs` saat memanggil `hide_overlay`, `show_overlay_clamped`, dan `apply_overlay_styles`.
  3. Pembaruan pengecekan handle `hwnd` di `src-tauri/src/profile/watcher.rs` (baris 40, 113, 141, 147) menggunakan `hwnd.0 as usize == 0` / `!= 0` yang aman dan kompatibel terhadap perubahan representasi internal handle `HWND`.
- **Hasil Verifikasi**:
  - `npm run build` sukses mengompilasi TypeScript dan mem-bundle aset web (0 error, 0 warning).
  - Inspeksi statis kode memvalidasi seluruh simbol Win32 API tetap konsisten dan siap untuk `cargo check` di GitHub Actions CI.

### [Langkah 9] System Tray & Tray Controls (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Implementasi modul system tray di `src-tauri/src/tray/mod.rs` menggunakan Tauri v2 `TrayIconBuilder` dan `tauri::menu`:
     - Pendaftaran context menu tray dengan 4 opsi utama sesuai spesifikasi:
       - `Open Config Folder` (`open_config`): membuka folder konfigurasi/profil di Windows File Explorer (`profile::config::resolve_profiles_dir`).
       - `Reload Profiles` (`reload_profiles`): memuat ulang seluruh berkas profil JSON, mencocokkan profil proses foreground saat ini, memperbarui profil aktif, serta memancarkan event `omniring://profile-changed`.
       - `Toggle Enable/Disable` (`toggle_enabled`): opsi `CheckMenuItem` (default: checked/enabled) untuk mengaktifkan atau menonaktifkan hook input dan gestures omniRing secara dinamis.
       - `Exit` (`exit`): mematikan aplikasi secara bersih (`app.exit(0)`).
     - Konfigurasi icon tray via `app.default_window_icon()` dengan tooltip `"omniRing - Radial Menu & Gesture Ring"`, dukungan double-click untuk membuka folder konfigurasi, dan menu klik kanan non-mengganggu.
  2. Perluasan modul low-level hook worker di `src-tauri/src/hook/worker.rs`:
     - Penambahan flag `enabled: bool` pada `HookConfig` (default: `true`).
     - Penambahan `enabled: AtomicBool` pada `HookGlobalState`.
     - Integrasi fast-path pada `mouse_ll_hook_proc`: jika status hook dinonaktifkan (`!state.enabled`), event langsung diteruskan ke `CallNextHookEx` tanpa supresi atau evaluasi gesture.
     - Penambahan method `is_enabled` dan `set_enabled` pada `HookWorker`.
     - Penambahan unit test `test_hook_config_custom_disabled` dan pembaruan `test_hook_config_default`.
  3. Integrasi state dan lifecycle pada `src-tauri/src/lib.rs`:
     - Deklarasi dan re-ekspor modul `tray`.
     - Registrasi state `AppTrayState` pada `tauri::Builder::default().manage(...)` untuk sinkronisasi runtime antara checkmark menu tray dan status aktif hook.
     - Inisialisasi system tray pada `setup` hook via `tray::setup_tray(app)`.
     - Penambahan dan registrasi Tauri command baru: `open_config_folder`, `is_omni_enabled`, `toggle_omni_enabled`, `set_omni_enabled`, dan `exit_app`.
     - Pembaruan handler `reload_profiles` untuk mendelegasikan eksekusi ke `tray::reload_profiles_internal`.
  4. Integrasi frontend pada `src/scripts/main.ts`:
     - Penambahan listener event `omniring://enabled-changed` untuk membersihkan canvas saat omniRing dinonaktifkan.
     - Ekspor fungsi helper TypeScript untuk interaksi tray/aplikasi: `openConfigFolder`, `isOmniEnabled`, `toggleOmniEnabled`, `setOmniEnabled`, dan `exitApp`.
  5. Penyusunan 3 unit test pada `src-tauri/src/tray/mod.rs` (`test_tray_menu_ids`, `test_app_tray_state_default`, `test_resolve_profiles_dir_valid`).
- **Hasil Verifikasi**:
  - `npm run build` (`tsc && vite build`) sukses mengompilasi TypeScript dan mem-bundle aset ke `dist/` (0 error, 0 warning).
  - Skrip pengujian statis Python memverifikasi kelengkapan seluruh fungsi, konstanta ID menu, registrasi Tauri commands, dan kesesuaian arsitektur hook.

---

### [Langkah 8] Action Dispatcher & Macro Execution (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Implementasi modul action dispatcher di `src-tauri/src/action/dispatcher.rs` untuk eksekusi makro keyboard dan aksi sistem:
     - `Shortcut`: Parsing nama tombol ke Win32 Virtual-Key codes (`parse_key_to_vk`) dengan dukungan modifier (Ctrl, Shift, Alt, Win/Super), huruf A-Z, angka 0-9, F1-F24, navigation/editing keys, tombol simbol/tanda baca, lock keys, dan media controls. Perencanaan urutan penekanan (down) dan pelepasan terbalik (up) via `plan_shortcut_events`, serta deteksi flag extended key (`KEYEVENTF_EXTENDEDKEY`).
     - `TypeText`: Konversi string teks ke UTF-16 code units dan injeksi via `SendInput` dengan flag `KEYEVENTF_UNICODE` (down & up pair).
     - `OpenUrl`: Validasi skema URL yang aman (`http://`, `https://`, `mailto:`, `ftp://`) dan peluncuran default browser via `rundll32.exe url.dll,FileProtocolHandler`.
     - `OpenApp`: Peluncuran aplikasi executable atau command line eksternal via detached process `cmd.exe /c start ""`.
     - `Sequence`: Eksekusi urutan rangkaian aksi berantai dengan delay terukur (`delay_ms`).
     - `Submenu`: Dukungan navigasi cincin hierarkis (child rings).
  2. Implementasi `dispatch_raw_key_events` menggunakan `SendInput` dari `windows::Win32::UI::Input::KeyboardAndMouse`.
  3. Re-ekspor modul `action` di `src-tauri/src/action/mod.rs` (`execute_action`, `execute_shortcut`, `execute_type_text`, `execute_open_url`, `execute_open_app`, `execute_sequence`, `parse_key_to_vk`, dll).
  4. Perluasan model `ProfileConfig` di `src-tauri/src/profile/config.rs` dengan `SubmenuConfig` dan field opsional `submenus: Option<HashMap<String, SubmenuConfig>>`, serta method `get_ring_slices(ring_id)` untuk resolusi irisan cincin dinamis.
  5. Penambahan dan registrasi Tauri commands di `src-tauri/src/lib.rs`: `execute_action` dan `execute_slice_action`. Jendela overlay disembunyikan secara otomatis sebelum aksi non-submenu dieksekusi agar fokus jendela target tidak terhalang.
  6. Pembaruan `handle_gesture_event` pada event `RadialRelease` agar tidak menutup overlay secara prematur saat pengguna mengonfirmasi irisan submenu.
  7. Penambahan contoh konfigurasi submenu pada `profiles/code.json` (sub-ring `git_tools` dengan aksi `Git View`, `Status Cmd`, dan tombol `Back` kembali ke root).
  8. Pembaruan frontend `src/scripts/canvas_renderer.ts`:
     - Dukungan `SubmenuConfig` dan `submenus` pada `RingConfig`.
     - State tracking cincin aktif (`currentRingId`, default `"root"`).
     - Method `switchToRing(ringId)`, `getActiveRingId()`, `resetToRoot()`, dan `getCurrentSlices()`.
     - Rendering label "Back" pada inner hub lingkaran pusat ketika cincin submenu sedang aktif.
     - Penanganan animasi transisi pembuka saat beralih antar-ring.
  9. Pembaruan frontend `src/scripts/main.ts`:
     - Pada event IPC `omniring://radial-release`, deteksi apakah irisan terkonfirmasi merupakan `Submenu` (pindah ring tanpa menutup window) atau aksi terminal (panggil `execute_action` dan tutup overlay).
     - Pelepasan kursor di area dead-zone saat berada di submenu akan mengembalikan menu ke cincin root.
     - Sinkronisasi interaksi klik pada mode preview browser standalone (`mouseup`).
  10. Penyusunan 13 unit test lengkap di `dispatcher.rs` dan unit test submenu di `config.rs`.
- **Hasil Verifikasi**:
  - `npm run build` (`tsc && vite build`) sukses mengompilasi TypeScript dan mem-bundle aset web ke `dist/` (0 error).
  - Skrip verifikasi Node.js memvalidasi logika `CanvasRenderer` untuk navigasi submenu, hit-testing di child ring, dan kembali ke root ring.
  - Skrip verifikasi Python memvalidasi keutuhan schema JSON seluruh file profil di `profiles/`, kelengkapan 13 unit test, serta registrasi perintah dan re-ekspor simbol modul Rust.
- **Aktivitas yang Diselesaikan**:
  1. Implementasi model data konfigurasi profil di `src-tauri/src/profile/config.rs`: `ActionConfig`, `SliceItem`, `ProfileConfig` dengan implementasi default fallback, serialisasi/deserialisasi JSON, serta helper pencocokan proses `matches_process` yang mendukung exact filename, case-insensitive match, wildcard `*`, dan path stripping.
  2. Implementasi `ProfileManager` di `config.rs` untuk pemindaian folder profil otomatis (`resolve_profiles_dir`), parsing semua file `.json`, caching pemetaan proses ke profil, dan fallback aman ke profil default jika proses target belum terkonfigurasi.
  3. Implementasi `ForegroundWatcher` di `src-tauri/src/profile/watcher.rs` berbasis thread latar belakang terisolasi dengan message loop Win32 (`GetMessageW` / `DispatchMessageW`) dan `SetWinEventHook` mendengarkan event `EVENT_SYSTEM_FOREGROUND` (`WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS`).
  4. Implementasi ekstraksi identitas aplikasi foreground via `get_process_info_from_hwnd`: `GetWindowThreadProcessId` + `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW(PROCESS_NAME_WIN32)` untuk mengekstrak path biner dan nama file executable secara aman dan minim hak akses.
  5. Penambahan proteksi pengabaian proses internal omniRing (`own_pid`) dan deduplikasi event berulang agar pergantian profil tidak terjadi saat overlay omniRing aktif di atas jendela target.
  6. Integrasi state `AppProfileState` di `src-tauri/src/lib.rs` membungkus `ProfileManager`, `ForegroundWatcher`, dan `active_profile` dengan thread coordinator yang menerima notifikasi foreground change dan memancarkan event Tauri `omniring://foreground-changed` dan `omniring://profile-changed`.
  7. Penambahan Tauri command: `get_active_profile`, `get_foreground_process`, `reload_profiles`, `set_active_profile_by_name`, dan `list_available_profiles`.
  8. Pembaruan `handle_gesture_event` pada event `RadialOpen` untuk menggunakan nilai `outer_radius` dinamis dari profil aktif sebagai parameter penempatan radial clamping.
  9. Pembuatan contoh profil tambahan `profiles/notepad.json` (untuk `notepad.exe`) melengkapi `code.json` dan `default.json` guna memvalidasi skenario pergantian profil multi-aplikasi.
  10. Integrasi listener event `omniring://profile-changed` dan `omniring://foreground-changed` di `src/scripts/main.ts`, serta pemanggilan otomatis `get_active_profile` saat startup untuk sinkronisasi antarmuka cincin radial instan.
- **Hasil Verifikasi**:
  - `npm run build` (`tsc && vite build`) sukses mengompilasi TypeScript dan Vite bundle ke `dist/` (0 error, 0 warning).
  - Skrip pengujian otomatis memvalidasi seluruh schema file JSON di `profiles/`, logika pencocokan proses dengan variasi path/ekstensi/huruf besar-kecil, serta kelengkapan ekspor simbol modul Rust.

### [Langkah 6] Frontend Radial Menu Polar Renderer (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Implementasi matematika polar coordinates di `src/scripts/canvas_renderer.ts` dengan pembagian sudut dinamis `slice_angle = (2 * PI) / N` dan start angle offset `-PI/2 - slice_angle/2` sehingga Slice 0 tepat berada di posisi Atas (12 o'clock, sejalan dengan Flick Up).
  2. Implementasi algoritma hit-testing `hitTest(x, y, isRelative)` berbasis `r = hypot(dx, dy)` dan `theta = atan2(dy, dx)` dengan toleransi batas luar serta deteksi dead-zone di dalam `inner_radius` (< 50px).
  3. Implementasi rendering Canvas 2D annular sectors dengan styling glassmorphism untuk irisan idle (`rgba(20, 24, 34, 0.85)`), highlight glow beranimasi untuk irisan aktif (`rgba(59, 130, 246, 0.85)`), separator radial lines, inner hub circle, dan cursor indicator bead.
  4. Penyediaan icon vector bawaan berbasis canvas paths (`copy`, `paste`/`clipboard`, `undo`, `redo`, `terminal`, `code`, `search`, `layout`) tanpa dependensi font eksternal atau latensi pemuatan gambar.
  5. Penambahan animasi pembuka lembut (scale-up 0.8 -> 1.0, fade-in < 80ms) berbasis `requestAnimationFrame`.
  6. Integrasi event listener di `src/scripts/main.ts`:
     - `omniring://radial-open`: inisialisasi center relative kursor dan memicu animasi buka.
     - `omniring://radial-move`: kalkulasi koordinat kursor relatif terhadap titik pusat clamped placement dan aktivasi hover slice.
     - `omniring://radial-release`: konfirmasi eksekusi irisan terpilih atau pembatalan jika berada di dead-zone.
     - DOM mouse event listeners fallback untuk pengujian interaktif lokal pada browser mode.
  7. Penyusunan styling canvas high-DPI scaling (`devicePixelRatio`) dan `overlay.css` dengan hardware acceleration.
- **Hasil Verifikasi**:
  - `npm run build` (`tsc && vite build`) sukses mengompilasi TypeScript dan Vite bundle ke `dist/` (0 error).
  - Skrip pengujian otomatis via Node 24 (`--experimental-strip-types`) memvalidasi 100% akurasi hit-testing untuk seluruh 4 kuadran, dead-zone, dan batas luar.

### [Langkah 5] Multi-Monitor Geometry & Screen Boundary Clamping (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Implementasi struct `ScreenRect`, `MonitorGeometry`, dan `RadialPlacement` di `src-tauri/src/window/manager.rs` untuk representasi geometri virtual screen dan relative window canvas coordinates.
  2. Implementasi fungsi kalkulasi murni `calculate_radial_placement` yang menjamin cincin radial dengan radius tertentu tidak pernah keluar dari batas `work_rect` monitor aktif, serta memposisikan overlay window membungkus cincin tersebut secara presisi.
  3. Implementasi integrasi Win32 API via `windows` crate: `GetCursorPos` untuk posisi kursor fisik aktual, serta `MonitorFromPoint` + `GetMonitorInfoW` (`rcMonitor`, `rcWork`, `dwFlags`) untuk multi-monitor awareness.
  4. Penyusunan 7 unit test komprehensif di `manager.rs` yang menguji skenario: kursor di tengah layar monitor, pojok kiri-atas (0,0), pojok kanan-bawah (1920, 1040), monitor sekunder di sisi kiri (koordinat negatif -1920..0), monitor sekunder di sisi kanan (1920..3840), serta layar sempit/resolusi rendah.
  5. Penambahan method `show_overlay_clamped` dengan flag `SWP_NOACTIVATE | SWP_SHOWWINDOW` untuk menampilkan window tanpa mencuri fokus aktif keyboard.
  6. Re-ekspor modul geometri dan clamping di `src-tauri/src/window/mod.rs`.
  7. Penambahan dan registrasi command Tauri di `src-tauri/src/lib.rs`: `get_cursor_position`, `get_monitor_geometry`, `calculate_clamped_placement`, serta pembaruan command `show_overlay` agar menerima parameter radius dan mengembalikan `RadialPlacement`.
  8. Integrasi event router `handle_gesture_event` pada event `RadialOpen` untuk menghitung clamping dan mengirimkan payload `RadialPlacement` ke frontend.
  9. Pembaruan frontend `src/scripts/main.ts` dan `src/scripts/canvas_renderer.ts` untuk menangani relative center coordinates (`relative_center_x`, `relative_center_y`) agar perenderan cincin radial di canvas tetap akurat saat window ter-clamp di batas monitor.
- **Hasil Verifikasi**:
  - `npm run build` sukses mengompilasi TypeScript dan Vite bundle ke `dist/` (0 error).
  - Skrip simulasi Python memverifikasi logika matematis clamping untuk seluruh skenario termasuk pojok kanan-bawah dan monitor bernilai negatif.

### [Langkah 4] Gesture Engine & Flick Directional Detection (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Implementasi State Machine 4 status (`Idle`, `PendingHold`, `Flicking`, `RadialActive`) di `src-tauri/src/gesture/detector.rs`.
  2. Implementasi kalkulasi gestur directional 4 arah (`Up`, `Right`, `Down`, `Left`) berbasis `atan2(dY, dX)` dengan pembagian 4 kuadran masing-masing 90 derajat yang presisi di sistem koordinat layar.
  3. Implementasi deteksi Flick cepat (`>= 40px` sebelum batas hold timer 150ms) yang langsung memicu event gestur terarah dan mencegah pemunculan menu radial.
  4. Implementasi deteksi Tap cepat (`< 150ms` dan pergeseran `<= 10px`) dengan fungsi `replay_tap_click` untuk meneruskan klik mouse sintetis ke aplikasi target melalui Win32 `mouse_event`.
  5. Penambahan proteksi `LLMHF_INJECTED` pada hook `WH_MOUSE_LL` di `src-tauri/src/hook/worker.rs` agar event sintetis tidak terblokir atau menimbulkan loop hook tak berujung.
  6. Implementasi `GestureWorker` di `src-tauri/src/gesture/mod.rs` sebagai thread koordinator latar belakang yang mengonsumsi event hook via bounded channel dan mendeteksi timeout penahanan kursor.
  7. Integrasi state `AppGestureState`, dispatch event Tauri (`omniring://gesture`, `omniring://radial-open`, `omniring://radial-move`, `omniring://radial-release`, `omniring://flick`, `omniring://tap`, `omniring://cancelled`), dan command `get_gesture_status` di `src-tauri/src/lib.rs`.
  8. Pembaruan frontend `src/scripts/main.ts` untuk meregistrasi event listener IPC Tauri yang menangani seluruh event gesture.
  9. Penyusunan 11 unit test lengkap di `detector.rs` yang menguji arah kuadran, sudut batas, tap supresi, flick 4 arah, radial open timeout, radial move/release, dan pembatalan gestur.
- **Hasil Verifikasi**:
  - `npm run build` berhasil mengompilasi TypeScript dan mem-bundle aset web ke `dist/` (0 error).
  - Skrip pengujian statis Python memverifikasi 11 unit test, konsistensi struktur modul, dan integrasi Tauri state/command.

### [Langkah 3] Dedicated Low-Level Input Hook Worker Thread (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Implementasi `HookWorker` di `src-tauri/src/hook/worker.rs` dengan background thread terisolasi menjalankan Windows Message Loop (`GetMessageW` / `TranslateMessage` / `DispatchMessageW`).
  2. Pemasangan `SetWindowsHookExW` (`WH_MOUSE_LL`) dengan prosedur hook berkecepatan tinggi (`< few microseconds`) untuk mencegah timeout Windows.
  3. Implementasi `HookDecoder` yang mendecode event mouse (`WM_LBUTTONDOWN/UP`, `WM_RBUTTONDOWN/UP`, `WM_MBUTTONDOWN/UP`, `WM_XBUTTONDOWN/UP`, `WM_MOUSEMOVE`, `WM_MOUSEWHEEL`) beserta koordinat layar dan timestamp.
  4. Implementasi supresi input instan (`LRESULT(1)`) untuk tombol pemicu aktif (default Mouse Button 4 / X1) saat fase evaluasi/penahanan agar klik tidak tembus ke aplikasi latar belakang.
  5. Pengiriman event mouse mentah ke receiver internal menggunakan bounded channel `crossbeam-channel` (kapasitas 1024) dengan `try_send` non-blocking.
  6. Penyediaan method kontrol dinamis (`set_trigger_button`, `set_suppress_trigger`, `set_track_all_moves`, `stop`) serta registrasi state `AppHookState` dan Tauri command di `src-tauri/src/lib.rs`.
  7. Penyusunan 8 unit test lengkap untuk decoding tombol, delta wheel, aturan supresi, dan nilai default config.
- **Hasil Verifikasi**:
  - `npm run build` sukses (0 error, TypeScript dan Vite bundle valid).
  - Skrip inspeksi statis Rust memverifikasi seluruh simbol, ekspor modul, dan integrasi state.

### [Langkah 2] Win32 Non-Activating Window & Hidden Persistent Window (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Implementasi `apply_overlay_styles(hwnd)` di `src-tauri/src/window/manager.rs` untuk menerapkan `WS_EX_NOACTIVATE`, `WS_EX_TOPMOST`, dan `WS_EX_TOOLWINDOW` via `SetWindowLongPtrW` + `SetWindowPos`.
  2. Implementasi `calculate_clamped_position` berbasis `MonitorFromPoint` + `GetMonitorInfoW` (`MONITORINFO.rcWork`) untuk mencegah window overlay terpotong keluar dari batas layar monitor aktif.
  3. Implementasi `show_overlay_at_cursor` dengan flag `SWP_NOACTIVATE | SWP_SHOWWINDOW` dan `hide_overlay` (`ShowWindow(SW_HIDE)`).
  4. Pemasangan handler window pada lifecycle `setup` dan registrasi Tauri command `show_overlay` & `hide_overlay` di `src-tauri/src/lib.rs`.
  5. Penguncian aturan lingkungan host di `AGENTS.md` (larangan scan drive lokal dan pemanggilan `cargo` di host lokal tanpa toolchain).
- **Hasil Verifikasi**:
  - `npm run build` sukses mengompilasi TypeScript dan bundle Vite (0 error).
  - Tinjauan statis kode Rust memenuhi panduan `rust-best-practices` (error handling bersih, zero unwrap di logika runtime, tipe Win32 aman).

### [Langkah 1] Scaffolding Proyek Tauri v2, Struktur Workspace & CI/CD Pipeline (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Konfigurasi `src-tauri/Cargo.toml` dengan dependensi inti (`tauri v2`, `windows-sys/windows 0.58`, `serde`, `serde_json`, `crossbeam-channel`).
  2. Inisialisasi struktur frontend ringan Vanilla TypeScript + Vite di `src/` (`index.html`, `overlay.css`, `main.ts`, `canvas_renderer.ts`, `package.json`, `tsconfig.json`, `vite.config.ts`).
  3. Konfigurasi window tunggal overlay di `src-tauri/tauri.conf.json` dengan flag `transparent: true`, `decorations: false`, `alwaysOnTop: true`, `visible: false`.
  4. Penyusunan skeleton modul Rust di `src-tauri/src/` (`hook`, `gesture`, `window`, `profile`, `action`).
  5. Pembuatan profil konfigurasi awal di `profiles/default.json` dan `profiles/code.json`.
  6. Penyusunan workflow GitHub Actions `.github/workflows/build.yml` untuk runner Windows (Node 22, Rust toolchain, cargo check, frontend build, tauri build).
- **Hasil Verifikasi**:
  - `npm run build` sukses mengompilasi TypeScript dan mem-bundle aset web ke `dist/` (0 error).
  - Validasi schema manifest `Cargo.toml`, `package.json`, `tauri.conf.json`, serta profil JSON lulus uji parsing python script.

### [Fase 0] Reverse Engineering & Bootstrap Perencanaan (Selesai)
- **Aktivitas yang Diselesaikan**:
  1. Analisis arsitektur dan ekstraksi plugin/model Dart dari `14914FroltSoftware.RadialMenu_1.33.4.0_x64__v5vt3srnrv4et.Msix`. Hasil didokumentasikan di `docs/rea-radialmenu.md`.
  2. Analisis arsitektur daemon, input hook, dan state machine gesture dari `logioptionsplus_installer.exe`. Hasil didokumentasikan di `docs/rea-logioptions.md`.
  3. Penyusunan `PRD.md` mendefinisikan target produk, alur pengguna, fitur inti, dan batasan teknis.
  4. Penyusunan spesifikasi teknis lengkap di `memory-bank/design-document.md`.
  5. Penentuan tech stack di `memory-bank/tech-stack.md` (Tauri v2 + Rust Core + Win32 API + Vanilla Webview Overlay).
  6. Perumusan 9 langkah terverifikasi di `memory-bank/implementation-plan.md`.
  7. Pemetaan struktur direktori dan modul di `memory-bank/architecture.md`.
  8. Pembuatan direktori `SKILL/` dan pendokumentasian seluruh skill yang digunakan di `SKILL/skill.md`.
- **Hasil Verifikasi**:
  - Semua dokumen teknis telah terbentuk dan terverifikasi di filesystem.
  - Kesepakatan grill-me terpenuhi: strategi hidden persistent window, WH_MOUSE_LL terisolasi, non-activating overlay (WS_EX_NOACTIVATE), dan target stack Rust/Tauri v2.

---

## Rencana Milestone Berikutnya
- **Fase Verifikasi CI/CD**: Push commit ke GitHub repository untuk memicu GitHub Actions CI runner (`windows-latest`) guna memverifikasi kompilasi biner Rust (`cargo check` & `tauri build`).
- **Pengujian Integrasi Windows**: Unduh executable artifact dari GitHub Actions Release/Artifacts dan jalankan uji runtime pada Windows desktop aktif.
