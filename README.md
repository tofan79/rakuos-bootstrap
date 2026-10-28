# rakuos-bootstrap

Wizard pasca-install untuk RakuOS minimal: memilih terminal, file manager,
browser, desktop, dan login manager, lalu memasangnya di atas base image
(overlay `rum`), menyalin config minimal, dan menyiapkan sesi GUI.

Bukan installer OS. `rakuos-installer-cli` yang menulis sistem ke disk; program
ini yang membuat sistem minimal itu **bisa dipakai**.

- Mockup tampilan TUI: [`ui.md`](ui.md)
- Katalog pilihan: [`data/catalog.toml`](data/catalog.toml)
- Binary: `rakuos-bootstrap` (Rust, TUI tanpa dependensi GUI, aman di console VT)

## Status

Kode sudah berjalan dan diuji lewat pty (`--dry-run`, 5 fase, ringkasan,
mode ASCII, Esc → exit 130). Belum dipaketkan sebagai RPM; itu tahap
berikutnya di `Mindset-Apps`.

## Pakai

```bash
cargo build --release

# lihat rencana, tidak menyentuh sistem
./target/release/rakuos-bootstrap --dry-run

# instalasi sungguhan (butuh root)
sudo ./target/release/rakuos-bootstrap

# dipanggil unit systemd saat boot pertama
sudo rakuos-bootstrap --first-boot
```

| Opsi | Arti |
|---|---|
| `--dry-run` | tampilkan rencana, jangan ubah sistem |
| `--first-boot` | mode unit systemd; reboot otomatis setelah selesai |
| `--ascii` | paksa tampilan ASCII murni (juga `RAKUOS_ASCII=1`) |
| `--catalog <PATH>` | pakai katalog TOML lain |
| `--base-dir <PATH>` | lokasi katalog/skel (default `/usr/share/rakuos/bootstrap`) |

## Alur

```
1. Terminal       boleh lebih dari satu
2. File manager   boleh lebih dari satu
3. Browser        boleh lebih dari satu
4. Login manager  satu
5. Desktop        satu
        │
        ▼
   ringkasan → konfirmasi → enable repo → refresh → rum install
        │
        ├─ salin skel desktop ke ~/.config (chown ke user)
        ├─ enable greeter / siapkan autologin
        └─ tulis /var/lib/rakuos/bootstrap.toml + .done
```

Fase 1–3 memakai checkbox: `Space` mencentang, `1`–`9` mencentang nomor itu,
`Enter` maju kalau minimal satu tercentang. Fase 4–5 tunggal karena hanya satu
yang bisa aktif saat boot.

Urutan tampilan selalu A-Z (diurutkan oleh kode), tidak bergantung urutan di
`catalog.toml`.

## Pilihan

Deskripsi di bawah ini sama persis dengan yang tampil di layar; kalau mau
dikoreksi, ubah `description` di `data/catalog.toml` lalu sesuaikan tabelnya.
Ukuran = paket utamanya saja, bukan total dependency.

### Terminal

| Nama | Deskripsi | Paket | Repo | Ukuran | Default |
|---|---|---|---|---|---|
| Alacritty | GPU-accelerated, konfigurasi YAML sederhana | `alacritty` | Fedora | 7,8 MB | |
| Foot | Ringan, Wayland-only, cocok untuk sistem minimal | `foot`, `foot-terminfo` | Fedora | 0,9 MB | |
| Ghostty | Dari repo Terra, diaktifkan otomatis saat instalasi | `ghostty`, `ghostty-terminfo`, `ghostty-shell-integration` | Terra | 28,7 MB | |
| Kitty | GPU-accelerated, paling umum dipakai di Hyprland | `kitty`, `kitty-kitten`, `kitty-shell-integration`, `kitty-terminfo` | Fedora | 16 MB | ✅ |

`@TERMINAL@` dan `@BROWSER@` di config desktop memakai pilihan pertama
menurut abjad dari yang dicentang; `@FILEMANAGER@` memakai file manager yang
dipilih. Kalau kosong, fallback-nya `foot`, `firefox`, dan `nautilus`.

### File manager

| Nama | Deskripsi | Paket | Repo | Ukuran | Default |
|---|---|---|---|---|---|
| Dolphin | File manager KDE, fitur lengkap (menarik Qt/KDE stack) | `dolphin` | Fedora | 14,1 MB | |
| HyprFM | Keyboard-first untuk Hyprland, dari repo RakuOS | `hyprfm` | RakuOS | 2,7 MB | |
| Nautilus | File manager GNOME, ringan untuk pemakaian dasar | `nautilus` | Fedora | 14,8 MB | ✅ |
| Nemo | File manager Cinnamon, GTK3, dukungan gvfs-fuse | `nemo` | Fedora | 4,3 MB | |
| Superfile | File manager TUI modern (binary spf), dari repo Terra | `superfile` | Terra | 19,7 MB | |
| Thunar | File manager Xfce, ringan dan cepat | `Thunar`, `thunar-volman` | Fedora | 11,3 MB | |
| Yazi | File manager TUI cepat, dari repo Terra | `yazi` | Terra | 33,3 MB | |

Catatan: Superfile dan Yazi berjalan di dalam terminal. Thunar memang bernama
paket `Thunar` (huruf T besar) dan `rum` mencocokkan nama secara peka huruf
besar.

### Browser

| Nama | Deskripsi | Paket | Repo | Ukuran | Default |
|---|---|---|---|---|---|
| Brave | Chromium dengan pemblokir iklan bawaan | `brave-browser` | RakuOS | 499 MB | |
| Chromium | Open source, tanpa sinkronisasi akun Google | `chromium` | Fedora | 374 MB | |
| Epiphany | GNOME Web (WebKitGTK), paling ringan | `epiphany` | Fedora | 11,5 MB | |
| Falkon | Qt/Chromium ringan, cocok sistem minimal | `falkon` | Fedora | 14,5 MB | |
| Firefox | Default, cepat, dan paling kompatibel | `firefox` | Fedora | 307 MB | ✅ |
| Google Chrome | Proprietary, sinkronisasi akun Google | `google-chrome-stable` | RakuOS | 466 MB | |
| Helium | Chromium privat dan cepat, dari Terra | `helium-browser-bin` | Terra | 410 MB | |
| LibreWolf | Firefox tanpa telemetri, fokus privasi | `librewolf` | RakuOS | 485 MB | |
| Microsoft Edge | Chromium Microsoft, paket paling besar | `microsoft-edge-stable` | RakuOS | 704 MB | |
| QuteBrowser | Keyboard-only, Qt/Python | `qutebrowser` | Fedora | 11 MB | |
| Vivaldi | Fitur banyak, proprietary | `vivaldi-stable` | RakuOS | 476 MB | |
| Waterfox | Firefox fork, fokus privasi | `waterfox` | RakuOS | 512 MB | |
| Zen Browser | Firefox fork, vertical tabs, dari Terra | `zen-browser` | Terra | 305 MB | |

Boleh centang lebih dari satu. Ukuran = paket utamanya saja.

### Login manager

| Nama | Deskripsi | Paket | Unit | Default |
|---|---|---|---|---|
| COSMIC Greeter | Greeter COSMIC (greetd), butuh stack | `cosmic-greeter` | `cosmic-greeter.service` | |
| DMS Greeter | Greeter Dux Shell (greetd), butuh quickshell | `greetd`, `dms-greeter` | `greetd.service` | |
| GDM | GNOME Display Manager, sudah ada di base image | `gdm` | `gdm.service` | |
| greetd + tuigreet | Greeter TUI paling ringan, cocok Wayland | `greetd`, `tuigreet` | `greetd.service` | ✅ |
| LightDM | Greeter Slick (GTK), sesi Wayland tetap bisa | `lightdm`, `slick-greeter` | `lightdm.service` | |
| Ly | Display manager TUI, ringan | `ly` | `ly.service` | |
| Noctalia Greeter | Greeter minimal Noctalia (greetd, wlroots) | `greetd`, `noctalia-greeter` | `greetd.service` | |
| Plasma Login | Greeter KDE modern (pengganti SDDM) | `plasma-login-manager` | `plasmalogin.service` | |
| SDDM | Display manager grafis, paling familiar | `sddm` | `sddm.service` | |
| Tanpa login manager | Autologin user langsung ke desktop di tty1 | — | — | |

KineticWE memaksa **KineticWE Greeter**, yaitu varian Noctalia bawaan
KineticWE. Pilihan Login tetap ditanyakan lebih dulu, tetapi kalau Desktop =
KineticWE, pilihan itu diabaikan dan diganti otomatis; ringkasan menampilkan
`(otomatis dari KineticWE)`. Karena itu `kineticwe-greeter` tidak muncul
sebagai pilihan: paketnya `Requires: kineticwe-git`, jadi hanya masuk akal
bersama desktop KineticWE.

### Desktop

| Nama | Deskripsi | Paket | Default |
|---|---|---|---|
| Hyprland | Tiling dinamis, config minimal tanpa bar | `hyprland`, `hyprland-guiutils`, `hyprland-uwsm`, `xdg-desktop-portal-hyprland`, `hyprland-preview-share-picker` | ✅ |
| KineticWE | Tiling berbasis KWin, shell Noctalia | `kineticwe-git` | |
| MangoWM | Tiling ringan berbasis dwl (wlroots) | `mangowm`, `scenefx`, `wlroots`, `xcb-util-wm`, `xdg-desktop-portal-wlr`, `slurp` | |
| Niri | Scrollable tiling, config minimal tanpa bar | `niri`, `xdg-desktop-portal-gnome` | |

`kineticwe-git` sudah `Requires` versi yang sama dari `kineticwe-noctalia` dan
`xdg-desktop-portal-kwe`, jadi cukup satu paket. `hyprland-uwsm` menarik
`uwsm`; `xdg-desktop-portal-hyprland` menarik `grim`, `slurp`, dan
`qt6-qtwayland`.

MangoWM memakai skel `data/skel/mango` (config TOML) dengan variabel
`$terminal`, `$launcher`, `$browser`, dan `$filemanager`. `$launcher` default
`fuzzel`, tetapi paketnya tidak dipasang: shell seperti Noctalia atau DMS
biasanya sudah membawa launcher dan agen polkit sendiri. Ganti `$launcher` ke
perintah shell kalau perlu. `exec_once` menjalankan
`dbus-update-activation-environment`; `slurp` melengkapi screenshot interaktif
dari `xdg-desktop-portal-wlr`. XWayland tidak diulang karena sudah ada di base
image.

### Lapisan dasar

Base image RakuOS sudah memasang audio (`pipewire`, `wireplumber`), GPU/video
(`mesa-*`, `libva`; image NVIDIA memakai driver sendiri), polkit, portal
(`xdg-desktop-portal`, `xdg-desktop-portal-gtk`, `xdg-user-dirs-gtk`), font,
`gvfs` + `gvfs-mtp`, `wl-clipboard`, dan `xorg-x11-server-Xwayland`. Wizard
tidak memasang ulang paket-paket itu.

Yang selalu ditambahkan wizard, apa pun pilihannya:

| Paket | Alasan |
|---|---|
| `dbus-daemon` | menyediakan `dbus-run-session`; image hanya membawa `dbus-broker` |
| `egl-wayland` | platform EGL Wayland, aman untuk Mesa maupun NVIDIA |

## Repo pihak ketiga

Terra dan RPM Fusion dibawa image dalam keadaan nonaktif. Wizard
menyalakannya sebelum instalasi, dengan cara yang sama seperti halaman
*Third-Party Repositories* di `rakuos-welcome`:

| Kunci di `catalog.toml` | Perintah |
|---|---|
| `terra` | `rum config-manager --set-enabled terra` |
| `rpmfusion` | `rum config-manager --set-enabled <isi /usr/share/rakuos/rpmfusion-default-repos>`, fallback `rpmfusion-*` |

Daftarnya ada di `repos = [...]` (level atas `catalog.toml`). Kosongkan kalau
tidak ingin repo pihak ketiga disentuh.

## Yang ditulis ke sistem

- `/var/lib/rakuos/bootstrap.toml` — pilihan terakhir:
  `terminals`, `file_managers`, `browsers` (dipisah spasi, boleh lebih dari
  satu), `desktop`, `login`, `repos`, `target_user`, `time`.
- `/var/lib/rakuos/bootstrap.done` — marker selesai; unit first-boot berhenti
  ditawarkan.
- `$HOME/.config/...` untuk user target (`SUDO_USER`, fallback `root`).
- `/usr/share/wayland-sessions/hyprland-uwsm.desktop` — sesi uwsm versi resmi
  (hanya kalau desktop = Hyprland).
- `/etc/pam.d/greetd` + `/var/lib/noctalia-greeter` — disiapkan skrip
  `setup_greeter_system.sh` milik greeter Noctalia/KineticWE.
- Display manager lain di-`disable` sebelum unit yang dipilih di-`enable`,
  supaya menjalankan wizard ulang tidak meninggalkan dua DM aktif.
- Drop-in `getty@tty1` + `~/.bash_profile` hanya kalau login = tanpa login
  manager.

## Menambah atau mengubah pilihan

Semua di `data/catalog.toml`, tanpa rebuild binary:

```toml
[[terminals]]
id = "foot"                     # dipakai di bootstrap.toml dan skel
name = "Foot"                   # yang tampil di layar
description = "..."             # satu baris, dipotong kalau terlalu panjang
packages = ["foot", "foot-terminfo"]
available = true                # false = tampil abu-abu "belum tersedia"
default = false                 # true = tercentang saat fase dibuka
```

Field khusus:

| Field | Untuk | Arti |
|---|---|---|
| `command` | semua | nama binary untuk skel kalau berbeda dari `id` (mis. `superfile` → `spf`) |
| `skel` | desktop | folder di `data/skel/` yang disalin ke `$HOME` |
| `session_file` | desktop | file di `data/sessions/` yang ditulis ke `/usr/share/wayland-sessions/` |
| `autologin_cmd` | desktop | perintah yang dijalankan saat login = tanpa login manager |
| `login_override` | desktop | id login yang dipaksa, apa pun pilihan user |
| `unit` | login | unit systemd display manager yang di-enable |
| `greetd_setup` | login | skrip setup paket greeter, dijalankan setelah `config.toml` |
| `hidden` | login | dipakai kode, tidak ditampilkan di layar |

## Dokumentasi lain

- [`ui.md`](ui.md) — tampilan tiap fase, charset, warna
