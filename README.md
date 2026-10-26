# rakuos-bootstrap

Wizard pasca-install untuk RakuOS minimal: memilih terminal, browser, desktop,
file manager, dan login manager, lalu memasangnya di atas base image (overlay
`rum`), menyalin config minimal, dan menyiapkan sesi GUI.

Bukan installer OS. `rakuos-installer-cli` yang menulis sistem ke disk; program
ini yang membuat sistem minimal itu **bisa dipakai**.

- Rencana lengkap + checklist eksekusi: [`PLAN.md`](PLAN.md)
- Mockup tampilan TUI: [`ui.md`](ui.md)
- Katalog pilihan: [`data/catalog.toml`](data/catalog.toml)
- Binary: `rakuos-bootstrap` (Rust, TUI `dialoguer`, tanpa dependensi GUI)

## Kenapa bukan `rakuos-setup` / `rakuos-install-dasar`

- `rakuos-setup.service` **sudah dipakai** oleh `rakuos-core` (menjalankan
  `/usr/libexec/rakuos/system-setup` saat boot). Nama itu akan tabrakan.
- `rakuos-install-dasar` terdengar seperti installer OS, padahal ini
  pasca-install.

`rakuos-bootstrap` = "menghidupkan sistem minimal sampai punya GUI".

## Status

Scaffold yang sudah bisa dikompilasi dan dijalankan dengan `--dry-run`.
Belum dipaketkan sebagai RPM (menyusul di `Mindset-Apps`).
