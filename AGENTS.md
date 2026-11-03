# Panduan Agent: omniRing

## PERINGATAN KERAS LINGKUNGAN HOST (CRITICAL ENVIRONMENT CONSTRAINT)
- Host lokal Windows TIDAK memiliki compiler Rust (`cargo`, `rustc`, `rustup`) maupun MSVC C++ Build Tools.
- DILARANG KERAS menjalankan perintah: `cargo`, `rustc`, `where cargo`, `where rustc`, `cargo check`, `cargo build`, maupun memindai drive lokal (`dir /s /b C:\cargo.exe`, dll) untuk mencari binary Rust.
- Validasi lokal HANYA dilakukan melalui inspeksi statis kode (code review, validasi tipe, sintaksis, schema JSON, dan struktur file).
- Seluruh verifikasi kompilasi dan build biner 100% didelegasikan ke GitHub Actions CI (`.github/workflows/build.yml`) menggunakan runner `windows-latest`.

## Aturan Memori & Vibe Coding Workflow

- Sebelum menulis atau mengubah kode apapun, wajib membaca secara lengkap:
  - `memory-bank/architecture.md`
  - `memory-bank/design-document.md`
  - `memory-bank/implementation-plan.md`
  - `memory-bank/progress.md`
- Eksekusi rencana implementasi harus dilakukan tepat 1 langkah dalam satu waktu (`implementation-plan.md`). Jangan melangkah ke langkah berikutnya sebelum langkah saat ini teruji dan terverifikasi.
- Setiap kali menyelesaikan fitur atau milestone penting, wajib memperbarui `memory-bank/progress.md` dan `memory-bank/architecture.md`.

## Catatan Aturan Penting

- 写任何代码前必须完整阅读 memory-bank/architecture.md
- 写任何代码前必须完整阅读 memory-bank/design-document.md
- 每完成一个重大功能或里程碑后，必须更新 memory-bank/architecture.md

## Konvensi Kode & Lingkungan
- **Platform**: Windows 11 / Windows 10 x64.
- **Backend**: Rust 2021 edition (Tauri v2 core).
- **Frontend**: Vanilla TypeScript / HTML5 Canvas / SVG (ringan, performa render tinggi, tanpa framework JS berat).
- **Gaya Komunikasi**: Langsung, pragmatis, berbasis bukti. Jangan gunakan tanda baca em dash (—).
