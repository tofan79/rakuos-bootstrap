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

- Upstream `master` sedang 1.x, sementara `kineticwe-2.0` benar-benar ada
  (detached head yang kita ambil sekarang, commit 4018f57652 “Update
  build-deb-stable.sh”). Branch tersebut sudah punya:
  - subpackage `kineticwe-noctalia` (fork Noctalia yang di-build in-tree),
  - `kineticwe-greeter` (pinned subpackage),
  - `xdg-desktop-portal-kwe` (backend kwe),
  - `kineticwe-shell` systemd unit,
  - `kineticwe-noctalia` binary.
  Sehingga kinetcwe 2.0 sudah ada sebagai nyata, bukan plan.
- Audit 107 dep: 107/107.
- Contoh revisi council: penting memakai 2.0 karna integrasi Noctalia greeter + shell
  otomatis (tidak lagi konflik novearity).


```text
specs/kineticwe.spec
workflows/kineticwe.yml
data/kineticwe/                  # skel/config otomatis (opsional)
```

Driver:
- `lgl-kineticwe-copr/kineticwe.spec` → salin/adapt ke Fedora 45.
- SOURCES: `kglobalacceld-*`, `kconfig-*`, `kdecoration-*`,
  `noctalia-greeter-*` dipin sesuai `%global` ref.
- `scripts/start-kineticwe.sh` → RPM-safe text (no user prefix).
- `kineticwe-setup-greeter.sh` dari upstream.

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