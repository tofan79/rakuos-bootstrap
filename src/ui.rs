//! Tampilan TUI RakuOS: aman di TTY (`TERM=linux`) sampai terminal modern.
//!
//! Tanpa dependensi baru, hanya `console`. Kemampuan terminal dideteksi sekali
//! di `Ui::new`, lalu semua gambar memakai tiga tingkat:
//!
//! | Charset | Kapan                              | Kotak | Penanda      |
//! |---------|------------------------------------|-------|--------------|
//! | Full    | terminal emulator UTF-8            | ╭─╮   | ✔ ● ○ ❯      |
//! | Tty     | `TERM=linux` + UTF-8 (console VT)  | ┌─┐   | [x] [>] [ ]  |
//! | Ascii   | locale bukan UTF-8 / `--ascii`     | +-+   | [x] [>] [ ]  |
//!
//! Warna: 256 (oranye bara) kalau terminal mendukung, 16 warna ANSI di console
//! VT, dan polos kalau output bukan TTY atau `NO_COLOR` aktif.

use std::fmt;
use std::io::Write;

use anyhow::{bail, Result};
use console::{measure_text_width, truncate_str, Key, Style, Term};

use crate::catalog::Choice;

/// Nama pendek tiap fase untuk stepper.
const STEPS: [&str; 5] = ["Terminal", "File mgr", "Browser", "Login", "Desktop"];

/// Pengguna menekan Esc / Ctrl-C.
#[derive(Debug)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dibatalkan")
    }
}

impl std::error::Error for Cancelled {}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ColorLevel {
    None,
    Vt16,
    Xterm256,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Charset {
    Ascii,
    Tty,
    Full,
}

struct Glyphs {
    tl: &'static str,
    tr: &'static str,
    bl: &'static str,
    br: &'static str,
    h: &'static str,
    v: &'static str,
    cursor: &'static str,
    done: &'static str,
    current: &'static str,
    todo: &'static str,
    link: &'static str,
    dot: &'static str,
    ellipsis: &'static str,
    updown: &'static str,
}

impl Glyphs {
    fn for_charset(cs: Charset) -> Self {
        match cs {
            Charset::Full => Glyphs {
                tl: "╭", tr: "╮", bl: "╰", br: "╯", h: "─", v: "│",
                cursor: "❯", done: "✔", current: "●", todo: "○",
                link: "──", dot: "·", ellipsis: "…", updown: "↑↓",
            },
            Charset::Tty => Glyphs {
                tl: "┌", tr: "┐", bl: "└", br: "┘", h: "─", v: "│",
                cursor: ">", done: "[x]", current: "[>]", todo: "[ ]",
                link: "──", dot: "·", ellipsis: "...", updown: "↑↓",
            },
            Charset::Ascii => Glyphs {
                tl: "+", tr: "+", bl: "+", br: "+", h: "-", v: "|",
                cursor: ">", done: "[x]", current: "[>]", todo: "[ ]",
                link: "--", dot: "-", ellipsis: "...", updown: "Up/Down",
            },
        }
    }
}

/// Warna per peran. Ganti di satu tempat ini kalau brand berubah.
struct Palette {
    accent: Style,   // oranye bara: elemen utama
    accent_b: Style, // sama, tebal
    ok: Style,
    warn: Style,
    dim: Style,
    bold: Style,
}

impl Palette {
    fn for_level(level: ColorLevel) -> Self {
        match level {
            ColorLevel::None => Palette {
                accent: Style::new(),
                accent_b: Style::new().bold(),
                ok: Style::new(),
                warn: Style::new(),
                dim: Style::new(),
                bold: Style::new().bold(),
            },
            // Console VT: "yellow" tanpa bold = cokelat/oranye (VGA #AA5500),
            // persis warna bara. Redup = hitam terang (abu-abu tua).
            ColorLevel::Vt16 => Palette {
                accent: Style::new().yellow(),
                accent_b: Style::new().yellow().bold(),
                ok: Style::new().green(),
                warn: Style::new().yellow().bold(),
                dim: Style::new().black().bright(),
                bold: Style::new().white().bold(),
            },
            ColorLevel::Xterm256 => Palette {
                accent: Style::new().color256(208),
                accent_b: Style::new().color256(208).bold(),
                ok: Style::new().color256(108),
                warn: Style::new().color256(220),
                dim: Style::new().color256(245),
                bold: Style::new().bold(),
            },
        }
    }
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn locale_is_utf8() -> bool {
    // Urutan POSIX: LC_ALL > LC_CTYPE > LANG; yang pertama terisi menang.
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        let value = env(key).to_lowercase();
        if !value.is_empty() {
            return value.contains("utf-8") || value.contains("utf8");
        }
    }
    false
}

pub struct Ui {
    pal: Palette,
    g: Glyphs,
    width: usize,
    version: String,
    badge: Option<String>,
}

impl Ui {
    pub fn new(term: &Term, force_ascii: bool, version: &str, badge: Option<String>) -> Self {
        let term_name = env("TERM");
        let is_vt = term_name == "linux";

        let charset = if force_ascii
            || !env("RAKUOS_ASCII").is_empty()
            || term_name == "dumb"
            || !locale_is_utf8()
        {
            Charset::Ascii
        } else if is_vt {
            Charset::Tty
        } else {
            Charset::Full
        };

        let color = if !console::colors_enabled() {
            ColorLevel::None
        } else if is_vt {
            ColorLevel::Vt16
        } else if term_name.contains("256") || !env("COLORTERM").is_empty() {
            ColorLevel::Xterm256
        } else {
            ColorLevel::Vt16
        };

        // Console VT umumnya 80x25. Ambil lebar nyata, tapi jangan melebar
        // lebih dari 78 supaya tetap enak dibaca di terminal lebar.
        let (_, cols) = term.size();
        let width = (cols as usize).saturating_sub(2).clamp(40, 78);

        Ui {
            pal: Palette::for_level(color),
            g: Glyphs::for_charset(charset),
            width,
            version: version.to_string(),
            badge,
        }
    }

    // ------------------------------------------------------------ helpers --

    fn pad(&self, text: &str, width: usize) -> String {
        let w = measure_text_width(text);
        if w >= width {
            text.to_string()
        } else {
            format!("{text}{}", " ".repeat(width - w))
        }
    }

    fn rule(&self, n: usize) -> String {
        self.g.h.repeat(n)
    }

    fn fit(&self, text: &str, max: usize) -> String {
        truncate_str(text, max, self.g.ellipsis).to_string()
    }

    /// Bungkus baris yang sudah berwarna dengan kotak selebar `self.width`.
    fn boxed(&self, lines: &[String]) -> Vec<String> {
        let inner = self.width - 4;
        let edge = &self.pal.accent;
        let mut out = Vec::new();
        out.push(format!(
            " {}",
            edge.apply_to(format!("{}{}{}", self.g.tl, self.rule(self.width - 2), self.g.tr))
        ));
        for line in lines {
            out.push(format!(
                " {} {} {}",
                edge.apply_to(self.g.v),
                self.pad(line, inner),
                edge.apply_to(self.g.v)
            ));
        }
        out.push(format!(
            " {}",
            edge.apply_to(format!("{}{}{}", self.g.bl, self.rule(self.width - 2), self.g.br))
        ));
        out
    }

    fn header(&self) -> Vec<String> {
        let inner = self.width - 4;
        let p = &self.pal;

        let brand = format!(
            "{}{} {}",
            p.accent_b.apply_to("RAKU"),
            p.bold.apply_to("OS"),
            p.dim.apply_to("bootstrap")
        );
        let ver = p.dim.apply_to(format!("v{}", self.version)).to_string();
        let gap = inner.saturating_sub(measure_text_width(&brand) + measure_text_width(&ver));
        let line1 = format!("{brand}{}{ver}", " ".repeat(gap));

        let tagline = p.dim.apply_to("Menghidupkan sistem minimal sampai punya GUI").to_string();
        let line2 = match &self.badge {
            Some(b) => {
                let b = p.warn.apply_to(b).to_string();
                let gap = inner.saturating_sub(measure_text_width(&tagline) + measure_text_width(&b));
                format!("{tagline}{}{b}", " ".repeat(gap))
            }
            None => tagline,
        };

        self.boxed(&[line1, line2])
    }

    /// `phase`: 0..=4 fase aktif, 5 = semua selesai.
    fn stepper(&self, phase: usize) -> String {
        let p = &self.pal;
        let compact = self.width < 72;
        let mut out = String::from(" ");
        for (i, name) in STEPS.iter().enumerate() {
            let (mark, mark_style, name_style) = if phase >= 5 || i < phase {
                (self.g.done, &p.ok, &p.dim)
            } else if i == phase {
                (self.g.current, &p.accent_b, &p.bold)
            } else {
                (self.g.todo, &p.dim, &p.dim)
            };
            out.push_str(&mark_style.apply_to(mark).to_string());
            if !compact || i == phase {
                out.push(' ');
                out.push_str(&name_style.apply_to(name).to_string());
            }
            if i + 1 < STEPS.len() {
                out.push_str(&format!(" {} ", p.dim.apply_to(self.g.link)));
            }
        }
        out
    }

    fn frame(&self, phase: usize, body: Vec<String>, footer: &str) -> Vec<String> {
        let mut lines = vec![String::new()];
        lines.extend(self.header());
        lines.push(String::new());
        lines.push(self.stepper(phase));
        lines.push(String::new());
        lines.extend(body);
        lines.push(String::new());
        lines.push(format!(" {}", self.pal.dim.apply_to(footer)));
        lines
    }

    /// Gambar ulang dari pojok kiri atas tanpa berkedip (tanpa clear penuh).
    fn paint(&self, term: &Term, lines: &[String]) -> Result<()> {
        let mut buf = String::from("\x1b[H");
        for line in lines {
            buf.push_str(line);
            buf.push_str("\x1b[K\n");
        }
        buf.push_str("\x1b[J");
        let mut out = term.clone();
        out.write_all(buf.as_bytes())?;
        out.flush()?;
        Ok(())
    }

    fn footer_select(&self) -> String {
        format!(
            "{} pilih   Enter lanjut   1-9 loncat   Esc batal",
            self.g.updown
        )
    }

    // ------------------------------------------------------------- layar ---

    /// Layar pilihan satu fase. Mengembalikan indeks di `choices`.
    pub fn select(
        &self,
        term: &Term,
        phase: usize,
        title: &str,
        hint: &str,
        choices: &[Choice],
        picked: &[String],
    ) -> Result<usize> {
        if choices.iter().all(|c| !c.available) {
            bail!("tidak ada pilihan tersedia untuk {title}");
        }
        let _guard = CursorGuard::hide(term);
        let mut sel = choices.iter().position(|c| c.available).unwrap_or(0);
        let mut msg: Option<String> = None;

        loop {
            let body = self.select_body(phase, title, hint, choices, sel, picked, msg.as_deref());
            let lines = self.frame(phase, body, &self.footer_select());
            self.paint(term, &lines)?;

            match term.read_key()? {
                Key::ArrowUp | Key::Char('k') => {
                    sel = (sel + choices.len() - 1) % choices.len();
                    msg = None;
                }
                Key::ArrowDown | Key::Char('j') | Key::Tab => {
                    sel = (sel + 1) % choices.len();
                    msg = None;
                }
                Key::Char(c) if c.is_ascii_digit() && c != '0' => {
                    let n = c as usize - '1' as usize;
                    if n < choices.len() {
                        sel = n;
                        msg = None;
                    }
                }
                Key::Enter => {
                    if choices[sel].available {
                        return Ok(sel);
                    }
                    msg = Some(format!("{} belum tersedia; pilih yang lain.", choices[sel].name));
                }
                Key::Escape | Key::CtrlC => return Err(Cancelled.into()),
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn select_body(
        &self,
        phase: usize,
        title: &str,
        hint: &str,
        choices: &[Choice],
        sel: usize,
        picked: &[String],
        msg: Option<&str>,
    ) -> Vec<String> {
        let p = &self.pal;
        let mut body = Vec::new();

        body.push(format!(
            " {} {} {}",
            p.accent_b.apply_to(format!("Fase {} dari {}", phase + 1, STEPS.len())),
            p.dim.apply_to(self.g.dot),
            p.bold.apply_to(title)
        ));
        body.push(format!(" {}", p.dim.apply_to(self.fit(hint, self.width - 2))));
        body.push(format!(" {}", p.dim.apply_to(self.rule(self.width - 2))));

        let name_w = 20;
        let desc_w = self.width.saturating_sub(name_w + 6);
        for (i, c) in choices.iter().enumerate() {
            let active = i == sel;
            let cursor = if active {
                p.accent_b.apply_to(self.g.cursor).to_string()
            } else {
                " ".repeat(measure_text_width(self.g.cursor))
            };
            let (name, desc) = if !c.available {
                (
                    p.dim.apply_to(self.pad(&c.name, name_w)).to_string(),
                    format!(
                        "{} {}",
                        p.warn.apply_to("(belum tersedia)"),
                        p.dim.apply_to(self.fit(&c.description, desc_w.saturating_sub(17)))
                    ),
                )
            } else if active {
                (
                    p.accent_b.apply_to(self.pad(&c.name, name_w)).to_string(),
                    p.bold.apply_to(self.fit(&c.description, desc_w)).to_string(),
                )
            } else {
                (
                    self.pad(&c.name, name_w),
                    p.dim.apply_to(self.fit(&c.description, desc_w)).to_string(),
                )
            };
            body.push(format!(" {cursor} {name} {desc}"));
        }

        body.push(match choices.get(sel) {
            Some(c) if c.available && measure_text_width(&c.description) > desc_w => {
                format!(" {}", p.dim.apply_to(self.fit(&c.description, self.width - 2)))
            }
            _ => String::new(),
        });
        body.push(match msg {
            Some(m) => format!(" {}", p.warn.apply_to(m)),
            None if !picked.is_empty() => format!(
                " {}",
                p.dim.apply_to(self.fit(&picked.join(&format!("  {}  ", self.g.dot)), self.width - 2))
            ),
            None => String::new(),
        });
        body
    }

    /// Layar ringkasan + konfirmasi. `true` = lanjut.
    pub fn confirm_summary(
        &self,
        term: &Term,
        rows: &[(&str, String)],
        packages: &[String],
        flatpak: Option<&str>,
    ) -> Result<bool> {
        let _guard = CursorGuard::hide(term);
        let p = &self.pal;
        let mut body = vec![
            format!(" {}", p.accent_b.apply_to("Ringkasan")),
            format!(" {}", p.dim.apply_to(self.rule(self.width - 2))),
        ];
        for (k, v) in rows {
            body.push(format!(" {} {}", p.dim.apply_to(self.pad(k, 14)), p.bold.apply_to(v)));
        }
        body.push(format!(
            " {}",
            p.dim.apply_to(format!("Paket RPM ({})", packages.len()))
        ));
        for line in self.wrap_list(packages, self.width - 4, 2) {
            body.push(format!("   {}", p.dim.apply_to(line)));
        }
        body.push(match flatpak {
            Some(_) => format!(" {}", p.warn.apply_to("Flatpak: unduhan pertama bisa ratusan MB.")),
            None => String::new(),
        });
        body.push(format!(
            " {} {}",
            p.accent_b.apply_to("?"),
            p.bold.apply_to("Lanjutkan instalasi?")
        ));

        let lines = self.frame(5, body, "Y/Enter lanjut   n/Esc batal   Ctrl-C keluar");
        self.paint(term, &lines)?;
        loop {
            match term.read_key()? {
                Key::Enter | Key::Char('y') | Key::Char('Y') => return Ok(true),
                Key::Char('n') | Key::Char('N') | Key::Escape => return Ok(false),
                Key::CtrlC => return Err(Cancelled.into()),
                _ => {}
            }
        }
    }

    /// Bersihkan layar dan tampilkan header saja; log instalasi mengalir di bawahnya.
    pub fn begin_log(&self, term: &Term) -> Result<()> {
        let mut lines = vec![String::new()];
        lines.extend(self.header());
        lines.push(String::new());
        lines.push(self.stepper(5));
        lines.push(String::new());
        self.paint(term, &lines)
    }

    /// Ya/tidak satu baris tanpa menghapus layar (mis. tawaran reboot).
    pub fn inline_confirm(&self, term: &Term, question: &str, default: bool) -> Result<bool> {
        let p = &self.pal;
        let choice = if default { "Y/n" } else { "y/N" };
        let mut out = term.clone();
        write!(
            out,
            "\n {} {} {} ",
            p.accent_b.apply_to("?"),
            p.bold.apply_to(question),
            p.dim.apply_to(format!("[{choice}]"))
        )?;
        out.flush()?;
        loop {
            let answer = match term.read_key()? {
                Key::Enter => Some(default),
                Key::Char('y') | Key::Char('Y') => Some(true),
                Key::Char('n') | Key::Char('N') | Key::Escape => Some(false),
                Key::CtrlC => return Err(Cancelled.into()),
                _ => None,
            };
            if let Some(a) = answer {
                writeln!(out, "{}", if a { "ya" } else { "tidak" })?;
                return Ok(a);
            }
        }
    }

    pub fn success(&self, text: &str) -> String {
        self.pal.ok.apply_to(format!("{} {text}", self.g.done)).to_string()
    }

    pub fn note(&self, text: &str) -> String {
        self.pal.dim.apply_to(text).to_string()
    }

    /// Rapatkan daftar jadi paling banyak `max_lines` baris; sisanya "+N lainnya".
    fn wrap_list(&self, items: &[String], width: usize, max_lines: usize) -> Vec<String> {
        let mut lines: Vec<Vec<&String>> = vec![Vec::new()];
        let mut used = 0usize;
        for item in items {
            let need = measure_text_width(item) + 2; // ", "
            if used + need > width + 2 && !lines.last().map_or(true, |l| l.is_empty()) {
                lines.push(Vec::new());
                used = 0;
            }
            lines.last_mut().unwrap().push(item);
            used += need;
        }
        let join = |l: &[&String]| l.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        if lines.len() <= max_lines {
            return lines.iter().map(|l| join(l)).collect();
        }

        lines.truncate(max_lines);
        let mut shown: usize = lines.iter().map(|l| l.len()).sum();
        let mut out: Vec<String> = lines[..max_lines - 1].iter().map(|l| join(l)).collect();
        let last = lines.last_mut().unwrap();
        loop {
            let text = format!(
                "{}, {} +{} lainnya",
                join(last),
                self.g.ellipsis,
                items.len() - shown
            );
            if measure_text_width(&text) <= width || last.len() <= 1 {
                out.push(text);
                return out;
            }
            last.pop();
            shown -= 1;
        }
    }
}

/// Sembunyikan kursor selama layar interaktif; pulihkan saat keluar/error.
struct CursorGuard<'a>(&'a Term);

impl<'a> CursorGuard<'a> {
    fn hide(term: &'a Term) -> Self {
        let _ = term.hide_cursor();
        CursorGuard(term)
    }
}

impl Drop for CursorGuard<'_> {
    fn drop(&mut self) {
        let _ = self.0.show_cursor();
    }
}
