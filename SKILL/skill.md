# Daftar Skill Proyek: omniRing

Dokumen ini mencatat seluruh skill yang telah, sedang, dan akan digunakan dalam perancangan serta pembangunan proyek omniRing.

---

## 1. Skill Fondasi & Perencanaan (Telah Digunakan)

### A. `reverse-engineer-anything` (REA)
- **Tujuan**: Menganalisis binary referensi yang ada (`14914FroltSoftware.RadialMenu` dan `logioptionsplus_installer.exe`).
- **Implementasi pada omniRing**:
  - Menginspeksi `AppxManifest.xml`, dependency DLL, dan runtime Flutter `data/app.so` untuk memetakan arsitektur radial menu, model polar coordinate, dan taksonomi makro.
  - Menginspeksi struktur Logi Options+ untuk mengekstrak state machine gesture 4 arah, mekanisme foreground process watcher, dan penanganan non-activating overlay.
  - Output: `docs/rea-radialmenu.md` dan `docs/rea-logioptions.md`.

### B. `reference-grounded-design`
- **Tujuan**: Menjembatani ekstraksi binary referensi ke perancangan arsitektur modern tanpa terjebak dekompilasi mendalam yang tidak efisien.
- **Implementasi pada omniRing**:
  - Memisahkan mekanisme yang diadopsi (polar math, startup hidden, dual-mode gesture/ring, zero focus stealing) dari anti-pattern yang dibuang (bloat runtime Flutter/Electron, cloud telemetry, lock hardware vendor).

### C. `grill-me`
- **Tujuan**: Wawancara adversarial (stress-testing) rencana teknis sebelum kode ditulis.
- **Implementasi pada omniRing**:
  - Menguji trade-off tech stack (memilih Rust + Tauri v2 untuk efisiensi RAM <50MB).
  - Mengunci keputusan arsitektur kritis: isolated thread untuk `WH_MOUSE_LL` guna mencegah OS hook timeout, supresi klik pemicu, dan window style `WS_EX_NOACTIVATE` agar tidak mencuri fokus aplikasi aktif.

### D. `vibe-coding-standard-workflow`
- **Tujuan**: Menerapkan alur pengembangan document-first dengan single-step execution loop yang terdisiplin.
- **Implementasi pada omniRing**:
  - Membangun `PRD.md` dan struktur `memory-bank/` (`design-document.md`, `tech-stack.md`, `implementation-plan.md`, `architecture.md`, `progress.md`).
  - Menetapkan aturan eksekusi 1 langkah dalam satu waktu dengan verifikasi sebelum melangkah ke tahap berikutnya.

### E. `graphify`
- **Tujuan**: Memetakan hubungan antar berkas, abstraksi inti (god nodes), dan komunitas komponen dalam bentuk knowledge graph interaktif.
- **Implementasi pada omniRing**:
  - Mengekstraksi 568 nodes dan 922 edges mencakup modul Win32, gesture state machine, polar canvas renderer, dan action dispatcher ke dalam `graphify-out/` (`graph.html`, `graph.json`, `GRAPH_REPORT.md`).

---

## 2. Skill Eksekusi & Implementasi Rust (Sedang / Akan Digunakan)

### A. `rust-async-patterns`
- **Lokasi**: `C:\Users\RigenEdnu\.hermes\skills\rust-async-patterns`
- **Tujuan**: Standar arsitektur konkurensi dan pemrograman asinkron berbasis Tokio.
- **Implementasi pada omniRing**:
  - Menjaga isolasi thread: Worker thread Win32 hook (`WH_MOUSE_LL`) berjalan di OS thread terpisah dengan Windows Message Loop sinkron, lalu mengirim event ke Tokio async runtime via non-blocking bounded channels (`crossbeam` / `mpsc`).
  - Menerapkan `tokio::select!` untuk menangani pembatalan (cancellation), timeout gestur (batas hold timer 150ms), dan sinyal shutdown aplikasi secara anggun.
  - Larangan blocking call di dalam task async (tidak menggunakan `std::thread::sleep` di runtime async).
  - Instrumentasi event pipeline menggunakan `tracing` untuk debugging aliran event tanpa overhead berat.

### B. `rust-best-practices`
- **Lokasi**: `C:\Users\RigenEdnu\.hermes\skills\rust-best-practices`
- **Tujuan**: Menulis kode Rust yang idiomatik, efisien, aman, dan mematuhi panduan standar produksi (Apollo Handbook).
- **Implementasi pada omniRing**:
  - **Ownership & Borrowing**: Mengutamakan `&str` dan slice referensi daripada clone berulang; meminimalkan alokasi heap saat pemrosesan event kursor berfrekuensi tinggi (60-1000Hz).
  - **Error Handling**: Menghindari `unwrap()` dan `expect()` pada kode produksi; menggunakan `thiserror` untuk domain error modul internal dan `anyhow` pada level aplikasi/command Tauri.
  - **Type-State Pattern**: Menerapkan validasi status gesture pada level compile-time (misal `Gesture<Idle>`, `Gesture<PendingHold>`, `Gesture<RadialActive>`) untuk mencegah transisi status ilegal.
  - **Linting & Hygiene**: Memastikan kode lolos pemeriksaan `cargo clippy` dengan zero warnings pada pipeline CI.

### C. `win32-input-hooks`
- **Lokasi**: Skill registry
- **Tujuan**: Arsitektur input hook tingkat rendah (`WH_MOUSE_LL` / `WH_KEYBOARD_LL`), message loop worker terisolasi, supresi input instan (`LRESULT(1)`), proteksi rekursi sintetis (`LLMHF_INJECTED`), mitigasi `LowLevelHooksTimeout`, serta kalkulasi multi-monitor geometry dan canvas relative offset.
- **Implementasi pada omniRing**:
  - Diterapkan pada Langkah 3 (Hook Worker), Langkah 4 (Gesture State Machine), dan Langkah 5 (Screen Boundary Clamping).

### D. `antislop` & `antislop-code`
- **Tujuan**: Menjaga kebersihan kode dari komentar AI generik (AI slop) dan boilerplate yang tidak perlu.
- **Implementasi pada omniRing**:
  - Menghilangkan komentar naratif redundan.
  - Memastikan kode ringkas, fungsional, dan langsung pada tujuannya (YAGNI).

### E. `superpowers:executing-plans` & `verification-before-completion`
- **Tujuan**: Eksekusi rencana implementasi langkah demi langkah dengan validasi berbasis bukti sebelum menyatakan selesai.
- **Implementasi pada omniRing**:
  - Setiap langkah diuji dan diverifikasi kesesuaian manifest dan alur logikanya.
  - Menyiapkan workflow GitHub Actions CI untuk verifikasi build cloud yang deterministik.
