# rakuos-bootstrap

Wizard TUI pasca-install untuk RakuOS minimal. Wizard membantu memilih aplikasi
dasar dan desktop, memasang paket ke overlay `rum`, menyiapkan login manager,
serta menyalin konfigurasi yang tersedia. Ini bukan installer OS;
`rakuos-installer-cli` yang memasang sistem ke disk.

- Katalog pilihan: [`data/catalog.toml`](data/catalog.toml)
- Panduan tampilan TUI: [`ui.md`](ui.md)
- Binary Rust: `rakuos-bootstrap`; tidak memerlukan GTK/Qt untuk UI

## Menjalankan

```bash
cargo build --release

# Tampilkan rencana tanpa mengubah sistem
./target/release/rakuos-bootstrap --dry-run

# Jalankan instalasi (butuh root)
sudo ./target/release/rakuos-bootstrap
```

Opsi yang tersedia:

| Opsi | Fungsi |
|---|---|
| `--dry-run` | Tampilkan tindakan tanpa menjalankan perintah atau menulis file |
| `--first-boot` | Mode unit systemd; reboot otomatis setelah instalasi dan verifikasi berhasil |
| `--ascii` | Paksa glyph ASCII; juga bisa memakai `RAKUOS_ASCII=1` |
| `--catalog <PATH>` | Baca katalog TOML alternatif |
| `--base-dir <PATH>` | Lokasi data katalog/skel/sesi; default `/usr/share/rakuos/bootstrap` |
| `--log <PATH>` | Lokasi log; default `/var/log/rakuos-bootstrap.log` |
| `--help`, `--version` | Tampilkan bantuan atau versi |

Wizard perlu TTY interaktif. Instalasi sungguhan perlu dijalankan sebagai root.
Log tidak dibuat saat `--dry-run`.

## Alur

Urutan layar adalah Terminal, File manager, Browser, Desktop, lalu Login manager
bila desktop yang dipilih memiliki `choose_login = true`. Pilihan untuk tiga
kategori pertama bisa jamak; desktop dan login manager tunggal. Desktop yang
tidak meminta login memakai display manager yang dibawa desktopnya.

`↑`/`↓` atau `j`/`k` memindahkan sorotan; `Tab` juga bergerak ke bawah.
`Space` atau tombol `1`–`9` mencentang pilihan pada kategori jamak. Pada daftar
tunggal, `1`–`9` memilih item. `Enter` melanjutkan. `Esc`, `Backspace`, `←`,
atau `b` kembali satu layar; `Ctrl-C` membatalkan wizard. Pada ringkasan,
`n` membatalkan tanpa instalasi, sedangkan `Esc` kembali ke pilihan terakhir.

Setelah konfirmasi, script mengaktifkan repo yang tercantum di katalog,
menyegarkan metadata, memasang paket (`rum`, atau `dnf` bila `rum` tidak ada),
menyiapkan file sesi/skel yang tersedia, mengatur login manager, menulis state,
dan memverifikasi hasilnya. Jika verifikasi gagal, reboot ditunda dan kegagalan
dicatat di log.

## Pilihan dalam katalog saat ini

Daftar di layar diurutkan alfabetis oleh program. Katalog adalah sumber nama,
deskripsi, paket, repo, dan nilai default; tabel berikut hanya merangkum nama
agar dokumentasi tidak menyalin metadata yang bisa berubah.

| Kategori | Pilihan | Default |
|---|---|---|
| Terminal | Alacritty, Foot, Ghostty, Kitty | Kitty |
| File manager | Dolphin, HyprFM, Nautilus, Nemo, Superfile, Thunar, Yazi | Nautilus |
| Browser | Brave, Chromium, Epiphany, Falkon, Firefox, Google Chrome, Helium, LibreWolf, Microsoft Edge, QuteBrowser, Vivaldi, Waterfox, Zen Browser | Firefox |
| Desktop | Budgie, Hyprland, KineticWE, LXQt, MangoWM, MiracleWM, Sway, Umbriel | Budgie |
| Login manager | DMS Greeter, Noctalia Greeter, SDDM, sysc-greet | SDDM |

Login manager hanya ditanyakan untuk Budgie, Hyprland, MangoWM, dan Sway; SDDM
menjadi pilihan awal. Kombinasi awalnya adalah Budgie + SDDM, dengan sesi
Budgie Wayland. MiracleWM memakai grup desktop Fedora dan katalog secara
otomatis mengaktifkan SDDM Wayland dari grup tersebut. LXQt memakai display
manager yang ikut terpasang bersama desktop. KineticWE dan Umbriel memakai
login yang ditetapkan katalog.

Repo yang diminta oleh katalog saat ini adalah Terra, RPM Fusion, dan COPR
`mindset/Mindset-Apps`. Jika `rum` tidak ditemukan, pengaktifan repo dilewati
dan pemasangan paket menggunakan `dnf`.

## File yang dipasang atau ditulis

- Paket dasar yang selalu diminta: `dbus-tools`, `egl-wayland`, `qt6-qtsvg`,
  `qt6-qtwayland`, `rakuos-software-qt`, `rakuos-system-qt`, dan
  `rakuos-welcome-qt`.
- `data/sessions/hyprland-uwsm.desktop` menggantikan file sesi Hyprland bila
  desktop Hyprland dipilih.
- Skel yang tersedia di repo hanya untuk MangoWM (`data/skel/mango`) dan
  Umbriel (`data/skel/umbriel`). Placeholder terminal, browser, dan file
  manager di skel diganti memakai pilihan pertama yang terurut alfabetis;
  fallback-nya `foot`, `firefox`, dan `nautilus`.
- File skel disalin ke home user target dan dimiliki user tersebut. File lama
  yang berbeda dicadangkan dengan akhiran `.bak` (atau `.bak.1`, dan seterusnya).
- Sesi Wayland yang diminta desktop tetapi belum disediakan paket dapat dibuat
  dari `autologin_cmd` di `/usr/share/wayland-sessions`.
- Konfigurasi SDDM ditulis saat SDDM dipilih. Drop-in sesi memuat direktori
  sesi Wayland dan X11; drop-in tema hanya ditulis jika `sddm_theme` diisi.
- `/var/lib/rakuos/bootstrap.toml` menyimpan pilihan terakhir dan
  `/var/lib/rakuos/bootstrap.done` menjadi penanda selesai.
- `/var/log/rakuos-bootstrap.log` menyimpan perintah, keluaran, peringatan,
  serta hasil verifikasi.

Target user diambil dari `SUDO_USER`; jika tidak ada, script mencari user
biasa pertama dengan home dan shell login, lalu memakai `root` sebagai fallback.

## Mengubah pilihan

Edit `data/catalog.toml`; tidak perlu mengubah kode untuk menambah entri
katalog yang mengikuti struktur `Choice`. `description` adalah teks yang
ditampilkan, `packages` adalah paket RPM, `default = true` menentukan pilihan
awal, dan `available = false` menampilkan pilihan sebagai belum tersedia.

Field khusus desktop dan login, termasuk `skel`, `session_file`,
`autologin_cmd`, `choose_login`, `login_override`, `unit`, `greetd_cmd`, dan
`greetd_setup`, dibaca oleh script. Lihat komentar dan contoh di
[`data/catalog.toml`](data/catalog.toml).

## Verifikasi

Sebelum reboot, script memeriksa paket yang diminta, unit display manager,
sesi Wayland untuk desktop yang memintanya, dan file skel jika desktop punya
skel. Pemeriksaan RPM juga melihat rpmdb overlay dari `rum paths` bila ada.
Jika ada pemeriksaan yang gagal, script menampilkan jumlah kegagalan dan tidak
melakukan reboot otomatis.
