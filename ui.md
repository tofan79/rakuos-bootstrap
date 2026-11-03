# Tampilan RakuOS Bootstrap

Panduan ini menjelaskan tampilan terminal yang digambar oleh `src/ui.rs` dan
alur setup di `src/main.rs`. Wizard berjalan di terminal teks, termasuk di
virtual console Linux. Tidak perlu GTK, Qt, atau mouse.

## Desain tampilan

Antarmuka memakai aksen cyan untuk merek RakuOS, progres langkah yang jelas,
baris pilihan yang disorot, deskripsi singkat, dan baris bantuan keyboard yang
konsisten. Warna menyesuaikan kemampuan terminal dan `NO_COLOR`; UI kembali ke
teks polos jika warna tidak tersedia atau output bukan terminal interaktif.

| Mode | Kapan dipakai | Kotak dan penanda |
|---|---|---|
| Full | Emulator terminal UTF-8 | `╭─╮`, `✔ ● ○ ❯` |
| TTY | `TERM=linux` dengan locale UTF-8 | `┌─┐`, `[x] [>] [ ]` |
| ASCII | Locale bukan UTF-8, `--ascii`, `RAKUOS_ASCII`, atau `TERM=dumb` | `+-+`, `[x] [>] [ ]` |

Terminal memakai cyan terang bila didukung. Di virtual console Linux dipakai
warna ANSI cyan terang; di terminal lain dipakai cyan 256 warna bila tersedia.

## Kontrol keyboard

- `↑` / `↓`, `j` / `k`, atau `Tab`: memindahkan sorotan.
- Daftar multi-pilih: `Space` mencentang atau melepas pilihan.
- `Enter`: lanjut bila pilihan sudah valid.
- `Esc`, `Backspace`, `←`, atau `b`: kembali satu layar.
- `Ctrl-C`: keluar dari wizard. Di layar pertama, tombol kembali hanya
  menampilkan petunjuk untuk memakai `Ctrl-C`.

Terminal, File Manager, dan Browser boleh memilih lebih dari satu dan wajib
memilih minimal satu. Desktop dan Login Manager hanya boleh satu pilihan.
Layar Login Manager muncul hanya jika desktop yang dipilih punya
`choose_login = true`. Di layar ringkasan, `Enter` atau `y` melanjutkan,
`n` membatalkan, dan tombol kembali mengembalikan ke layar pilihan terakhir
yang benar-benar ditampilkan.

## Alur setup

```text
Desktop → Terminal → File Manager → Browser → [Login Manager bila perlu]
                                                    ↓
                                         Review your setup
                                                    ↓
                              Install, configure, and verify
```

Layar login dilewati bila desktop sudah membawa login manager sendiri
(misalnya Budgie dan LXQt) atau katalog menetapkan `login_override`. Di terminal dengan
lebar di bawah 72 kolom, stepper hanya menampilkan nama langkah aktif.

KineticWE mengaktifkan COPR `theblackdon/kwee` untuk paket desktopnya dan tidak
mengaktifkan `mindset/Mindset-Apps`. Terra dan RPM Fusion tetap bisa
diaktifkan untuk aplikasi lain yang dipilih.

## Contoh layar

Contoh memakai katalog saat ini. Pilihan dan deskripsi berasal dari
`data/catalog.toml` dan bisa berubah.

### Pilih terminal

```text
 ╭────────────────────────────────────────────────────────────────────────────╮
 │ ◆ RakuOS Setup                                                      v0.5.0 │
 │ Post-install setup: choose your desktop and apps                  DRY-RUN  │
 ╰────────────────────────────────────────────────────────────────────────────╯

 ✔ Desktop ── ● Terminal ── ○ File Manager ── ○ Browser ── ○ Login

 Terminal                                                         Step 2 of 5
 Where you type commands. Pick one or more
 ────────────────────────────────────────────────────────────────────────────
 Options                      │ Details
 ─────────────────────────────┼─────────────────────────────────────────────
   □ Alacritty                │ Kitty
   □ Foot                     │ ✔ Selected  ·  Default
   □ Ghostty                  │
 ❯ ■ Kitty                    │ GPU-accelerated and commonly used with
                              │ Hyprland
                              │
                              │ Packages (4)
 1 selected  ·  So far: Hyprland

 ↑↓ Move  Space Toggle  Enter Continue  Esc/← Back  Ctrl-C Quit
```

`default = true` di katalog menentukan pilihan multi-pilih yang tercentang saat
layar dibuka. Pilihan diurutkan A-Z. Centang tetap tersimpan saat kembali ke
langkah sebelumnya. Terminal lebar menampilkan daftar pilihan di samping detail
paket yang disorot; terminal sempit memakai daftar ringkas.

### Pilih desktop

```text
 ╭────────────────────────────────────────────────────────────────────────────╮
 │ ◆ RakuOS Setup                                                      v0.5.0 │
 │ Post-install setup: choose your desktop and apps                  DRY-RUN  │
 ╰────────────────────────────────────────────────────────────────────────────╯

 ● Desktop ── ○ Terminal ── ○ File Manager ── ○ Browser ── ○ Login

 Desktop                                                          Step 1 of 5
 Choose your Wayland desktop
 ────────────────────────────────────────────────────────────────────────────
 Options  ↓1                  │ Details
 ─────────────────────────────┼─────────────────────────────────────────────
   ○ Budgie                   │ Hyprland
 ❯ ● Hyprland                 │
   ○ KineticWE                │ Dynamic-tiling compositor with smooth
   ○ LXQt                     │ animations; no panel, highly customizable
   ○ MangoWM                  │
   ○ MiracleWM                │
   ○ Sway                     │ Packages (5)
                              │ hyprland, hyprland-guiutils, … +3 more

 ↑↓ Move  Enter Continue  Esc/← Back  Ctrl-C Quit
```

### Review your setup

Ringkasan menampilkan perangkat lunak yang dipilih, metode login, repositori,
target user, dan jumlah paket RPM yang akan dipasang. Nama paket dibungkus
maksimal dua baris.

```text
 ✔ Desktop ── ✔ Terminal ── ✔ File Manager ── ✔ Browser ── ✔ Login

 Review your setup
 ────────────────────────────────────────────────────────────────────────────
 Desktop                Budgie
 Terminal               Kitty
 File Manager           Nautilus
 Browser                Firefox
 Login                  built-in for Budgie
 Repositories           terra, rpmfusion, mindset
 Target user            mindset
 RPM packages (…)
   …
 ? Continue with installation?

 Enter/Y Continue  Esc/← Back  N Cancel  Ctrl-C Quit
```

## Instalasi dan penyelesaian

Setelah konfirmasi, layar pilihan digantikan header cyan RakuOS dan keluaran
perintah secara langsung. Perintah beserta keluarannya ditulis ke log. Di mode
`--dry-run`, perintah yang direncanakan diawali `+`; tidak ada paket yang
dipasang dan tidak ada file yang ditulis.

Setelah verifikasi, proses normal menanyakan apakah akan reboot (`Y/n`). Mode
`--first-boot` melakukan reboot otomatis setelah verifikasi berhasil. Jika
verifikasi gagal, reboot otomatis dilewati dan lokasi log ditampilkan.

Lebar layar dibatasi 78 kolom dan menyesuaikan sampai 40 kolom. Di terminal
lebar, deskripsi dibungkus di panel detail dan daftar pilihan panjang bisa
digulir sesuai tinggi yang tersedia. Terminal sempit memakai daftar ringkas
dengan deskripsi yang dipotong. Kursor disembunyikan selama wizard interaktif
dan dikembalikan saat keluar. UI tidak memakai alternate screen, jadi keluaran
akhir tetap tersimpan di scrollback terminal.
