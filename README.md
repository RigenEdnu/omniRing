# omniRing

omniRing adalah utilitas *radial menu* dan gestur mouse modern berkecepatan tinggi untuk Windows 10 dan Windows 11 (64-bit). Terinspirasi oleh konsep Logitech Options+ dan RadialMenu, omniRing menghadirkan antarmuka visual Bubble Nodes melingkar, gestur usap (*flick*) instan, antarmuka Desktop Configurator dengan Action Library lengkap, serta dukungan multi-mode trigger yang otomatis menyesuaikan konteks aplikasi aktif di layar pengguna tanpa mencuri fokus keyboard (*zero focus stealing*).

---

## Fitur Utama

- **Zero-Latency Non-Activating Overlay**: Window overlay transparan menggunakan flag Win32 `WS_EX_NOACTIVATE | WS_EX_TOPMOST`. Shortcut keyboard langsung dikirim ke aplikasi aktif tanpa jeda aktivasi jendela.
- **Bubble Nodes UI (Logitech Options+ Style)**: Render berbasis HTML5 Canvas 2D dengan hub pembatalan tengah (simbol '×', r = 22px), 8 circular action bubble nodes gelap (r = 24px) pada orbital radius 95px dengan ikon vektor resolusi tinggi, serta outer floating pill badges pada orbital radius 175px dengan efek hover glow interaktif.
- **Dedicated Desktop Configurator & Action Library**: Jendela aplikasi desktop khusus dengan strip pemilih profil, preview interaktif 8-slot wheel, sidebar Action Library dengan pencarian instan dan 4 kategori terorganisir (System Actions, Device Navigation, Shortcuts, App Launchers), serta dock kontrol Undo, Redo, Clear Slot, dan Reset Slices.
- **Advanced Multi-Mode Trigger Engine**:
  - *Hold Mode*: Menahan tombol mouse (default Button 4 / X1) lebih dari 150ms.
  - *Combo Mode*: Menekan kombinasi tombol modifier keyboard (Ctrl, Shift, Alt, Win) bersamaan dengan klik mouse untuk membuka menu radial secara instan tanpa jeda hold timer.
  - *Corner Hover Mode*: Menggerakkan kursor ke sudut layar monitor untuk memicu aksi atau menu.
- **Fast-Path Directional Flick**: Geser cepat kursor (> 40px dalam < 150ms) saat menekan tombol pemicu untuk mengeksekusi aksi 4 arah (Atas, Kanan, Bawah, Kiri) secara instan tanpa perlu menunggu menu visual muncul.
- **Instant Dismissal & Auto-Close**:
  - *Esc Suppression*: Tekan tombol Escape saat radial menu aktif untuk menutup menu seketika; tombol Esc disupresi bersih sehingga tidak membatalkan pekerjaan pada aplikasi aktif di bawahnya.
  - *Outside-Click Auto-Close*: Klik di luar batas menu radial menutup menu secara instan dan klik diteruskan langsung ke sistem.
  - *Center Dismiss Hub*: Arahkan kursor ke tombol silang '×' di pusat lingkaran (< 26px) untuk membatalkan menu.
- **Multi-Monitor Boundary Clamping**: Menu mendeteksi batas monitor aktif tempat kursor berada dan otomatis bergeser jika kursor berada di tepi layar agar menu dan label mengambang tidak terpotong.
- **Dynamic Per-App Profiles & Submenus**: Menggunakan Win32 foreground hook untuk mendeteksi aplikasi aktif (seperti `code.exe`, `notepad.exe`) dan otomatis memuat konfigurasi profil yang sesuai, lengkap dengan dukungan cincin bertingkat (*child rings*).
- **Mesin Eksekusi Aksi Lengkap**:
  - `Shortcut`: Injeksi kombinasi tombol via Win32 `SendInput` (mendukung Ctrl, Shift, Alt, Win, tombol fungsi, media keys, dan extended keys).
  - `TypeText`: Injeksi teks langsung via UTF-16 Unicode (`KEYEVENTF_UNICODE`).
  - `OpenApp`: Menjalankan aplikasi biner atau perintah sistem.
  - `OpenUrl`: Membuka link web menggunakan browser default sistem.
  - `Sequence`: Menjalankan rantai aksi berurutan dengan jeda (*delay*).
- **System Tray Management**: Ikon pada area notifikasi Windows untuk membuka jendela konfigurasi, memuat ulang profil, membuka folder konfigurasi, mengaktifkan/menonaktifkan fungsi hook, atau keluar dari aplikasi.

---

## Persyaratan Sistem

- **Sistem Operasi**: Windows 10 x64 atau Windows 11 x64.
- **Komponen Pendukung**: Microsoft Edge WebView2 Runtime (sudah terpasang bawaan pada Windows 11 dan versi modern Windows 10).
- **Hardware**: Mouse dengan tombol samping (Mouse Button 4 / X1 / Back button sebagai tombol pemicu default).

---

## Panduan Penggunaan

### 1. Menjalankan Aplikasi

1. Ekstrak arsip rilis `omniring-windows-x64.zip`.
2. Letakkan folder `profiles` di samping file `omniring.exe`.
3. Jalankan `omniring.exe`.
4. Ikon omniRing akan muncul di area System Tray (pojok kanan bawah taskbar Windows).

```text
Folder Aplikasi:
├── omniring.exe
└── profiles/
    ├── default.json
    ├── code.json
    └── notepad.json
```

### 2. Mengoperasikan Tombol Pemicu & Mode Trigger

Aplikasi mendukung tiga mode pemicu input yang fleksibel:

#### A. Mode Hold (Default)
Menggunakan tombol samping mouse (X1 / Button 4):

| Aksi Pengguna | Durasi / Jarak | Respons Aplikasi |
|---|---|---|
| **Tap Biasa** | Lepas tombol < 150ms dan pergeseran mouse <= 10px | Diteruskan ke sistem sebagai klik normal Mouse Button 4 (tombol Back). |
| **Flick Cepat** | Geser mouse > 40px sebelum 150ms | Membatalkan kemunculan radial menu dan langsung memicu gestur terarah (Up, Down, Left, Right). |
| **Tahan (Hold)** | Tahan tombol > 150ms | Memunculkan Radial Menu Bubble Nodes tepat di sekitar kursor mouse. |

#### B. Mode Combo (Keyboard + Mouse)
- Contoh konfigurasi: `Ctrl + Right Click`.
- Menekan tombol modifier (Ctrl) lalu menekan tombol mouse (Right Click) akan membuka menu radial seketika tanpa perlu menunggu jeda hold timer.
- Klik kanan biasa tanpa tombol Ctrl tetap berfungsi normal membuka context menu sistem.

#### C. Mode Corner Hover
- Mengarahkan dan mendiamkan kursor di sudut layar monitor (misal pojok kiri atas atau kanan atas) selama durasi dwell (misal 200ms) untuk membuka menu.

### 3. Berinteraksi dengan Bubble Nodes Radial Menu

1. **Arahkan Kursor**: Saat menu terbuka, gerakkan kursor mouse ke arah node bubble yang ingin dipilih. Node aktif akan membesar 1.12x dan menyala dengan aksen pendaran (*glow*), sementara pill badge beralih ke kontras tinggi.
2. **Eksekusi Aksi**: Lepaskan tombol pemicu saat kursor berada di atas node target. Overlay akan otomatis tertutup dan aksi langsung dieksekusi ke aplikasi target.
3. **Membatalkan Menu**:
   - *Via Center Hub*: Arahkan kursor kembali ke tombol silang tengah ('×', radius < 26px) lalu lepaskan tombol pemicu.
   - *Via Tombol Escape*: Tekan tombol `Esc` pada keyboard untuk membatalkan menu seketika tanpa mempengaruhi aplikasi latar belakang.
   - *Via Outside Click*: Klik di luar area menu radial untuk langsung menutup overlay dan meneruskan klik ke jendela di bawahnya.
4. **Navigasi Submenu**:
   - Jika irisan bertipe `Submenu` dipilih, cincin akan berganti ke irisan child ring tanpa menutup jendela.
   - Untuk kembali ke cincin utama, pilih node `Back` atau klik lingkaran tengah (*hub*).

### 4. Menggunakan Desktop Configurator & Action Library

omniRing dilengkapi dengan window konfigurator visual interaktif untuk mengelola profil dan penugasan aksi:

1. **Membuka Configurator**:
   - Klik kanan ikon omniRing di System Tray dan pilih **Configure OmniRing**, atau
   - Lakukan klik ganda (*double-click*) pada ikon omniRing di System Tray.
2. **Mengelola Profil Aplikasi**:
   - Pilih tab profil pada bar navigasi atas (`Default Global`, `VS Code`, `Notepad`).
   - Klik tab **+ Add Profile** untuk menambahkan profil khusus aplikasi baru berdasarkan nama proses target (misal `blender.exe`).
3. **Mengedit Irisan pada Preview Wheel**:
   - Klik salah satu dari 8 slot bubble pada preview radial wheel di sebelah kiri. Slot aktif akan dilingkari oleh pendaran seleksi ungu (`#5E48E8`).
   - Gunakan dock kontrol di bawah wheel:
     - **Undo**: Membatalkan perubahan aksi slot sebelumnya.
     - **Redo**: Mengulangi perubahan aksi slot berikutnya.
     - **Clear Slot**: Mengosongkan aksi pada slot yang sedang dipilih.
     - **Reset Slices**: Mengembalikan seluruh susunan slot ke preset bawaan profil.
4. **Memilih Aksi dari Action Library**:
   - Gunakan kolom pencarian di sebelah kanan untuk mencari aksi secara cepat.
   - Telusuri 4 kategori accordion:
     - `System Actions`: Aksi sistem operasi (Lock Screen, Volume Mute, Play/Pause, Screenshot, dsb).
     - `Device Navigation & Folders`: Navigasi jendela dan folder (Explorer, Switch Window, Back, Desktop, dsb).
     - `Keyboard Shortcuts`: Pintasan keyboard umum (Copy, Paste, Undo, Redo, Custom).
     - `App Launchers & URLs`: Pembuka aplikasi dan link web (Terminal, Browser, Custom App, Web URL).
   - Klik kartu aksi yang diinginkan untuk langsung menugaskannya ke slot terpilih pada preview wheel.
5. **Mengatur Mode Pemicu (Trigger Settings)**:
   - Klik tombol **Trigger Settings** di header untuk beralih ke panel pengaturan mode input.
   - Pilih mode `Hold`, `Combo`, atau `Corner Hover` dan sesuaikan parameter durasi atau tombol pemicu.
6. **Penyimpanan Otomatis**:
   - Setiap modifikasi langsung disimpan secara otomatis ke disk JSON dan disinkronkan ke live overlay runtime secara instan.

### 5. Kontrol System Tray

Klik kanan pada ikon omniRing di System Tray untuk mengakses opsi berikut:

- **Configure OmniRing**: Menampilkan dan memfokuskan jendela Desktop Configurator.
- **Open Config Folder**: Membuka direktori `profiles` di Windows File Explorer untuk mempermudah backup atau kustomisasi manual file JSON.
- **Reload Profiles**: Membaca ulang seluruh berkas JSON konfigurasi profil dari disk dan menyinkronkannya dengan jendela aplikasi aktif.
- **Toggle Enable/Disable**: Centang atau hapus centang untuk mengaktifkan atau menonaktifkan hook input omniRing secara instan tanpa perlu mematikan aplikasi.
- **Exit**: Mematikan aplikasi dan melepaskan seluruh hook Win32 secara bersih.

---

## Konfigurasi Profil (JSON)

Setiap profil mendefinisikan daftar proses target dan kumpulan irisan (*slices*). File disimpan dalam format JSON di dalam folder `profiles/`.

### Struktur Dasar Profil

```json
{
  "profile_name": "Visual Studio Code",
  "process_targets": ["code.exe"],
  "inner_radius": 22,
  "outer_radius": 210,
  "slices": [
    {
      "id": "slice_0",
      "label": "Format Code",
      "icon": "code",
      "action": {
        "type": "Shortcut",
        "keys": ["Shift", "Alt", "F"]
      }
    },
    {
      "id": "slice_1",
      "label": "Git Tools",
      "icon": "folder",
      "action": {
        "type": "Submenu",
        "target_ring_id": "git_tools"
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
      "label": "Quick Search",
      "icon": "search",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "Shift", "F"]
      }
    },
    {
      "id": "slice_4",
      "label": "New File",
      "icon": "new-note",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "N"]
      }
    },
    {
      "id": "slice_5",
      "label": "Command Palette",
      "icon": "options",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "Shift", "P"]
      }
    },
    {
      "id": "slice_6",
      "label": "Copy",
      "icon": "copy",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "C"]
      }
    },
    {
      "id": "slice_7",
      "label": "Paste",
      "icon": "paste",
      "action": {
        "type": "Shortcut",
        "keys": ["Control", "V"]
      }
    }
  ],
  "submenus": {
    "git_tools": {
      "title": "Git Tools",
      "slices": [
        {
          "id": "git_status",
          "label": "Status Cmd",
          "icon": "terminal",
          "action": {
            "type": "TypeText",
            "text": "git status\n"
          }
        },
        {
          "id": "git_back",
          "label": "Back",
          "icon": "undo",
          "action": {
            "type": "Submenu",
            "target_ring_id": "root"
          }
        }
      ]
    }
  }
}
```

### Format Tipe Aksi

#### 1. Shortcut
Mengirimkan kombinasi penekanan tombol keyboard.
```json
"action": {
  "type": "Shortcut",
  "keys": ["Control", "Shift", "P"]
}
```
*Tombol yang didukung*: `Control`, `Shift`, `Alt`, `Win` / `Super`, huruf (`a`-`z`), angka (`0`-`9`), tombol fungsi (`F1`-`F24`), tombol navigasi (`Enter`, `Escape`, `Tab`, `Space`, `Backspace`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`), media keys, dsb.

#### 2. TypeText
Menginjeksi rentetan teks karakter Unicode ke aplikasi aktif.
```json
"action": {
  "type": "TypeText",
  "text": "console.log('hello world');\n"
}
```

#### 3. OpenUrl
Membuka alamat tautan web di browser default.
```json
"action": {
  "type": "OpenUrl",
  "url": "https://github.com"
}
```

#### 4. OpenApp
Menjalankan executable atau perintah sistem.
```json
"action": {
  "type": "OpenApp",
  "path": "calc.exe",
  "args": []
}
```

#### 5. Submenu
Membuka cincin menu cabang (*child ring*).
```json
"action": {
  "type": "Submenu",
  "target_ring_id": "nama_submenu"
}
```
*Catatan*: Gunakan `"target_ring_id": "root"` untuk kembali ke cincin utama.

#### 6. Sequence
Menjalankan rantai aksi dengan jeda milidetik (*delay*) terukur.
```json
"action": {
  "type": "Sequence",
  "actions": [
    { "type": "Shortcut", "keys": ["Control", "a"] },
    { "type": "Shortcut", "keys": ["Control", "c"] }
  ],
  "delay_ms": 50
}
```

---

## Profil Default Bawaan

1. **`default.json`**: Profil global untuk seluruh aplikasi (`process_targets: ["*"]`). Berisi 8 slot Bubble Nodes: Play/Pause, New Note, Copy, Paste, Undo, Redo, Terminal, dan Quick Search.
2. **`code.json`**: Profil khusus editor VS Code (`code.exe`). Berisi aksi Format Code, Git Tools (Submenu), Toggle Terminal, Quick Search, New File, Command Palette, Copy, dan Paste.
3. **`notepad.json`**: Profil khusus Notepad (`notepad.exe`). Berisi aksi New File (`Ctrl+N`), Save (`Ctrl+S`), Find (`Ctrl+F`), Select All (`Ctrl+A`), Copy, Paste, Undo, dan Redo.

---

## FAQ & Tips Pemecahan Masalah

- **Menu tidak muncul saat menekan tombol samping**: Pastikan mouse Anda memiliki tombol samping (X1 / Button 4). Tahan tombol setidaknya selama 150 milidetik tanpa menggeser mouse terlalu jauh.
- **Mengubah tombol pemicu**: Anda dapat mengatur mode pemicu ke `Hold`, `Combo`, atau `Corner Hover` secara visual melalui jendela **Configure OmniRing** pada tab Trigger Settings.
- **Shortcut tidak tembus ke aplikasi**: Jika aplikasi target dijalankan sebagai Administrator (Elevated), jalankan juga `omniring.exe` sebagai Administrator agar injeksi `SendInput` dari Windows UIPI (User Interface Privilege Isolation) tidak diblokir oleh Windows.
- **Menutup radial menu secara cepat**: Anda dapat menekan tombol `Esc` kapan saja atau melakukan klik di luar menu untuk menutup overlay seketika.
