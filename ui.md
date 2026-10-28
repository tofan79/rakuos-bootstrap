# UI — rakuos-bootstrap

Tampilan wizard pasca-install RakuOS minimal. TUI murni: tidak ada GTK/Qt,
tidak ada mouse, dan tidak butuh font khusus — karena ISO-nya sendiri TTY-only.

Digambar oleh `src/ui.rs` di atas crate `console` (tanpa `dialoguer`). Modul
itu mendeteksi kemampuan terminal sekali di awal, lalu memilih charset dan
warna yang cocok.

## Cara melihat

File ini teks biasa. Di editor/GitHub tampil apa adanya, di terminal bisa:

```bash
glow ui.md
```

Blok di bawah ditulis untuk lebar 80–100 kolom.

## Charset

| Mode | Kapan | Kotak | Penanda |
|---|---|---|---|
| Full | terminal emulator UTF-8 | `╭─╮` | `✔ ● ○ ❯` |
| Tty | `TERM=linux` (console VT) | `┌─┐` | `[x] [>] [ ]` |
| Ascii | locale non-UTF-8, `--ascii`, atau `RAKUOS_ASCII=1` | `+-+` | `[x] [>] [ ]` |

Teks tetap aman walau locale `C`: yang berubah hanya glyph.

## Warna

| Mode | Kapan | Aksen |
|---|---|---|
| 256 | `COLORTERM`/`TERM=*256*` | oranye bara (208), hijau 108, abu 245 |
| 16 | console VT | kuning (cokelat VGA), hijau, hitam terang |
| None | bukan TTY / `NO_COLOR` | polos, tanpa escape |

Warna mati otomatis kalau output di-pipe, jadi log build tetap bersih.

Tombol: `↑`/`↓` atau `j`/`k` pindah, `1`–`9` loncat, `Enter` pilih, `Esc`
batal.

## Alur

```
     ┌──────────────────────────────────────────┐
     │ 1 Terminal    2 File manager             │
     │ 3 Browser     4 Login     5 Desktop      │
     └───────────────────┬──────────────────────┘
                         ▼
                ringkasan + konfirmasi
                   │              │
                tidak            ya
                   │              ▼
                   │       rum install (overlay)
                   │       salin skel ke ~/.config
                   │       enable login / autologin
                   │       tulis bootstrap.toml + .done
                   │              ▼
                   │       Selesai → tanya reboot
                   ▼
      "Dibatalkan. Tidak ada yang diubah." (exit 130)
```

---

## 1. Terminal

```
 ╭────────────────────────────────────────────────────────────────────────────╮
 │ RAKUOS bootstrap                                                    v0.1.0 │
 │ Menghidupkan sistem minimal sampai punya GUI                       DRY-RUN │
 ╰────────────────────────────────────────────────────────────────────────────╯

 ● Terminal ── ○ File mgr ── ○ Browser ── ○ Login ── ○ Desktop

 Fase 1 dari 5 · Terminal
 Tempat mengetik perintah
 ────────────────────────────────────────────────────────────────────────────
 ❯ Kitty                GPU-accelerated, paling umum dipakai di Hyprland
   Foot                 Ringan, Wayland-only, cocok untuk sistem minimal
   Alacritty            GPU-accelerated, konfigurasi YAML sederhana
   Ghostty              (belum tersedia) Belum ada di Fedora 45 (butuh COPR…)

 ↑↓ pilih   Enter lanjut   1-9 loncat   Esc batal
```

Entri `(belum tersedia)` tetap terlihat supaya user tahu pilihannya ada tapi
belum bisa dipakai. Kalau dipilih lalu `Enter`, baris kuning muncul dan daftar
tidak maju:

```
   Ghostty              (belum tersedia) Belum ada di Fedora 45 (butuh COPR…)
 ❯ Ghostty              (belum tersedia) Belum ada di Fedora 45 (butuh COPR…)

 Ghostty belum tersedia; pilih yang lain.
```

Di bagian bawah layar, nama pilihan sebelumnya dirangkai sebagai jejak:

```
 Kitty  ·  Nautilus  ·  Firefox (RPM)  ·  greetd + tuigreet
```

## 2. File manager

```
 ● Terminal ── ● File mgr ── ○ Browser ── ○ Login ── ○ Desktop

 Fase 2 dari 5 · File manager
 Menjelajah file dan mount USB/HP
 ────────────────────────────────────────────────────────────────────────────
 ❯ Nautilus             File manager GNOME, ringan untuk pemakaian dasar
   Dolphin              File manager KDE, fitur lengkap (menarik Qt/KDE stack)
   Thunar               File manager Xfce, ringan dan cepat
```

## 3. Browser

```
 Fase 3 dari 5 · Browser
 Untuk web; semua pilihan berupa RPM
 ────────────────────────────────────────────────────────────────────────────
 ❯ Firefox (RPM)        Native RPM lewat overlay, tanpa sandbox Flatpak
   Chromium (RPM)       Native RPM, tanpa sinkronisasi akun Google
   Brave (RPM)          Dari repo RakuOS, pemblokir iklan bawaan
```

## 4. Login manager

```
 Fase 4 dari 5 · Login manager
 Layar masuk setelah boot
 ────────────────────────────────────────────────────────────────────────────
 ❯ greetd + tuigreet   Greeter TUI paling ringan, cocok untuk Wayland minimal
   SDDM                 Display manager grafis, paling familiar
   Ly                   Display manager TUI, ringan
   Tanpa login manager  Autologin user langsung ke desktop di tty1
```

## 5. Desktop

```
 Fase 5 dari 5 · Desktop
 Compositor Wayland; config minimal tanpa bar
 ────────────────────────────────────────────────────────────────────────────
 ❯ Hyprland             Tiling dinamis, config minimal tanpa bar
   Niri                 Scrollable tiling, config minimal tanpa bar
   Labwc                Stacking compositor ringan (ala Openbox), tanpa con…
   MangoWM              (belum tersedia) Belum dipaketkan untuk Fedora/RakuOS
```

---

## 6. Ringkasan

Semua langkah berubah jadi `✔`, lalu:

```
 ✔ Terminal ── ✔ File mgr ── ✔ Browser ── ✔ Login ── ✔ Desktop

 Ringkasan
 ────────────────────────────────────────────────────────────────────────────
 Terminal       Kitty
 File manager   Nautilus
 Browser        Firefox (RPM)
 Login          greetd + tuigreet
 Desktop        Hyprland
 User target    mindset
 Paket RPM (25)
   dbus-tools, default-fonts, firefox, fuzzel, google-noto-color-emoji-fonts
   google-noto-sans-fonts, greetd, gvfs, gvfs-mtp, hyprland, … +15 lainnya

 ? Lanjutkan instalasi?

 Y/Enter lanjut   n/Esc batal   Ctrl-C keluar
```

Daftar paket dipadatkan: maksimal 2 baris, sisanya `… +N lainnya`.

`n` atau `Esc` → keluar tanpa mengubah apa pun.

## 7. Proses instalasi

Layar dibersihkan, header + stepper tetap, lalu log mengalir (tidak ada
progress bar — output `rum`/`dnf` yang tampil apa adanya):

```
 ╭────────────────────────────────────────────────────────────────────────────╮
 │ RAKUOS bootstrap                                                    v0.1.0 │
 ╰────────────────────────────────────────────────────────────────────────────╯

 ✔ Terminal ── ✔ File mgr ── ✔ Browser ── ✔ Login ── ✔ Desktop

Memasang paket
  → rum install -y dbus-tools default-fonts firefox fuzzel ...

Menyiapkan config desktop
  → salin /usr/share/rakuos/bootstrap/skel/hyprland -> /home/mindset

Menyiapkan login manager
  → systemctl enable greetd.service
  → systemctl set-default graphical.target
```

Kalau `rum` tidak ada:

```
peringatan: rum tidak ditemukan; memakai dnf (overlay RakuOS tidak aktif)
```

## 8. Selesai

```
 ✔ Selesai.

 ? Reboot sekarang? [Y/n]
```

Jawaban ditulis di baris yang sama (`ya`/`tidak`). Kalau `tidak`:

```
 Reboot nanti dengan: systemctl reboot
```

---

## Varian

### `--dry-run`

Badge `DRY-RUN` di kanan atas. Semua perintah tampil dengan `+`, tidak ada
yang dijalankan:

```
Memasang paket
  + rum install -y alacritty dbus-tools default-fonts firefox ...

Menyiapkan config desktop
  + salin data/skel/hyprland -> /home/mindset
  + ganti @TERMINAL@ dengan alacritty

Menyiapkan login manager
  + systemctl enable greetd.service
  + systemctl set-default graphical.target

✔ Selesai.
(dry-run: tidak ada paket yang dipasang, tidak ada file yang ditulis)
```

### `--first-boot`

Badge `BOOT PERTAMA`, dipanggil unit systemd, langsung reboot setelah selesai
tanpa tanya.

### `--ascii` / `RAKUOS_ASCII=1` / locale non-UTF-8

```
 +----------------------------------------------------------------------------+
 | RAKUOS bootstrap                                                    v0.1.0 |
 | Menghidupkan sistem minimal sampai punya GUI                       DRY-RUN |
 +----------------------------------------------------------------------------+

 [>] Terminal -- [ ] File mgr -- [ ] Browser -- [ ] Login -- [ ] Desktop

 Fase 1 dari 5 - Terminal
 Tempat mengetik perintah
 ----------------------------------------------------------------------------
 > Kitty                GPU-accelerated, paling umum dipakai di Hyprland
   Foot                 Ringan, Wayland-only, cocok untuk sistem minimal
```

### Bukan root

```
error: jalankan dengan sudo: sudo rakuos-bootstrap
```

### Bukan TTY

```
error: butuh terminal interaktif (TTY)
```

### Kategori kosong

```
error: tidak ada pilihan tersedia untuk Terminal
```

### `Esc` / `Ctrl-C`

```
Dibatalkan. Tidak ada yang diubah.
```

Exit code `130`, tidak ada marker `.done` yang ditulis, jadi wizard ditawarkan
lagi saat boot berikutnya.

---

## Batasan

- Lebar efektif 40–78 kolom (diturunkan dari lebar terminal, dikurangi 2).
- Di bawah 72 kolom, stepper hanya menampilkan nama fase yang aktif.
- Deskripsi panjang dipotong dengan `…`; nama pilihan tidak pernah dipotong.
- Kursor disembunyikan selama layar interaktif dan dipulihkan saat keluar,
  termasuk saat `Esc`/error.
- Tidak ada alternate screen: setelah keluar, frame terakhir tetap di
  scrollback, jadi log installer masih bisa dibaca.

## Status implementasi

| Bagian | Status |
|---|---|
| Header, stepper, 5 fase, ringkasan, konfirmasi | ✅ `src/ui.rs` + `src/main.rs` |
| Entri `(belum tersedia)` tampil & ditolak | ✅ |
| Charset Full/Tty/Ascii + `--ascii` | ✅ |
| Warna 256/16/none, hormati `NO_COLOR` | ✅ |
| `Esc`/`Ctrl-C` → exit 130 tanpa menulis | ✅ |
| `--dry-run`, `--first-boot` badge | ✅ |
| Progres install dari output `rum`/`dnf` | ✅ `src/install.rs` |
| WezTerm di katalog | ❌ belum ada (Ghostty sudah, `available = false`) |
