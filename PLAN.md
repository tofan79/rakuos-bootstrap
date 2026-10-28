# rakuos-bootstrap — rencana eksekusi

Dokumen ini yang dipakai saat mengeksekusi. Kalau ada yang tidak cocok dengan
kenyataan di mesin, **ubah dokumen ini dulu**, baru kodenya.

---

## 1. Masalah yang diselesaikan

RakuOS minimal hasil `rakuos-installer-cli` hanya punya TTY. Tidak ada
compositor, greeter, browser, atau file manager. User harus hafal `rum install`
dan tahu paket apa saja yang dibutuhkan sebelum bisa masuk GUI.

`rakuos-bootstrap` menggantikan hafalan itu dengan 5 pertanyaan.

**Bukan** installer OS. Yang menulis disk tetap `rakuos-installer-cli`.
Program ini jalan **setelah** sistem ter-install dan bisa dijalankan ulang
kapan saja dengan `sudo rakuos-bootstrap`.

---

## 2. Keputusan yang sudah diambil

| Hal | Keputusan | Alasan |
|---|---|---|
| Nama paket/binary | `rakuos-bootstrap` | `rakuos-setup.service` sudah dipakai `rakuos-core`; `rakuos-install-dasar` menyesatkan (kedengaran seperti installer OS) |
| Bahasa | Rust | konsisten dengan `rakuos-installer` (backend yang sama), satu binary statis, tanpa runtime script |
| TUI | modul `ui` sendiri di atas `console` | tidak butuh `dialoguer`; mendeteksi UTF-8/console VT/ASCII dan warna 256/16/none, jadi tetap rapi di `TERM=linux` |
| Model instalasi | `rum` (overlay) | tidak rebase, tidak ubah base image, persisten setelah update |
| Browser | RPM saja | semua browser yang dipakai sudah tersedia sebagai RPM (Fedora/RakuOS), jadi tidak perlu Flatpak |
| Desktop "pure" | compositor + config minimal | tanpa paket meta/app bawaan; tapi tetap diberi keybind dasar supaya tidak terlihat rusak |
| Lokasi kode | `/var/home/mindset/Projects/rakuos-bootstrap/` | repo terpisah dulu; RPM menyusul di `Mindset-Apps` |

### Yang belum masuk (backlog, jangan dikerjakan sekarang)

- `mangowm` — belum ada di repo Fedora/RakuOS, perlu COPR/spec sendiri.
- `kineticwe` — belum jelas paket/hulunya; tunggu link.
- Singularity OS / Singularity Desktop (`singularityos-lab`) — ini DE GTK4/Vala
  untuk distro lain (Sinty OS), belum ada paket Fedora. Bukan pekerjaan wizard.
- `ghostty` — sekarang tersedia lewat repo Terra (lihat §6.2); COPR sendiri
  baru perlu kalau Terra mau dilepas.
- Desktop penuh (KDE/GNOME/Xfce/Cinnamon/LXQt/Budgie/COSMIC) — bukan "pure",
  dan akan menarik banyak app. Boleh masuk nanti sebagai kategori terpisah.

---

## 3. Penting: app ini harus ada di **image target**, bukan di ISO

`bootc install` menulis image dari registry (`quay.io/rakuos/rakuos-minimal:staging`).
Apa pun yang dipasang di live ISO **tidak ikut** ke sistem hasil install.

Artinya `rakuos-bootstrap` harus ada di image yang di-install. Dua cara:

1. **Derived image (dipakai)**: `custome-rakuos` → image minimal + paket
   `rakuos-bootstrap` → push `quay.io/mindset404/rakuos-minimal` → ISO memakai
   image itu sebagai `--container-image`.
2. Download saat first boot — ditolak: butuh jaringan sebelum ada yang bisa
   dijalankan, dan tidak ada yang menjalankan downloader itu.

Di tahap uji, wizard tetap bisa dicoba manual di sistem minimal yang sudah
ter-install (copy binary + `sudo ./rakuos-bootstrap --dry-run`).

---

## 4. Alur

```
first boot (image minimal)
        │
        ▼
systemd: rakuos-bootstrap.service  (TTY1, root)
        │  ConditionPathExists=!/var/lib/rakuos/bootstrap.done
        ▼
┌──────────────────────────────────────────────┐
│ 1. Terminal     (boleh >1)                    │
│ 2. File manager (boleh >1)                    │
│ 3. Browser      (boleh >1)                    │
│ 4. Login        (satu)                        │
│ 5. Desktop      (satu)                        │
└──────────────────────────────────────────────┘
        │
        ▼
ringkasan → konfirmasi → enable repo → rum make-cache → install
        │
        ├─ rum install <base + pilihan>
        ├─ salin skel desktop ke ~/.config (chown ke user)
        ├─ enable greeter / siapkan autologin
        ├─ tulis /var/lib/rakuos/bootstrap.toml + .done
        ▼
reboot → greeter / desktop
```

Kalau dibatalkan di tengah: tidak ada marker `.done`, jadi sistem menawarkan
lagi saat boot berikutnya (atau user menjalankan `sudo rakuos-bootstrap`).

---

## 5. Katalog pilihan

File: `data/catalog.toml` (di-install ke
`/usr/share/rakuos/bootstrap/catalog.toml`). Menambah pilihan = edit TOML,
tanpa recompile.

### Lapisan dasar (selalu dipasang)

| Paket | Alasan |
|---|---|
| `pipewire`, `pipewire-pulseaudio`, `wireplumber` | audio untuk browser/desktop |
| `mesa-dri-drivers`, `mesa-vulkan-drivers`, `libva` | GPU + video decode |
| `polkit`, `lxpolkit` | agent polkit (GUI prompt tidak muncul tanpa ini) |
| `xdg-desktop-portal`, `xdg-desktop-portal-gtk` | file picker, screen share |
| `dbus-tools` | `dbus-run-session`, `dbus-update-activation-environment` |
| `default-fonts`, `google-noto-sans-fonts`, `google-noto-color-emoji-fonts` | teks + emoji |
| `fuzzel` | launcher keyboard (dipakai config minimal) |
| `gvfs`, `gvfs-mtp` | mount USB/HP dari file manager |

### Pilihan

| Kategori | id | Paket | Status |
|---|---|---|---|
| terminal | `kitty` | kitty | ✅ Fedora 45 |
| terminal | `foot` | foot | ✅ Fedora 45 |
| terminal | `alacritty` | alacritty | ✅ Fedora 45 |
| terminal | `ghostty` | ghostty | ✅ repo Terra (diaktifkan otomatis) |
| browser | `brave` | brave-browser | ✅ repo RakuOS 45 |
| browser | `chromium` | chromium | ✅ Fedora 45 |
| browser | `epiphany` | epiphany | ✅ Fedora 45 (WebKitGTK, paling ringan) |
| browser | `falkon` | falkon | ✅ Fedora 45 |
| browser | `firefox` | firefox | ✅ Fedora 45 |
| browser | `google-chrome` | google-chrome-stable | ✅ repo RakuOS 45 |
| browser | `helium` | helium-browser-bin | ✅ Terra |
| browser | `librewolf` | librewolf | ✅ repo RakuOS 45 |
| browser | `microsoft-edge` | microsoft-edge-stable | ✅ repo RakuOS 45 |
| browser | `qutebrowser` | qutebrowser | ✅ Fedora 45 |
| browser | `vivaldi` | vivaldi-stable | ✅ repo RakuOS 45 |
| browser | `waterfox` | waterfox | ✅ repo RakuOS 45 |
| browser | `zen-browser` | zen-browser | ✅ Terra |
| desktop | `hyprland` | hyprland, hyprland-guiutils, hyprland-uwsm, uwsm, egl-wayland, xdg-desktop-portal-hyprland | ✅ repo RakuOS 45 + Terra (`uwsm`) |
| desktop | `kineticwe` | kineticwe-git | ✅ repo RakuOS 45 (menarik kineticwe-noctalia, xdg-desktop-portal-kwe, Xwayland) |
| desktop | `mangowm` | mangowm, scenefx, wlroots, xorg-x11-server-Xwayland, xdg-desktop-portal-wlr | ✅ Terra (`mangowm`, `scenefx`) + Fedora 45 |
| desktop | `niri` | niri, xdg-desktop-portal-gnome | ✅ Fedora 45 (menarik xwayland-satellite) |
| desktop | `labwc` | labwc | ✅ Fedora 45 |
| desktop | `mangowm` | — | ⏳ belum dipaketkan |
| file manager | `dolphin` | dolphin | ✅ Fedora 45 (menarik Qt/KDE) |
| file manager | `hyprfm` | hyprfm | ✅ repo RakuOS 45 |
| file manager | `nautilus` | nautilus | ✅ Fedora 45 (menarik GTK4) |
| file manager | `nemo` | nemo | ✅ Fedora 45 (menarik xapps) |
| file manager | `superfile` | superfile | ✅ Terra (TUI, binary `spf`) |
| file manager | `thunar` | Thunar, thunar-volman | ✅ Fedora 45 (nama paket berhuruf besar) |
| file manager | `yazi` | yazi | ✅ Terra (TUI) |
| login | `cosmic-greeter` | cosmic-greeter | ✅ repo RakuOS 45 (menarik greetd) |
| login | `gdm` | gdm | ✅ sudah ada di base image |
| login | `greetd-tuigreet` | greetd, tuigreet | ✅ Fedora 45 |
| login | `lightdm-slick` | lightdm, slick-greeter | ✅ Fedora 45 |
| login | `ly` | ly | ✅ Fedora 45 |
| login | `plasma-login` | plasma-login-manager | ✅ Fedora 45 |
| login | `sddm` | sddm | ✅ Fedora 45 |
| login | `none` | — (autologin ke desktop) | ✅ |

Catatan: `rum repoquery` mencocokkan nama paket **peka huruf besar**; paket
Thunar memang bernama `Thunar` (huruf T besar). Nama di `packages` harus
sama persis dengan nama RPM.

Catatan verifikasi: dicek dengan `dnf repoquery --available` di container
`quay.io/fedora/fedora:45` dan metadata `repo.rakuos.org/rakuos/45` pada
2026-10-26. `hyprpolkitagent` **tidak** ada di Fedora 45 → pakai `lxpolkit`.

---

## 6. Perilaku instalasi (detail yang gampang bikin gagal)

### 6.1 Paket

- Semua paket RPM lewat `rum install -y <daftar>`.
- `rum` adalah package manager overlay RakuOS: paket masuk ke
  `/var/lib/rakuos/overlay/upper`, dicatat di `/var/lib/rakuos/packages.list`,
  dan tetap ada setelah image di-update.
- Paket yang sudah ada di base image **tidak bisa** di-overlay. `rum` akan
  melaporkannya, bukan gagal — jangan dianggap error.
- Jangan pakai `dnf` langsung: di sistem overlay, `dnf` adalah shim `rum`,
  tapi eksplisit `rum` lebih jelas di log.
- Kalau `rum` tidak ada (uji di distro lain), fallback ke `dnf`, tapi tulis
  peringatan bahwa overlay tidak aktif.

### 6.2 Repo pihak ketiga

Terra dan RPM Fusion dibawa image dalam keadaan `enabled=0`. Wizard
menyalakannya sebelum paket dipasang, mengikuti halaman *Third-Party
Repositories* di `rakuos-welcome`:

| Kunci di `catalog.toml` | Perintah |
|---|---|
| `terra` | `rum config-manager --set-enabled terra` |
| `rpmfusion` | `rum config-manager --set-enabled <isi /usr/share/rakuos/rpmfusion-default-repos>`; fallback `rpmfusion-*` |

- Daftar repo ada di level atas `catalog.toml` (`repos = [...]`). Kosongkan
  kalau tidak ingin repo pihak ketiga disentuh.
- Ghostty hanya ada di Terra. Tanpa langkah ini `rum install ghostty` gagal
  dengan "No match for argument".
- `--set-enabled <id eksplisit>` menyalakan repo persis seperti yang
  ditulis, termasuk `rpmfusion-*-updates-testing` (rakuos-base mencatatnya
  sebagai default RPM Fusion).
- Wildcard `rpmfusion-*` justru melewati source/debuginfo/testing; itu
  sebabnya fallback tidak sama dengan daftar default.
- Kalau `rum` tidak ada, langkah ini dilewati dengan peringatan (bukan
  gagal), karena `dnf` tidak mengenal overlay RakuOS.

Setelah repo dinyalakan, metadata disegarkan dengan `rum make-cache` (fallback
`dnf makecache`). Perintah ini hanya mengambil metadata; berbeda dengan
`rum update`/`upgrade` yang ikut menyentuh paket overlay.

### 6.3 Config minimal (skel)

- Sumber: `/usr/share/rakuos/bootstrap/skel/<id>/.config/...`
- Disalin ke `$HOME` user target (`SUDO_USER`, fallback `root`), lalu
  `chown -R user:user $HOME/.config`.
- Placeholder `@TERMINAL@` diganti dengan terminal pertama menurut abjad
  dari yang dicentang (boleh lebih dari satu terminal).
- Isi Hyprland minimal: monitor auto, `$terminal`, `exec-once` polkit agent,
  `SUPER+Return` terminal, `SUPER+D` fuzzel, `SUPER+Q` close, `SUPER+M` exit.
- Isi niri minimal: keybind yang sama + `spawn-at-startup` polkit agent.
- **Tanpa bar** (tetap "pure"). Bar bisa jadi item terpisah nanti.

### 6.4 Login manager

| Pilihan | Unit | Aksi |
|---|---|---|
| `cosmic-greeter` | `cosmic-greeter.service` | `systemctl enable`, `set-default graphical.target`; unit ini menjalankan `greetd` dengan config COSMIC |
| `dms-greeter` | `greetd.service` | Tulis `/etc/greetd/config.toml` dengan `dms-greeter --command <kompositor>` dan user `greeter` (dibuat paket lewat sysusers) |
| `gdm` | `gdm.service` | `systemctl enable`, `set-default graphical.target`; GDM sudah ada di base image |
| `greetd-tuigreet` | `greetd.service` | Sesi dibaca dari `/usr/share/wayland-sessions` |
| `lightdm-slick` | `lightdm.service` | Drop-in `/etc/lightdm/lightdm.conf.d/50-rakuos-bootstrap.conf` berisi `greeter-session=slick-greeter` |
| `ly` | `ly.service` | `systemctl enable`, `set-default graphical.target` |
| `noctalia-greeter` | `greetd.service` | Tulis config greetd berisi `noctalia-greeter-session`, user `greeter` dibuat kalau belum ada |
| `plasma-login` | `plasmalogin.service` | Greeter KDE, pengganti SDDM |
| `sddm` | `sddm.service` | `systemctl enable`, `set-default graphical.target` |
| `none` | — | `set-default multi-user.target`, enable `getty@tty1` + autologin user, `~/.bash_profile` menjalankan `dbus-run-session <autologin_cmd>` |

Semua pilihan selain `none` di-enable dengan `systemctl enable <unit>` lalu
`systemctl set-default graphical.target`. Drop-in LightDM dihapus lagi kalau
user pindah ke login manager lain.

Greeter berbasis greetd (`greetd-tuigreet`, `dms-greeter`, `noctalia-greeter`)
menulis `/etc/greetd/config.toml`; config lama disimpan sekali sebagai
`config.toml.bak`. `@COMPOSITOR@` diganti sesuai desktop yang dipilih
(hyprland, niri, labwc, mango). User `greeter` dibuat kalau belum ada dan
dimasukkan ke grup video/render/input (best-effort).

Autologin user (bukan root) untuk pilihan `none`. Jangan pernah autologin root
di sistem ter-install.

`autologin_cmd` per desktop: `Hyprland`, `start-kineticwe`, `labwc`, `mango`,
`niri-session`.

### 6.5 Marker & state

- `/var/lib/rakuos/bootstrap.toml` — pilihan terakhir (untuk audit/re-run).
  Kunci: `terminals`, `file_managers`, `browsers` (dipisah spasi, boleh lebih
  dari satu), `desktop`, `login`, `repos`, `target_user`, `time`.
- `/var/lib/rakuos/bootstrap.done` — marker selesai; unit first-boot berhenti
  ditawarkan.
- Kalau user membatalkan: jangan tulis marker.

---

## 7. Struktur file

```
rakuos-bootstrap/
├── Cargo.toml
├── README.md
├── PLAN.md                      <- dokumen ini
├── src/
│   ├── main.rs                  <- argumen, langkah TUI, ringkasan
│   ├── catalog.rs               <- struct + loader catalog.toml
│   └── install.rs               <- rum, skel, login, marker
├── data/
│   ├── catalog.toml
│   ├── rakuos-bootstrap.service
│   └── skel/
│       ├── hyprland/.config/hypr/hyprland.conf
│       └── niri/.config/niri/config.kdl
└── packaging/
    └── rakuos-bootstrap.spec    <- nanti, dipindah ke Mindset-Apps
```

### Packaging RPM (nanti di Mindset-Apps)

Instalasi file:

| Dari | Ke |
|---|---|
| `target/release/rakuos-bootstrap` | `/usr/bin/rakuos-bootstrap` |
| `data/catalog.toml` | `/usr/share/rakuos/bootstrap/catalog.toml` |
| `data/skel/*` | `/usr/share/rakuos/bootstrap/skel/*` |
| `data/rakuos-bootstrap.service` | `/usr/lib/systemd/system/rakuos-bootstrap.service` |

`BuildRequires`: `cargo`, `rust`, dan crate vendored (offline build di COPR
perlu `cargo vendor` + `.cargo/config.toml`, seperti spec Rust lain di
`Mindset-Apps`).

`%post`: tidak enable unit otomatis di semua sistem. Unit di-enable dari
derived image (`systemctl enable rakuos-bootstrap.service`) supaya ISO biasa
tidak ikut menampilkan wizard.

---

## 8. Unit first boot

`data/rakuos-bootstrap.service`:

```ini
[Unit]
Description=RakuOS bootstrap (pilih desktop & aplikasi dasar)
After=network-online.target
Wants=network-online.target
ConditionPathExists=!/var/lib/rakuos/bootstrap.done

[Service]
Type=oneshot
ExecStart=/usr/bin/rakuos-bootstrap --first-boot
StandardInput=tty-force
StandardOutput=tty
StandardError=tty
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
TTYVTDisallocate=yes

[Install]
WantedBy=multi-user.target
```

`--first-boot` = tidak menanyakan konfirmasi reboot dua kali dan menulis
marker; kalau gagal, unit gagal tapi sistem tetap bisa login TTY.

---

## 9. Testing

### 9.1 Tanpa mengubah sistem

```bash
cargo build --release
sudo ./target/release/rakuos-bootstrap --dry-run
```
Harus mencetak daftar paket + perintah yang akan dijalankan, tanpa menyentuh
sistem.

### 9.2 Di sistem minimal (VM)

1. Install RakuOS minimal di VM (ISO hasil branch `minimal`).
2. Copy binary + `data/` ke VM, jalankan `sudo ./rakuos-bootstrap`.
3. Pilih: kitty + firefox RPM + hyprland + nautilus + greetd+tuigreet.
4. Reboot → greetd → Hyprland → `SUPER+Return` membuka kitty → browser jalan.
5. Cek persistensi: `rum list --installed | grep -E 'kitty|firefox'` dan
   `/var/lib/rakuos/packages.list`.
6. Ulangi skenario `login=none` (autologin) dan `login=sddm`.

### 9.3 Matriks minimum sebelum dianggap selesai

| Skenario | Harus lolos |
|---|---|
| dry-run sebagai non-root | mencetak rencana, tidak error permission |
| hyprland + greetd | masuk GUI, terminal keybind jalan, polkit prompt muncul |
| niri + greetd | sama |
| login `none` | autologin ke desktop tanpa layar login |
| browser RPM | firefox terpasang dan jalan |
| batalkan di ringkasan | tidak ada `.done`, tidak ada paket terpasang |
| jalankan ulang | pilihan lama muncul sebagai default |

---

## 10. Risiko

| Risiko | Mitigasi |
|---|---|
| Overlay `rum` menolak paket yang sudah ada di base | deteksi output, jangan anggap gagal; log jelas |
| `xdg-desktop-portal-hyprland` hanya ada di repo RakuOS | pastikan repo RakuOS aktif (sudah default di image) |
| Config minimal tidak cocok versi Hyprland/niri | uji di image staging terbaru; config hanya keybind dasar |
| Wizard jalan sebagai root, file user jadi milik root | `chown -R` setelah salin skel |
| Boot ke TTY tapi user tidak tahu harus apa | marker `.done` + pesan "jalankan sudo rakuos-bootstrap" di `/etc/motd` |

---

## 11. Checklist eksekusi

- [x] Tentukan nama & alasan (`rakuos-bootstrap`)
- [x] Tentukan TUI (modul `ui` sendiri, aman di console VT)
- [x] Verifikasi ketersediaan paket Fedora 45 + RakuOS 45
- [x] Scaffold Rust + katalog TOML + skel
- [ ] `cargo build` bersih di mesin ini
- [ ] Uji `--dry-run` (tanpa root)
- [ ] Uji nyata di VM minimal: hyprland + greetd + kitty + firefox
- [ ] Uji `login=none`
- [ ] Spec RPM di `Mindset-Apps` + build COPR
- [ ] Masukkan paket ke derived image (`custome-rakuos`) + rebuild ISO
- [ ] Dokumentasi pengguna singkat di README

---

## 12. Backlog (setelah 5 pilihan dasar stabil)

1. Bar opsional (waybar) + tema.
2. `ghostty` via COPR sendiri (sekarang masih dari Terra).
3. `mangowm` (spec sendiri) — kalau memang mau.
4. Desktop penuh (KDE/Xfce/Cinnamon/LXQt/Budgie/COSMIC) sebagai kategori
   "full desktop", ditandai jelas bukan pure.
5. Mode non-interaktif (`--preset hyprland-kitty-firefox`) untuk provisioning.
6. Integrasi `rakuos-shell-picker` setelah desktop Hyprland terpasang.
