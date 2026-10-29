# Plan: Paket RPM Singularity Desktop di Mindset-Apps

## Status singkat

- Target repositori:
  - `singularity-desktop` (`master`, commit `6427e9c`)
  - `singularity-shell` (`main`, commit `57ef177`)
  - `sinty-logind` (`main`, commit `b5ea5c8`)
  - `packaging` snapshot `rpm-nobara44-20261008`
- Target build: Fedora 45 x86_64 melalui COPR `mindset/Mindset-Apps`.
- Status upstream:
  - Desktop sudah punya source RPM lengkap: 89 RPM binary + 4 SRPM, 325 tes meson lulus, verifikasi native selesai.
- Keputusan maturitas:
  - **Singularity Desktop**: **GO**, tetapi sebagai paket sumber monolit besar, bukan build per aplikasi.
  - **Login manager Singularity**: **GO dengan helper terpisah**, karena ia adalah greeter di atas greetd, bukan service `sinty-logind`.
  - **sinty-logind**: **NO-GO** untuk Fedora/RakuOS normal. Ia mengganti `systemd-logind`, `seatd`, dan melayani `org.freedesktop.login1` sendiri pada sistem tanpa systemd.

## Pemisahan yang harus dipahami

1. **Singularity Greeter / session**: bisa dipakai pada Fedora sebagai greeter greetd.
2. **sinty-logind**: komponen inti SintyOS, bukan kandidat RPM Fedora.
3. **Singularity Desktop**: cocok untuk COPR, tetapi mahal, lambat, dan perlu chroot penuh.
4. **GDM tetap default** pada RPM session upstream (`Requires: gdm`).

## Strategi umum

1. Gunakan `singularityos-lab/packaging/rpm` sebagai baseline, bukan build dari nol.
2. Jaga spec desktop sebagai paket monolit agar tetap sesuai baseline upstream.
3. Semua migrasi Fedora 45 dilakukan di repositori Mindset-Apps.
4. `vala-cups`, `gstreamer1-plugin-voaacenc`, dan `librespot` ditangani bila chroot tidak menyediakannya.
5. `sinty-logind` tidak menjadi pilihan login di bootstrap dan tidak menjadi paket default.

## Struktur target

```text
specs/
  singularity-desktop-fedora45.spec
  singularity-greeter-setup.spec
data/singularity/
  source-manifest-baseline.txt
workflows/
  singularity-desktop.yml
  singularity-greeter-setup.yml
```

## Tahap 1 — Import baseline

1. Unduh source RPM:
   - `singularity-desktop-0.1.0-1.fc44.src.rpm`
   - `vala-cups-20240428-1.fc44.src.rpm`
   - `gstreamer1-plugin-voaacenc-1.28.6-1.fc44.src.rpm`
   - `librespot-0.8.0-1.fc44.src.rpm`

2. Ekstrak baseline:
   ```sh
   rpm -i --define "_topdir $(pwd)/rpm" ./rpm/SRPMS/*.src.rpm
   ```

3. Salin spec + `SOURCES/` ke Mindset-Apps.

4. Audit:
   - `__requires_exclude`
   - `__provides_exclude_from`
   - dependency `fprint-2-tod` dan `gusb`

### Checkpoint
- [ ] Spec lolos `rpmlint`.
- [ ] `Source0..13` terverifikasi.
- [ ] Patch native (drag icon, accessible text, runtime compat) ada.

## Tahap 2 — Migrasi Fedora 45

1. Build pertama di VM Fedora 45 x86_64.
2. Lanjut:
   ```sh
   dnf builddep specs/singularity-desktop-fedora45.spec
   rpmbuild --rebuild ...
   ```
3. Fokus ketergantungan:
   - `gtk4 >= 4.22`
   - `libsfdo*`
   - `wayland-protocols >= 1.39`
   - `libdrm >= 2.4.129`
   - `pixman-1 >= 0.46.0`
   - `hwdata-devel`
4. Remap paket:
   - `gstreamer1-plugins-ugly` / `bad-freeworld` → RPM Fusion
   - `vala-cups` → import kalau belum ada
5. Aktifkan `%check` asli setelah build passing.
6. Sync chroot COPR Fedora 45 (`fedora-45-x86_64`).

### Checkpoint
- [ ] `meson compile` lulus.
- [ ] `rpmbuild --rebuild` lulus.
- [ ] GDM menampilkan Singularity.
- [ ] 1-2 aplikasi essential lolos.

## Tahap 3 — Paket greeter Singularity

Rancangan paket:
- `singularity-greeter-setup` install helper + dependency `greetd`.
- `greetd` tetap owner `/etc/greetd/config.toml`.
- user `greetd` dibuat dari helper.
- helper menulis:
  - `/etc/greetd/greeter-session`
  - `/etc/greetd/start-greeter`
  - `/etc/greetd/config.toml`
- disable display manager lain secara eksplisit (GDM, SDDM, Plasma Login, Ly, LightDM).
- Mode preview aman (setup tapi tidak enable).

Integrasi `rakuos-bootstrap` (hanya setelah stabil):
```toml
[[logins]]
id = "singularity-greeter"
name = "Singularity Greeter"
description = "Greeter Singularity (greetd, labwc)"
packages = ["greetd", "singularity-greeter", "singularity-greeter-setup"]
unit = "greetd.service"
greetd_cmd = "/etc/greetd/start-greeter"
greetd_user = "greeter"
```

Catatan:
- Jangan jalankan `dms-greeter sync`.
- Setup greeter bolet dijalankan manual dari fase login.
- Login command memakai compositor passing `@COMPOSITOR@`.

## Tahap 4 — `sinty-logind`: NO-GO

Keputusan tegas:
- tidak masuk `login` choices.
- tidak menggantikan `systemd-logind`.
- tidak install service login1 pada Fedora.
- bila dibatasi, jadilah unit preview:
  - build+test saja,
  - `systemctl is-active systemd-logind` tetap jalan,
  - no-run no-enable.

## Workflow Otomatisasi

Baseline monolitik → pola simple auto-upstream rilis kurang cocok. Rekomendasi:
- build manual/nightly dari release baseline.
- tag build berdasarkan `rpm-nobara44-YYYYMMDD`.
- smart skip berdasarkan tag baseline.
- jangan auto-update terlampau sering (>89 RPM).
- user install `dnf copr enable mindset/Mindset-Apps`.

## Risiko utama

1. Dependency eksternal tidak tersedia di Fedora 45.
2. GStreamer + codecs.
3. `libfprint-TOD` / `libgusb` patch/path.
4. `glibc-compat` dan arch path.
5. ukuran SRPM besar.
6. Prefix meson (`/usr` vs `/opt/local`) memengaruhi skel rakuos-bootstrap.

## Testing

1. Smoke: GDM → Singularity; shell/common.
2. Greeter: config greetd + user greetd + reboot test.
3. Multi-desktop: Hyprland/Niri/Mango/KineticWE dengan setup script.
4. No-go: `systemctl is-active systemd-logind`.
5. Preview tests helper dan cgroup/login1/pam hanya bila preview.

## Urutan setelah rakuos-bootstrap stabil

1. Ambil baseline upstream.
2. Rebuild lokal.
3. Migrasi sukses ke Fedora 45.
4. Cabang greeter setup.
5. Integrasi bootstrap.
6. Publikasikan COPR.

## Keputusan yang harus disepakati

- `sinty-logind` hanya preview.
- default login tetap GDM.
- prefix RPM `/usr`.
- baseline v0.1.0 atau commit master terbaru.

## Kebijakan jejak

- Commit/pesan author/committer harus hanya:
  - `tofan79 <nanasmuda.404@outlook.co.id>`
- tanpa referensi tool AI.
- dokumentasi cukup di `README.md`, `ui.md`, dan `PLAN.md` yang sesuai repo.
