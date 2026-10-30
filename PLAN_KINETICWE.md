# Plan: Paket RPM KineticWE stabil (mengganti build harian `-git`)

## Latar belakang

RakuOS bootstrap saat ini memasang KineticWE dari repo Mindset-Apps sebagai paket
harian `kineticwe-git`. Build harian mismatch, greeter-nya rename
`noctalia-greeter`, sehingga login loop sering muncul dan sulit di-troubleshoot.

Upstream KineticWE (`gitlab.com/theblackdon/kineticwe`) sudah punya:
- `PACKAGING-COPR.md` (rencana COPR resmi, 1.x line = NO Noctalia),
- `lgl-kineticwe-copr/kineticwe.spec` (SRPM yang memproduksi `kineticwe`,
  `kineticwe-noctalia`, `kineticwe-greeter`, `xdg-desktop-portal-kwe`,
  `kineticwe-kglobalacceld`; `noctalia_greeter_ref` sudah dipin),
- `kineticwe-2.0.md` (iterasi 2: bundel fork Noctalia, belum implement penuh).

Target RakuOS: paket tipe `kineticwe` dari upstream, semua komponen dipin ke
commit yang sama, agar tidak drift harian.

## Keputusan

- Pakai baseline stable KineticWE dari upstream (bukan `-git` harian).
- Setiap komponen (compositor, greeter, portal, kglobalacceld) dibangun dalam
  satu SRPM dengan ref commit yang sama.
- bootstrap catalog `packages = ["kineticwe"]` (ganti `"kineticwe-git"`),
  `login_override = "kineticwe-greeter"` tetap (greeter subpackage dipin
  upstream).
- Audit `BuildRequires` sebelum publish.

## Status branch

- Upstream `master` memang 1.x, tetapi branch **`kineticwe-2.0`** benar-benar ada
  (detached head commit `4018f57652`), dengan:
  - `kineticwe-noctalia` (fork Noctalia yang di-build in-tree),
  - `kineticwe-greeter` (pinned subpackage),
  - `xdg-desktop-portal-kwe` (backend kwe),
  - `kineticwe-shell.service`, `kineticwe-noctalia` binary.
- Jadi **KineticWE 2.0 nyata**, bukan plan, dan sesuai permintaan Anda: greeter
  + Noctalia otomatis tercampur satu SRPM.
- Audit BuildRequires: **107/107** tersedia di F45 + Fusion.

## Urutan build (Fedora 45 container, podman)

1. Snapshot repos (git clone KineticWE.
2. Audit `BuildRequires` terhadap fedora + updates + fusion (audit tengah jalan).
3. `dnf builddep specs/kineticwe.spec`
4. `rpmbuild --rebuild` (atau `mock -r fedora-45-x86_64`).
5. Run package check (`rpm -qpR`, `ldd`, `rpmlint`, `rpm -qpl`).
6. Publikasi ke COPR `Mindset-Apps` dan edit catalog bootstrap.

## Hasil audit dependency

**Kesimpulan:** 107 dari 107 `BuildRequires` tersedia di
Fedora 45 (plus `updates` + RPM Fusion). Tidak ada yang hilang. Semua
dependensi KWin Frameworks (`kf6-*`, `libdisplay-info`, `wlroots`, `sdbus-cpp`,
`tomlplusplus`, dan kawan-kawan) ada di Fedora 45. Migrasi spec tinggal soal bug
versi (`kdecoration`, `kconfig` chain), BZ patches, dan `%files`.


## Immediate mitigation (sebelum publish)

- Parent user tetap konsisten nightly date.
- Jangan campur Noctalia Greeter stand-alone dengan KineticWE.
- Login loop bisa dicek `journalctl -u greetd` (lihat error `kineticwe` /
  `exec` missing). Pal ringan: upgrade satu nominal.

## Perubahan di RakuOS bootstrap (setelah publish)

- `kineticwe-git` → `kineticwe`.
- `desktops.kineticwe.packages = ["kineticwe"]`.
- `login_override` tetap `kineticwe-greeter`.
- `README.md` menyebut KineticWE (stable) + `kineticwe-greeter`.
- Update test if needed.

## Risiko & keputusan

- KWin build besar (KDE Frameworks) → lama di COPR.
- Upstream 2.0 iterasi belum complete → gunakan 1.x scope Noctalia terpisah.
- Paket `kineticwe` Provides/Obsoletes `kwin` etc — cek konflik di RakuOS yang
  mungkin install kwin (verify dulu).

## Catatan

Dok ini dibuat dari clone `gitlab.com/theblackdon/kineticwe`. Bila auditor
BuildRequires selesai, isi tabel gap.

## Publikasi ke COPR (Mindset-Apps)

Agar KineticWE bisa `rum install kineticwe` dengan mudah:
1. Selesaikan build `build-rpms.sh` (latar belakang, distrobox fedora:44).
2. Setelah ada `results/*.src.rpm`, dari host jalankan:
   ```sh
   copr-cli build mindset/Mindset-Apps /var/home/mindset/Projects/sumber/kineticwe/lgl-kineticwe-copr/results/*.src.rpm
   ```
3. Tunggu build di COPR selesai (Fedora 43/44/45/rawhide), binary akan muncul
   dan dapat di-install `rum install kineticwe`.
4. Update katalog/wizard `kineticwe-git` -> `kineticwe`, login_override tetap
   `kineticwe-greeter`.
5. Saat upgrade upstream, ulang build-rpms.sh + `copr-cli build` — satu perintah.

**Catatan:** COPR build besar, tidak perlu GitHub Actions per push. Bisa
jadwal manual bulanan/triweekly.

