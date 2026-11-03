# Technology Stack: omniRing

## 1. Core Architecture Overview
omniRing menggunakan arsitektur hybrid performa tinggi: core logika sistem dan hook tingkat rendah diimplementasikan menggunakan Rust, sedangkan antarmuka overlay radial dirender menggunakan WebView2 melalui framework Tauri v2.

## 2. Rincian Lapisan Teknologi

| Lapisan | Teknologi Terpilih | Justifikasi & Rationale |
| :--- | :--- | :--- |
| **Aplikasi & Shell** | Tauri v2 | Footprint distribusi kecil (<15MB), konsumsi RAM idle rendah (~30-45MB), dan integrasi event bridge Rust ke Webview yang aman dan teruji. |
| **Bahasa Backend** | Rust (Edition 2021) | Mematuhi `rust-best-practices` (Apollo Handbook) dan `rust-async-patterns` (Tokio, channel non-blocking, zero unwrap di produksi). |
| **Win32 API Bindings** | `windows` crate (Microsoft official) | Akses langsung ke API `WH_MOUSE_LL`, `WH_KEYBOARD_LL`, `SetWinEventHook`, `SendInput`, `MonitorFromPoint`, dan manipulasi style window `WS_EX_NOACTIVATE`. |
| **Concurrency / IPC** | `crossbeam-channel` / `tokio` | Antrian komunikasi non-blocking antara thread callback hook Windows dan thread utama aplikasi untuk mencegah hook timeout oleh OS. |
| **Serialisasi Data** | `serde` + `serde_json` | Penyimpanan konfigurasi profil aplikasi dalam format JSON lokal yang mudah dibaca dan diedit pengguna. |
| **Antarmuka Overlay** | HTML5 Canvas / SVG + Vanilla TypeScript | Menghindari framework JavaScript berat (React/Angular). Rendering murni Canvas/SVG memberikan 60-140 FPS stabil dengan latensi render < 8ms. |
| **Iconografi** | SVG Vector Icons + Native Shell Extraction | Format SVG untuk icon sistem standar, serta Win32 `SHGetFileInfoW` / `ExtractIconExW` di sisi Rust untuk mengekstrak icon aplikasi target. |
| **System Tray** | Tauri v2 Tray Icon Manager | Manajemen icon di taskbar Windows untuk membuka konfigurasi, memilih profil aktif secara manual, atau keluar dari aplikasi. |

## 3. Komparasi Terhadap Stack Referensi

| Aspek | RadialMenu (Flutter x64) | Logi Options+ (Daemon+Electron) | omniRing (Rust + Tauri v2) |
| :--- | :--- | :--- | :--- |
| **Ukuran Distribusi** | ~75 MB (MSIX) / 184 MB (Unpacked) | ~300+ MB (Full Suite) | **< 15 MB** (Installer / Portable) |
| **Konsumsi RAM Idle** | 80 - 150 MB | 150 - 400 MB (multi-proses) | **< 45 MB** |
| **Overhead CPU Idle** | 0.5% - 2% | 1% - 3% (background agent) | **0.0%** (Event-driven pure sleep) |
| **Dukungan Hardware** | Software murni | Vendor-locked (Logitech saja) | **Universal (Semua mouse/keyboard)** |
| **Ketergantungan Akun** | Ada (Firebase Auth/Firestore) | Wajib (Logitech ID) | **100% Offline & Lokal** |

## 4. Kebutuhan Toolchain & Strategi Build
- **Lingkungan Lokal**: Didedikasikan untuk full coding, perancangan arsitektur, dan penulisan source code tanpa ketergantungan toolchain kompilator lokal.
- **Lingkungan Build & Verifikasi**: GitHub Actions CI (`windows-latest`).
  - Rust Toolchain: `stable-x86_64-pc-windows-msvc`
  - Node.js & Package Manager: Node.js v20+ / pnpm
  - Build Pipeline: GitHub Actions Workflow (`.github/workflows/build.yml`) yang menjalankan pemeriksaan sintaksis (`cargo check`), unit test, dan kompilasi bundle rilis Tauri v2 (`cargo tauri build`).
  - Output Artifact: Standalone executable atau installer ZIP yang dapat diunduh langsung dari GitHub Actions Release/Artifacts.
