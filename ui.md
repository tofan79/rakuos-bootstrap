# UI — rakuos-bootstrap

Mockup tampilan wizard pasca-install RakuOS minimal. Semuanya TUI (terminal),
tanpa mouse, tanpa GTK/Qt — karena ISO-nya sendiri TTY-only.

Dokumen ini adalah **target tampilan**. Status implementasi per bagian ada di
bagian terakhir.

## Cara melihat

File ini cuma teks, jadi tampil apa adanya di editor, GitHub, atau:

```bash
glow ui.md          # render markdown di terminal
bat ui.md           # syntax highlight, tanpa render
```

Semua blok di bawah ditulis untuk terminal **80 kolom**. Di layar lebih sempit,
deskripsi akan terpotong (lihat bagian "Batasan").

## Legenda

| Simbol | Arti | Warna (ColorfulTheme) |
|---|---|---|
| `?` | pertanyaan aktif | hijau |
| `›` | akhir baris prompt | hijau tebal |
| `❯` | kursor pilihan | cyan tebal |
| `✔` | langkah selesai | hijau |
| `·` | pemisah prompt ↔ jawaban | hijau |
| teks redup | deskripsi pilihan | abu-abu |
| `(belum tersedia)` | entri tanpa paket | kuning redup |
| `→` | perintah yang dijalankan | cyan |
| `+` | aksi dry-run | redup |

Tombol: `↑`/`↓` pindah, `Enter` pilih, `Ctrl-C` batal.

## Alur

```
        ┌──────────────────────────────┐
        │ banner + 5 pertanyaan        │
        │ 1 terminal  2 browser        │
        │ 3 desktop   4 file manager   │
        │ 5 login manager              │
        └──────────────┬───────────────┘
                       ▼
              ringkasan + konfirmasi
                 │            │
              tidak        ya
                 │            ▼
                 │     rum install (overlay)
                 │     flatpak install (opsional)
                 │     salin skel ke ~/.config
                 │     enable login manager / autologin
                 │     tulis /var/lib/rakuos/bootstrap.toml + .done
                 │            ▼
                 │     Selesai → tawaran reboot
                 ▼
        "Dibatalkan. Tidak ada yang diubah."
```

---

## 1. Banner + Terminal

```
$ sudo rakuos-bootstrap

RakuOS bootstrap v0.1.0
Pilih 5 hal dasar; sisanya bisa diubah kapan saja dengan `sudo rakuos-bootstrap`.

? Terminal ›
❯ Alacritty            GPU-accelerated, konfigurasi YAML sederhana
  Kitty                GPU-accelerated, paling umum dipakai di Hyprland
  Ghostty              (belum tersedia) Belum ada di Fedora 45 (butuh COPR)
  WezTerm              (belum tersedia) Belum ada di Fedora 45 (butuh COPR)
  Foot                 Ringan, Wayland-only, cocok untuk sistem minimal

  ↑↓ to move, enter to select
```

Memilih entri `(belum tersedia)` tidak melanjutkan wizard; baris kuning muncul
dan daftar ditampilkan lagi:

```
? Terminal ›
  Alacritty            GPU-accelerated, konfigurasi YAML sederhana
  Kitty                GPU-accelerated, paling umum dipakai di Hyprland
❯ Ghostty              (belum tersedia) Belum ada di Fedora 45 (butuh COPR)
  WezTerm              (belum tersedia) Belum ada di Fedora 45 (butuh COPR)
  Foot                 Ringan, Wayland-only, cocok untuk sistem minimal

Ghostty belum tersedia; pilih yang lain.
```

Setelah dipilih, dialoguer mengganti baris prompt dengan versi ringkas:

```
✔ Terminal · Alacritty
```

## 2. Browser

```
? Browser ›
❯ Firefox (RPM)        Native RPM lewat overlay, tanpa Flatpak
  Firefox (Flatpak)    Terisolasi, update otomatis, unduhan pertama besar
  Chromium (RPM)       Native RPM, tanpa sinkronisasi akun Google
  Brave (RPM)          Dari repo RakuOS, pemblokir iklan bawaan
```

Kalau memilih Flatpak, ringkasan nanti menambah baris `Flatpak` dan catatan
unduhan besar.

## 3. Desktop

```
✔ Browser · Firefox (RPM)

? Desktop ›
❯ Hyprland             Tiling dinamis, config minimal tanpa bar
  Niri                 Scrollable tiling, config minimal tanpa bar
  Labwc                Stacking compositor ringan (ala Openbox), tanpa config tambahan
  MangoWM              (belum tersedia) Belum dipaketkan untuk Fedora/RakuOS
```

## 4. File manager

```
✔ Desktop · Hyprland

? File manager ›
❯ Nautilus             File manager GNOME, ringan untuk pemakaian dasar
  Dolphin              File manager KDE, fitur lengkap (menarik Qt/KDE stack)
  Thunar               File manager Xfce, ringan dan cepat
```

## 5. Login manager

```
✔ File manager · Nautilus

? Login manager ›
❯ greetd + tuigreet   Greeter TUI paling ringan, cocok untuk Wayland minimal
  SDDM                 Display manager grafis, paling familiar
  Ly                   Display manager TUI, ringan
  Tanpa login manager  Autologin user langsung ke desktop di tty1
```

---

## 6. Ringkasan

```
✔ Login manager · greetd + tuigreet

Ringkasan
  Terminal      Alacritty
  Browser       Firefox (RPM)
  Desktop       Hyprland
  File manager  Nautilus
  Login         greetd + tuigreet
  User target   mindset

  Paket RPM (25):
    - dbus-tools
    - default-fonts
    - firefox
    - fuzzel
    - google-noto-color-emoji-fonts
    - google-noto-sans-fonts
    - greetd
    - gvfs
    - gvfs-mtp
    - hyprland
    - hyprland-uwsm
    - kitty
    - libva
    - lxpolkit
    - mesa-dri-drivers
    - mesa-vulkan-drivers
    - nautilus
    - pipewire
    - pipewire-pulseaudio
    - polkit
    - tuigreet
    - wireplumber
    - xdg-desktop-portal
    - xdg-desktop-portal-gtk
    - xdg-desktop-portal-hyprland

? Lanjutkan instalasi? (Y/n) ›
```

Jawaban `n` → `Dibatalkan. Tidak ada yang diubah.` dan keluar dengan kode 0.

## 7. Proses instalasi

```
✔ Lanjutkan instalasi? · yes

Memasang paket
  → rum install -y dbus-tools default-fonts firefox fuzzel ... xdg-desktop-portal-hyprland
Package                 Arch   Version         Repository        Size
Installing:
 alacritty              x86_64 0.16.1-1.fc45   updates       6.2 MiB
 ...
Complete!

Menyiapkan config desktop
  → salin /usr/share/rakuos/bootstrap/skel/hyprland -> /home/mindset

Menyiapkan login manager
  → systemctl enable greetd.service
  → systemctl set-default graphical.target
```

Kalau `rum` tidak ada (image tanpa overlay RakuOS), muncul peringatan kuning
sekali, lalu semua langkah memakai `dnf`:

```
peringatan: rum tidak ditemukan; memakai dnf (overlay RakuOS tidak aktif)
```

## 8. Selesai

```
Selesai.

? Reboot sekarang? (Y/n) ›
```

`y` → `systemctl reboot`. `n` →

```
Reboot nanti dengan: systemctl reboot
```

---

## Varian

### `--dry-run`

Tidak menulis apa pun, hanya menunjukkan rencana. Cocok untuk demo/screenshot:

```
Mode dry-run: hanya menampilkan rencana.

...

Memasang paket
  + rum install -y alacritty dbus-tools default-fonts firefox ...

Menyiapkan config desktop
  + salin /usr/share/rakuos/bootstrap/skel/hyprland -> /home/mindset
  + ganti @TERMINAL@ dengan alacritty

Menyiapkan login manager
  + systemctl enable greetd.service
  + systemctl set-default graphical.target

Selesai.
(dry-run: tidak ada paket yang dipasang, tidak ada file yang ditulis)
```

### `--first-boot`

Dipanggil unit systemd saat boot pertama, jadi tanpa tawaran reboot:

```
Mode boot pertama: wizard berjalan di tty1.
...
Sistem akan reboot untuk masuk ke desktop...
```

### Bukan root

```
error: jalankan dengan sudo: sudo rakuos-bootstrap
```

### Kategori kosong

Kalau semua entri di satu kategori `available = false`:

```
error: tidak ada pilihan tersedia untuk Terminal
```

### `Ctrl-C` / `Esc`

dialoguer mengembalikan error, jadi keluar dengan:

```
error: ...
```

Tidak ada marker `.done` yang ditulis, sehingga wizard ditawarkan lagi saat
boot berikutnya.

---

## Batasan

- Lebar minimum 80 kolom. Di bawah itu deskripsi terpotong; yang dipotong
  hanya teks redup, nama pilihan tidak pernah dipotong.
- Nama pilihan dipadatkan ke 20 kolom, lalu deskripsi mengikuti.
- Tidak butuh font khusus: hanya ASCII + `❯ ✔ · › →`, yang ada di hampir
  semua font monospace. Kalau `LANG=C` dan terminal tidak mendukung Unicode,
  `console` tetap menampilkan teksnya (kotak-kotak).
- Warna otomatis mati kalau output bukan TTY (mis. `| tee`), jadi log build
  tetap bersih.

## Status implementasi

| Bagian | Status |
|---|---|
| Banner, 5 pertanyaan, ringkasan, konfirmasi | ✅ ada di `src/main.rs` |
| Progres install + skel + login + marker | ✅ ada di `src/install.rs` |
| `--dry-run`, `--first-boot` | ✅ |
| Entri `(belum tersedia)` ditampilkan & ditolak | ❌ masih disembunyikan — `ask()` memfilter `available` |
| WezTerm di katalog | ❌ belum ada di `data/catalog.toml` (Ghostty sudah, `available = false`) |
| Screenshot ANSI asli | ⏳ belum; blok di atas adalah target |

Dua baris ❌ itu yang membuat tampilan sekarang hanya berisi 3 terminal
(Alacritty/Kitty/Foot) dan 3 desktop (Hyprland/Niri/Labwc).
