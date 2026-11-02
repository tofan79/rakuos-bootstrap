# Plan: Paket RPM Singularity Desktop di Mindset-Apps

## Status singkat

- Baseline: `singularityos-lab/packaging` tag `rpm-nobara44-20261008`
  (`singularity-desktop-0.1.0-1.fc44`) plus tiga paket prasyarat buatan
  upstream: `vala-cups`, `gstreamer1-plugin-voaacenc`, `librespot`.
- Target build: Fedora 45 x86_64, COPR `mindset/Mindset-Apps`.
- Isi baseline (terverifikasi dari `SPECS/` dan `SOURCES/`):
  - 4 source RPM, 89 binary RPM, 85 subpackage.
  - 148 baris `BuildRequires`, 152 kapabilitas unik untuk keempat spec.
  - `Source0..13`, `Patch0..2`, `%build` = `native-build.sh`, `%install` =
    `native-install.sh`.
  - `source-manifest.json`: 82 repositori (root + 81 submodule), 76 komponen
    terpasang, arsip `singularity-desktop-0.1.0.tar.zst`.
- Verifikasi upstream (`VERIFICATION.md`): 325 tes meson lulus di VM Nobara 44,
  2.620 hash file cocok, 237 ELF resolve, 57 launcher tampil.
- Keputusan maturitas:
  - **Singularity Desktop**: **GO**, sebagai satu paket sumber monolit.
  - **Login manager Singularity**: **GO**, sebagai paket glue greetd terpisah.
  - **sinty-logind**: **NO-GO** untuk Fedora/RakuOS berbasis systemd.

## Pemisahan yang harus dipahami

1. `singularity-greeter` (RPM, dari SRPM desktop) adalah klien GTK4 layer-shell,
   **bukan** compositor. Ia butuh compositor Wayland untuk tampil.
2. Glue greetd tidak dikirim oleh RPM mana pun. Semuanya ada di
   `os/rootfs-overlay` milik image SintyOS (`/etc/greetd/config.toml`,
   `singularity-greeter-session`, `-client`, `-startup`,
   `singularity-session-common`, unit `singularity-compositor.service`), jadi
   harus jadi paket sendiri.
3. `sinty-logind` adalah komponen inti SintyOS: pengganti `systemd-logind` dan
   `seatd`, sekaligus pemilik `org.freedesktop.login1`. Bukan kandidat RPM
   Fedora.
4. Upstream menaruh `Requires: gdm` di subpackage `singularity-session`. Di
   RakuOS ini harus dilonggarkan supaya sesi Singularity bisa dipakai bersama
   greetd/SDDM tanpa menarik GDM.

## Temuan yang sudah diverifikasi

| Hal | Hasil | Sumber |
| --- | --- | --- |
| Build privat | libinput → `/usr/lib64/singularity`, labwc + wlroots + scenefx → `/usr/lib64` (rpath `$ORIGIN/../../lib64/singularity`), libgusb + libfprint → `/usr/lib64/singularity/fprint` | `SOURCES/native-build.sh`, `native-install.sh` |
| Installer & librespot saat build | dimatikan: `-Dinstaller=false`, `-Dsingularity-media-plugins:librespot=disabled` | `native-build.sh` |
| Pengecualian dependency | `__requires_exclude` untuk `libfprint-2-tod.so.1` dan `libgusb.so.2` (keduanya salinan privat) | header spec |
| Ketersediaan `BuildRequires` | 149 dari 152 tersedia di fedora + updates + rpmfusion-free (F45) | audit `rum repoquery --whatprovides` |
| Sisa 3 `BuildRequires` | `vala-cups`, `gstreamer1-plugin-voaacenc`, `pkgconfig(vo-aacenc)` — hanya dari SRPM upstream | audit yang sama |
| Codec di RPM Fusion | `gstreamer1-plugins-ugly` dan `gstreamer1-plugins-bad-freeworld` 1.28.7 ada di `rpmfusion-free` | `repoquery --queryformat '%{repoid}...'` |
| `%check` | hanya accessibility + keyring di Xvfb; 325 tes meson tidak dijalankan saat build RPM | `%check` spec |
| Dependency runtime yang berisiko | `gdm` (singularity-session), `librespot` (media-plugins), `gstreamer1-plugins-ugly`/`bad-freeworld`/`voaacenc` + `/usr/bin/ffmpeg` (keyframe, montage, photos), `vala-cups` (libsingularity-devel) | `Requires:` per subpackage |
| Sesi desktop | `wayland-sessions/singularity.desktop` → `singularity-labwc-session`, compositor `/usr/libexec/singularity/labwc` | `native-install.sh` |
| Greeter di spec | spec tidak menyebut `greetd`, `greeter-session`, `labwc-locked`, atau `config.toml` | grep spec |

## Strategi umum

1. Pakai `singularityos-lab/packaging/rpm` sebagai baseline; jangan build dari nol.
2. Pertahankan spec desktop sebagai paket monolit agar tetap sesuai baseline.
3. Semua migrasi Fedora 45 dikerjakan di Mindset-Apps, di container/VM Fedora 45
   — bukan di host RakuOS.
4. Bangun tiga paket prasyarat lebih dulu, baru desktop.
5. RPM Fusion dipakai sebagai external repo COPR untuk codec, atau dependency
   codec dipindah ke `Requires` runtime (keputusan di Tahap 2).
6. `sinty-logind` tidak masuk pilihan login dan tidak menjadi paket default.

## Struktur target

```text
specs/
  singularity-desktop-fedora45.spec
  vala-cups.spec
  gstreamer1-plugin-voaacenc.spec
  librespot.spec
  singularity-greeter-setup.spec
data/singularity/
  source-manifest-baseline.txt
  greeter/            # config greetd + wrapper hasil port dari os/rootfs-overlay
workflows/
  singularity-desktop.yml
  singularity-greeter-setup.yml
```

## Tahap 1 — Import baseline

1. Unduh source RPM (butuh `gh`):
   ```sh
   ./rpm/fetch.sh            # hanya SRPMS/
   ./rpm/fetch.sh all        # SRPMS/ + RPMS/ untuk uji cepat
   ```
2. Ekstrak baseline ke `rpm/` lokal:
   ```sh
   rpm -i --define "_topdir $PWD/rpm" ./rpm/SRPMS/*.src.rpm
   ```
3. Salin `SPECS/` + `SOURCES/` ke Mindset-Apps, catat SHA256 dari
   `SRPM-SHA256SUMS`.
4. Audit awal:
   - `__requires_exclude` (`libfprint-2-tod`, `gusb`) masih diperlukan karena
     kedua library dibangun privat.
   - `Patch0..2` (drag icon, accessible text, runtime compat) harus tetap ada.
   - `Source5` (vetro vendor) dan `Source9` (runtime notices) diekstrak di
     `%prep`; jangan lupa ikut disalin.

### Checkpoint
- [ ] `rpmlint` tidak menambah temuan baru dibanding baseline.
- [ ] `Source0..13` + SHA256 cocok.
- [ ] Keempat SRPM ter-import dan bisa di-`rpmbuild --rebuild` di F45.

## Tahap 2 — Migrasi Fedora 45

Jalankan di container Fedora 45 dengan volume kerja, misalnya:

```sh
podman run --rm -it -v "$PWD:/work:Z" fedora:45 bash
dnf install -y rpm-build rpmdevtools dnf-plugins-core
dnf install -y \
  https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
```

Urutan build (wajib, karena saling membutuhkan):

1. `vala-cups` → install.
2. `gstreamer1-plugin-voaacenc` → install (menyediakan `pkgconfig(vo-aacenc)`).
3. `librespot` → install (kebutuhan runtime `singularity-media-plugins`).
4. `singularity-desktop`:

   ```sh
   dnf builddep ./rpm/SRPMS/singularity-desktop-0.1.0-1.fc44.src.rpm
   rpmbuild --rebuild ./rpm/SRPMS/singularity-desktop-0.1.0-1.fc44.src.rpm
   ```

Hasil audit dependency untuk F45:

| Kebutuhan | Status | Tindakan |
| --- | --- | --- |
| 149 dari 152 `BuildRequires` | tersedia di fedora/updates/rpmfusion-free | pakai langsung |
| `gstreamer1-plugins-ugly`, `gstreamer1-plugins-bad-freeworld` | `rpmfusion-free` 1.28.7 | aktifkan RPM Fusion di COPR |
| `gstreamer1-plugin-voaacenc`, `pkgconfig(vo-aacenc)` | tidak ada | build SRPM upstream |
| `vala-cups` | tidak ada | build SRPM upstream |
| `librespot` (runtime media-plugins) | tidak ada | build SRPM upstream |

Penyesuaian spec yang perlu diputuskan saat migrasi:

- `Requires: gdm` pada `singularity-session` → `Recommends:` (atau `%bcond`
  `gdm`) supaya greetd/SDDM tetap bisa jadi login manager.
- Dependency codec (`singularity-keyframe`, `-montage`, `-photos`) tetap
  `Requires`, tetapi `BuildRequires` codec hanya perlu bila `%check` memakainya.
  Jika COPR tidak mau memasang RPM Fusion, hapus dari `BuildRequires` dan biarkan
  sebagai `Requires` runtime.
- Jangan menyentuh `%global debug_package %{nil}` dan `_lto_cflags` tanpa alasan
  ukuran build.

### Checkpoint
- [ ] `meson compile` (via `native-build.sh`) lulus di F45.
- [ ] `rpmbuild --rebuild` keempat SRPM lulus.
- [ ] `%check` lulus (`xvfb-run`, accessibility, keyring).
- [ ] Tidak ada `Requires` yang menunjuk `/usr/lib64/singularity/*` bocor ke
      dependency otomatis.
- [ ] GDM menampilkan entri "Singularity" (uji cepat sebelum greeter diport).
- [ ] 1–2 aplikasi essential jalan.

## Tahap 3 — Paket greeter Singularity

Paket `singularity-greeter-setup` hanya berisi glue, bukan biner greeter:

- `/usr/libexec/singularity-greeter/session` — hasil port
  `os/rootfs-overlay/usr/bin/singularity-greeter-session`.
- `/usr/share/singularity/greeter/labwc-locked/` — config labwc (bila tidak
  ternyata sudah dikirim paket `singularity-labwc`).
- unit/entry yang dibutuhkan greetd, dan skrip setup idempoten.
- `Requires: greetd`, `singularity-greeter`, `singularity-labwc`.

Pilihan jalur: **self-compositor**, yaitu greetd menjalankan satu perintah yang
memulai labwc lalu greeter di dalamnya:

```sh
labwc -C /usr/share/singularity/labwc-locked -s /usr/bin/singularity-greeter
```

Alasannya: jalur *persistent compositor* milik upstream
(`singularity-compositor.service`) bergantung pada `sinty-logind`/`seatd`,
`ConditionPathExists=/run/atom/installed`, grup `singularity-session`, dan uid
`102` yang di-hardcode. Sebuah systemd service tidak punya sesi logind, jadi di
Fedora DRM master tidak bisa diambil lewat libseat/logind. Sebaliknya, perintah
yang dijalankan greetd adalah sesi logind sungguhan, sehingga akses DRM/input
aman tanpa seatd.

Port yang wajib dilakukan pada glue:

| Asal (SintyOS) | Ganti dengan |
| --- | --- |
| `XDG_RUNTIME_DIR=/run/user/102`, `HOME=/run/user/102` | `XDG_RUNTIME_DIR` dari greetd/PAM, `HOME=/var/lib/greeter` |
| `/var/lib/singularity/vconsole.conf`, `locale.conf` | `/etc/vconsole.conf`, `/etc/locale.conf` (atau symlink) |
| sentinel splash `/run/singularity/boot-splash-up`, `greeter-ready` | dilewati; splash tidak wajib |
| `/usr/bin/labwc` | `/usr/libexec/singularity/labwc` (sesuai `native-install.sh`) |
| `sscommon_exec_labwc` | tetap, tetapi tanpa penantian splash |
| `atom-install-session` (`[initial_session]`) | dibuang |
| `dbus-run-session` | tetap |

Config greetd yang ditulis paket:

```toml
[terminal]
vt = 1

[default_session]
command = "/usr/libexec/singularity-greeter/session"
user = "greeter"
```

Catatan:

- Jangan pakai jalur `singularity-greeter-client` (single-compositor) di Fedora.
- User `greeter` ditambahkan ke `video`, `render`, `input`.
- Disable display manager lain secara eksplisit; mode preview aman (setup tanpa
  `enable`) tetap disediakan.
- `greetd` tetap pemilik `/etc/greetd/config.toml`; paket hanya menulis isinya
  dengan backup `.bak` sekali.

Integrasi `rakuos-bootstrap` hanya setelah paket stabil:

```toml
[[logins]]
id = "singularity-greeter"
name = "Singularity Greeter"
description = "Greeter Singularity (greetd, labwc)"
packages = ["greetd", "singularity-greeter", "singularity-labwc", "singularity-greeter-setup"]
unit = "greetd.service"
greetd_cmd = "/usr/libexec/singularity-greeter/session"
greetd_user = "greeter"
```

### Checkpoint
- [ ] Greeter tampil setelah reboot di F45 (VM).
- [ ] Login user berhasil dan masuk sesi Singularity.
- [ ] Keyboard/locale mengikuti `/etc/vconsole.conf`.
- [ ] Gagal-DRM tidak lagi black screen tanpa log (log ke file, bukan tty).

## Tahap 4 — `sinty-logind`: NO-GO

- Tidak masuk pilihan `login`.
- Tidak menggantikan `systemd-logind`.
- Tidak memasang service `login1` di Fedora.
- Bila tetap ingin dipelihara, batasi sebagai unit preview: build + tes saja,
  tanpa `enable`, dan `systemctl is-active systemd-logind` harus tetap jalan.

## Workflow otomatisasi

- Build dipicu manual/nightly dari tag baseline (`rpm-nobara44-YYYYMMDD`).
- Smart skip berdasarkan tag baseline, bukan versi aplikasi.
- Jangan auto-update terlalu sering: satu build = 89 RPM.
- COPR: tambahkan `rpmfusion-free` sebagai external repo bila dependency codec
  dipertahankan di `BuildRequires`.
- Paket prasyarat (`vala-cups`, `voaacenc`, `librespot`) dibangun di source COPR
  yang sama sebelum desktop, atau dipublikasikan sebagai paket terpisah agar
  urutan build tidak rapuh.

## Risiko utama

1. **`Requires: gdm`** pada `singularity-session` menarik GDM di RakuOS.
2. **Codec dari RPM Fusion** — COPR harus punya Fusion, atau pemisahan
   BuildRequires/Requires harus benar.
3. **Library privat** (`/usr/lib64/singularity`, `fprint`) — jangan sampai
   dependency otomatis menunjuk ke path privat atau ke `libfprint` Fedora.
4. **Ukuran SRPM** besar (`tar.zst` + vendor vetro + runtime notices); COPR
   punya batas ukuran dan waktu build.
5. **`labwc-locked`** belum bisa diverifikasi dari checkout (submodule kosong),
   harus dicek isi paket `singularity-labwc` setelah build pertama.
6. **Prefix `/usr` + `lib64`** — skrip sesi sudah disesuaikan `native-install.sh`,
   tetapi skel/session RakuOS harus mengikuti `singularity-labwc-session`.
7. **Jalur compositor persisten** tidak cocok dengan systemd/logind (lihat
   Tahap 3); jangan dipaksakan.
8. **Audit dependency runtime** 137 entri belum lengkap; beberapa paket runtime
   (`tailscale`, `tesseract-langpack-eng`, `edk2-ovmf`, `virtiofsd`) perlu dicek
   ketersediaannya di Fedora 45 sebelum dijadikan `Requires` wajib.

## Testing

1. Smoke: GDM → sesi "Singularity" → shell, files, terminal.
2. Greeter: config greetd + user `greeter` + reboot test (VM F45).
3. Multi-desktop: Hyprland/Mango/Sway tetap bisa dipilih di bootstrap,
   tidak terpengaruh paket Singularity.
4. No-go: `systemctl is-active systemd-logind` tetap aktif.
5. Cek `rpm -qp --requires` keempat belas subpackage penting, pastikan tidak ada
   kebutuhan path privat yang bocor.

## Urutan eksekusi

1. Import baseline + build prasyarat di container F45.
2. Build desktop, perbaiki dependency yang gagal.
3. Uji sesi Singularity lewat GDM di VM.
4. Cabang greeter: port glue + paket `singularity-greeter-setup`.
5. Uji greeter + reboot, baru integrasikan ke `rakuos-bootstrap`.
6. Publikasikan ke COPR.

## Yang belum diverifikasi

- Ukuran akhir SRPM/BRPM dan waktu build di COPR.
- Isi `labwc-locked` di paket jadi.
- Ketersediaan seluruh `Requires` runtime F45 (baru sebagian).
- Perilaku `singularity-shell` saat dijalankan di bawah greetd (bukan
  `sinty-logind`).
- Apakah `libsingularity-devel` benar-benar butuh `vala-cups` saat runtime, atau
  itu sisa dari build time.

## Keputusan yang harus disepakati

- RPM Fusion sebagai external repo COPR, atau codec jadi `Requires` saja.
- `Requires: gdm` → `Recommends` pada `singularity-session`.
- Baseline: tag `rpm-nobara44-20261008` (v0.1.0) atau master terbaru.
- Prefix RPM `/usr` dengan library privat di `/usr/lib64/singularity`.
- Greeter memakai jalur self-compositor, bukan compositor persisten.

## Kebijakan jejak

- Author/committer hanya `tofan79 <nanasmuda.404@outlook.co.id>`.
- Tanpa referensi tool otomatis.
- Dokumentasi cukup di `README.md`, `ui.md`, dan `PLAN.md`.
