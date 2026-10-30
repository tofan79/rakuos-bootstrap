//! Log ke file (default `/var/log/rakuos-bootstrap.log`).
//!
//! Mode `--first-boot` jalan di tty1 lalu reboot, jadi layar tidak bisa
//! dibaca ulang. Semua perintah beserta keluarannya, peringatan, dan hasil
//! verifikasi ditulis ke sini. Kegagalan membuka file tidak fatal.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static LOG: OnceLock<Mutex<File>> = OnceLock::new();
static PATH: OnceLock<String> = OnceLock::new();

pub const DEFAULT_PATH: &str = "/var/log/rakuos-bootstrap.log";

/// Lokasi log untuk pesan galat; "log wizard" kalau log tidak aktif.
pub fn hint() -> &'static str {
    PATH.get().map(String::as_str).unwrap_or("log wizard")
}

/// Buka file log (append). Mengembalikan pesan galat kalau gagal.
pub fn init(path: &Path, stamp: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("tidak bisa membuka log {}: {e}", path.display()))?;
    let _ = writeln!(file, "\n==== rakuos-bootstrap {stamp} ====");
    let _ = PATH.set(path.display().to_string());
    let _ = LOG.set(Mutex::new(file));
    Ok(())
}

/// Tulis satu baris ke log (kalau log aktif).
pub fn note(text: &str) {
    if let Some(lock) = LOG.get() {
        if let Ok(mut f) = lock.lock() {
            let _ = writeln!(f, "{text}");
        }
    }
}

/// Mengubah aliran byte keluaran perintah jadi baris log yang bersih:
/// tanpa kode warna ANSI, dan progress bar (`\r`) hanya menyisakan versi
/// terakhir tiap baris.
#[derive(Default)]
pub struct LineLogger {
    buf: Vec<u8>,
}

impl LineLogger {
    pub fn feed(&mut self, chunk: &[u8]) {
        for &b in chunk {
            if b == b'\n' {
                self.flush_line();
            } else {
                self.buf.push(b);
            }
        }
    }

    pub fn finish(&mut self) {
        self.flush_line();
    }

    fn flush_line(&mut self) {
        let raw = String::from_utf8_lossy(&self.buf).to_string();
        self.buf.clear();
        let last = raw.rsplit('\r').find(|p| !p.is_empty()).unwrap_or("");
        let clean = strip_ansi(last);
        if !clean.trim().is_empty() {
            note(&format!("  {clean}"));
        }
    }
}

pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
