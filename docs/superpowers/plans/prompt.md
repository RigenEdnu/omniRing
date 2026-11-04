# OmniRing: Task Execution Prompts

File ini berisi panduan prompt ringkas dan siap pakai untuk mengeksekusi setiap task rencana implementasi secara terpisah per sesi.

File referensi rencana lengkap:
`docs/superpowers/plans/2026-11-04-ui-redesign-and-feature-development.md`

---

## Aturan Lingkungan Penting (Wajib Diikuti di Setiap Sesi)
- Jangan menjalankan perintah `cargo`, `rustc`, `cargo check`, atau `cargo build` di host Windows lokal (tidak ada compiler lokal).
- Verifikasi lokal backend dilakukan melalui inspeksi statis kode, tipe Win32, dan sintaksis.
- Verifikasi frontend dilakukan via `npm run build` (`tsc && vite build`).
- Kompilasi biner penuh didelegasikan ke GitHub Actions CI.

---

## Sesi 1: Task 1 - Redesign Canvas Polar Renderer (Bubble Nodes UI)

```markdown
Lanjutkan pengerjaan omniRing untuk Task 1 dari dokumen rencana:
`docs/superpowers/plans/2026-11-04-ui-redesign-and-feature-development.md`

Tujuan:
Ubah perenderan Canvas di `src/scripts/canvas_renderer.ts` dan `src/styles/overlay.css` dari model irisan pie menjadi antarmuka Bubble Nodes sirkular sesuai referensi `docs/bubbles.webp`.

Spesifikasi:
1. Hub tengah: tombol batal lingkaran kecil (r = 22px) dengan tanda silang '×'.
2. Node aksi: 8 lingkaran hitam (r = 24px) di radius orbital 95px dengan ikon vektor putih (2px stroke).
3. Label mengambang: pill badge putih dengan drop shadow halus di radius orbital 175px sepanjang vektor radial.
4. Efek hover: node membesar (1.12x), glow accent ring, dan kontras pill badge.
5. Hit-testing: berbasis sudut kuadran polar Fitts's Law + proteksi dead-zone di hub tengah.

Validasi:
Buat script verifikasi logika polar, lalu jalankan `npm run build` untuk memastikan TypeScript dan bundle lolos tanpa error.
```

---

## Sesi 2: Task 2 - Native Trigger System & Auto-Close Engine (Rust Backend)

```markdown
Lanjutkan pengerjaan omniRing untuk Task 2 dari dokumen rencana:
`docs/superpowers/plans/2026-11-04-ui-redesign-and-feature-development.md`

Tujuan:
Perluas sistem trigger dan auto-close di modul Rust Win32 backend (`src-tauri/src/hook/worker.rs`, `src-tauri/src/gesture/detector.rs`, `src-tauri/src/lib.rs`).

Spesifikasi:
1. Dukungan TriggerMode: Hold (X1/Middle), Combo (modifier keyboard + mouse, misal Ctrl + Right Click via GetAsyncKeyState), dan Corner Hover dwell.
2. Auto-close saat klik di luar menu: Pada WH_MOUSE_LL, jika menu aktif dan klik terjadi di luar radius menu, sembunyikan overlay secara otomatis.
3. Penutupan via Esc: Pasang hook WH_KEYBOARD_LL ringan untuk mendeteksi VK_ESCAPE saat menu aktif, batalkan overlay, dan supresi event Esc (LRESULT(1)).
4. Tauri Commands: Sediakan command untuk get/set konfigurasi trigger.

Validasi:
Lakukan inspeksi statis kode Rust, pastikan struktur struct/enum, tipe Win32, dan registrasi command Tauri konsisten tanpa error.
```

---

## Sesi 3: Task 3 - Configurator Window & Action Library UI (`menu.png`)

```markdown
Lanjutkan pengerjaan omniRing untuk Task 3 dari dokumen rencana:
`docs/superpowers/plans/2026-11-04-ui-redesign-and-feature-development.md`

Tujuan:
Bangun Window Konfigurator Pengaturan OmniRing kedua (`config.html`, `src/scripts/config.ts`, `src/styles/config.css`) berdasarkan referensi `docs/menu.png` (Screenshot 2026-11-04 105827.png).

Spesifikasi:
1. Multi-page Vite & Tauri: Daftarkan window "configurator" di `src-tauri/tauri.conf.json` dan `vite.config.ts`.
2. Header: Tab pemilih profil aplikasi (Default, VS Code, Notepad, + Tambah Profil).
3. Workspace Kiri (70%): Preview interaktif 8-slot wheel OmniRing dengan node bubble dan badge terpilih.
4. Sidebar Kanan (30%): Action Library dengan kolom pencarian instan dan accordion kategori (System Actions, Navigation, Shortcuts, App Launchers).
5. Tab Pengaturan: Pengaturan mode trigger (Hold, Combo, Corner Hover) dan gesture flick.

Validasi:
Jalankan `npm run build` untuk memastikan bundling multi-page (`index.html` dan `config.html`) sukses tanpa error.
```

---

## Sesi 4: Task 4 - System Tray Integration & Interaction Coordination

```markdown
Lanjutkan pengerjaan omniRing untuk Task 4 dari dokumen rencana:
`docs/superpowers/plans/2026-11-04-ui-redesign-and-feature-development.md`

Tujuan:
Integrasikan interaksi end-to-end antara System Tray, Window Konfigurator, dan Overlay Live (`src-tauri/src/tray/mod.rs`, `src-tauri/src/lib.rs`, `src/scripts/main.ts`).

Spesifikasi:
1. System Tray: Tambahkan menu "Configure OmniRing" yang menampilkan dan memberi fokus ke Window Konfigurator.
2. Overlay Coordinator: Perbarui `src/scripts/main.ts` agar menangani klik hub batal (×), event dismiss Esc, dan auto-close instan sebelum eksekusi aksi.
3. Sinkronisasi IPC: Pastikan perubahan profil di konfigurator langsung tersinkronisasi ke overlay live runtime.

Validasi:
Jalankan `npm run build` dan verifikasi bahwa alur pembukaan jendela, penutupan otomatis, dan tray command terhubung sempurna.
```

---

## Sesi 5: Task 5 - Memory Bank Update & Dokumentasi Akhir

```markdown
Lanjutkan pengerjaan omniRing untuk Task 5 dari dokumen rencana:
`docs/superpowers/plans/2026-11-04-ui-redesign-and-feature-development.md`

Tujuan:
Perbarui seluruh dokumentasi arsitektur dan riwayat progres proyek omniRing.

Spesifikasi:
1. Perbarui `memory-bank/architecture.md` dengan struktur file baru (window konfigurator, multi-page build, modul trigger).
2. Perbarui `memory-bank/design-document.md` dengan arsitektur UI Bubble Nodes dan sistem multi-mode trigger.
3. Catat milestone penyelesaian pada `memory-bank/progress.md`.
4. Perbarui panduan pengguna di `README.md`.

Validasi:
Periksa sintaksis markdown, pastikan tidak ada karakter em dash, dan jalankan `npm run build` untuk verifikasi akhir.
```
