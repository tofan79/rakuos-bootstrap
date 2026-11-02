# UI — rakuos-bootstrap

Dokumen ini menjelaskan TUI yang digambar oleh `src/ui.rs` dan alur pilihan
di `src/main.rs`. UI berjalan di terminal teks, termasuk console VT; tidak
memerlukan GTK, Qt, atau mouse.

## Charset dan warna

| Charset | Kondisi | Bingkai dan penanda |
|---|---|---|
| Full | Terminal UTF-8 biasa | `╭─╮`, `✔ ● ○ ❯` |
| Tty | `TERM=linux` dan locale UTF-8 | `┌─┐`, `[x] [>] [ ]` |
| Ascii | Locale bukan UTF-8, `--ascii`, `RAKUOS_ASCII`, atau `TERM=dumb` | `+-+`, `[x] [>] [ ]` |

Warna mengikuti dukungan terminal dan `NO_COLOR`: 256 warna jika tersedia,
warna ANSI 16 untuk terminal lain yang mendukung warna, dan polos jika warna
dinonaktifkan atau output tidak interaktif.

## Tombol

- `↑`/`↓`, `j`/`k`: pindah sorotan; `Tab` juga bergerak ke bawah.
- Daftar jamak: `Space` atau `1`–`9` mencentang pilihan pada posisi itu.
- Daftar tunggal: `1`–`9` memilih pilihan pada posisi itu.
- `Enter`: lanjut jika ada pilihan yang valid.
- `Esc`, `Backspace`, `←`, atau `b`: kembali satu layar.
- `Ctrl-C`: batalkan wizard. Saat berada di langkah pertama, tombol kembali
  menampilkan pesan bahwa `Ctrl-C` digunakan untuk membatalkan.

Kategori Terminal, File manager, dan Browser menerima satu pilihan atau lebih;
minimal satu harus dicentang. Desktop menerima satu pilihan. Login manager
muncul sesudah Desktop hanya untuk pilihan desktop dengan `choose_login = true`.
Saat ringkasan terbuka, `Enter`/`y` melanjutkan, `n` membatalkan, dan tombol
kembali mengembalikan pengguna ke pilihan terakhir.

## Alur layar

```text
Terminal → File manager → Browser → Desktop → [Login manager bila diminta]
                                      ↓
                              Ringkasan dan konfirmasi
                                      ↓
                       pasang paket, siapkan sesi/config,
                          verifikasi, lalu tanya reboot
```

Stepper selalu menyediakan lima label dalam urutan Terminal, File manager,
Browser, Desktop, Login. Login dilewati untuk desktop yang memakai login
bawaannya atau `login_override` di katalog. Pada lebar terminal di bawah 72
kolom, hanya nama fase aktif yang tampil di stepper.

## Contoh layar

Contoh berikut memakai katalog saat ini. Deskripsi dan pilihan berasal dari
`data/catalog.toml`; data katalog bisa berubah.

### Terminal

```text
╭────────────────────────────────────────────────────────────────────────────╮
│ RAKUOS bootstrap                                                    v0.2.4 │
│ Menghidupkan sistem minimal sampai punya GUI                       DRY-RUN │
╰────────────────────────────────────────────────────────────────────────────╯

 ● Terminal ── ○ File manager ── ○ Browser ── ○ Desktop ── ○ Login

 Fase 1 dari 5 · Terminal
 Tempat mengetik perintah
 ────────────────────────────────────────────────────────────────────────────
   [ ] Alacritty        GPU-accelerated, konfigurasi YAML sederhana
   [ ] Foot             Ringan, Wayland-only, cocok untuk sistem minimal
   [ ] Ghostty          Dari repo Terra, diaktifkan otomatis saat instalasi
 ❯ [x] Kitty            GPU-accelerated, paling umum dipakai di Hyprland

 ↑↓ pilih  Space centang  Enter lanjut  Esc/← kembali  Ctrl-C batal
```

`default = true` di katalog menentukan pilihan jamak yang dicentang saat layar
pertama kali dibuka. Urutan pilihan selalu alfabetis. Pilihan yang dicentang
tetap tersimpan ketika pengguna kembali ke layar itu.

### Browser

Daftar bisa lebih panjang daripada tinggi terminal. UI menampilkan jendela
pilihan dan jumlah item tersembunyi di bawah atau di atasnya.

```text
 Fase 3 dari 5 · Browser
 Pilih satu atau lebih; semuanya paket RPM
 ────────────────────────────────────────────────────────────────────────────
   [ ] Brave            Chromium dengan pemblokir iklan bawaan
   [ ] Chromium         Open source, tanpa sinkronisasi akun Google
   [ ] Epiphany         GNOME Web (WebKitGTK), paling ringan
   [ ] Falkon           Qt/Chromium ringan, cocok sistem minimal
 ❯ [x] Firefox          Default, cepat, dan paling kompatibel
   [ ] Google Chrome   Proprietary, sinkronisasi akun Google
   [ ] Helium           Chromium privat dan cepat, dari Terra
   [ ] LibreWolf        Firefox tanpa telemetri, fokus privasi
   [ ] Microsoft Edge  Chromium Microsoft, paket paling besar
   [ ] QuteBrowser      Keyboard-only, Qt/Python
 ↓3 lagi
```

### Desktop

```text
 Fase 4 dari 5 · Desktop
 Compositor Wayland; config minimal tanpa bar
 ────────────────────────────────────────────────────────────────────────────
 ❯ Budgie       Desktop tradisional modern, mode Wayland via Budgie Labwc
   Hyprland     Tiling dinamis, config minimal tanpa bar
   KineticWE    Tiling berbasis KWin, shell Noctalia
   LXQt         Desktop ringan, sesi Wayland lewat Miriway
   MangoWM      Tiling ringan berbasis dwl (wlroots)
   MiracleWM    Tiling Wayland, paket desktop Fedora dengan SDDM
   Sway         Tiling Wayland ringan, config minimal tanpa bar
   Umbriel      Compositor Wayland ringan (paket Mindset-Apps, build git harian)
```

Budgie adalah pilihan desktop default. Sesudah memilih desktop, wizard
menampilkan Login manager untuk Budgie, Hyprland, MangoWM, dan Sway. Pilihan
login yang terlihat saat ini adalah DMS Greeter, Noctalia Greeter, SDDM, dan
sysc-greet; SDDM adalah pilihan awal. Dengan default katalog, hasilnya Budgie
Wayland + SDDM. MiracleWM memakai grup desktop Fedora yang menyertakan SDDM dan
greeter Wayland Miracle; katalog menetapkan SDDM untuk desktop ini tanpa
menampilkan pilihan Login manager. LXQt memakai display manager yang tersedia
bersama desktop, sementara KineticWE dan Umbriel memakai login yang ditetapkan
katalog.

### Ringkasan

Contoh ini menggambarkan isi ringkasan, bukan daftar paket tetap. Jumlah dan
nama paket dihitung dari pilihan, katalog, serta paket dasar.

```text
 ✔ Terminal ── ✔ File manager ── ✔ Browser ── ✔ Desktop ── ✔ Login

 Ringkasan
 ────────────────────────────────────────────────────────────────────────────
 Terminal       Kitty
 File manager   Nautilus
 Browser        Firefox
 Login          SDDM
 Desktop        Hyprland
 User target    mindset
 Paket RPM (…)
   …

 ? Lanjutkan instalasi?

 Y/Enter lanjut  Esc/← kembali  n tidak  Ctrl-C batal
```

Ringkasan membatasi daftar paket hingga dua baris. `n` keluar tanpa
menjalankan instalasi; `Esc`/`←` kembali ke langkah terakhir.

## Saat proses dan selesai

Sesudah konfirmasi, frame pilihan diganti header dan stepper. Perintah beserta
keluarannya tampil dan dicatat ke file log. `--dry-run` menampilkan perintah
dengan awalan `+` dan tidak menjalankan perintah atau menulis file.

Setelah verifikasi berhasil, instalasi biasa menanyakan reboot (`Y/n`);
`--first-boot` langsung meminta reboot. Jika verifikasi gagal, reboot otomatis
ditunda dan lokasi log ditampilkan.

## Pembatalan dan batas tampilan

- `Ctrl-C` menghasilkan pesan pembatalan dan exit code 130.
- `n` pada ringkasan kembali dengan sukses tanpa mengubah sistem.
- Lebar bingkai dibatasi sekitar 40–78 kolom. Deskripsi panjang dipotong,
  sedangkan daftar pilihan dapat digulir di dalam layar.
- Kursor terminal disembunyikan selama layar interaktif lalu dipulihkan.
- UI tidak memakai alternate screen; frame terakhir tetap di scrollback.
