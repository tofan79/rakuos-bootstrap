//! Tampilan TUI RakuOS: aman di TTY (`TERM=linux`) sampai terminal modern.
//!
//! Tanpa dependensi baru, hanya `console`. Kemampuan terminal dideteksi sekali
//! di `Ui::new`, lalu semua gambar memakai tiga tingkat:
//!
//! | Charset | Kapan                              | Kotak | Penanda      |
//! |---------|------------------------------------|-------|--------------|
//! | Full    | terminal emulator UTF-8            | ╭─╮   | ✔ ● ○ ■ □ ❯  |
//! | Tty     | `TERM=linux` + UTF-8 (console VT)  | ┌─┐   | [x] (*) [ ]  |
//! | Ascii   | locale bukan UTF-8 / `--ascii`     | +-+   | [x] (*) [ ]  |
//!
//! Warna: 256 (cyan terang) kalau terminal mendukung, 16 warna ANSI di console
//! VT, dan polos kalau output bukan TTY atau `NO_COLOR` aktif.

use std::fmt;
use std::io::Write;

use anyhow::{bail, Result};
use console::{measure_text_width, truncate_str, Key, Style, Term};

use crate::catalog::Choice;

/// Nama pendek tiap fase untuk stepper.
const STEPS: [&str; 5] = ["Desktop", "Terminal", "File Manager", "Browser", "Login"];

/// Hasil satu layar pilihan: lanjut dengan pilihan, atau mundur satu fase.
pub enum Nav<T> {
    Next(T),
    Back,
}

/// Hasil layar ringkasan.
pub enum Confirm {
    Yes,
    No,
    Back,
}

/// Tombol untuk mundur satu fase. Membatalkan seluruh wizard hanya lewat Ctrl-C.
fn is_back(key: &Key) -> bool {
    matches!(
        key,
        Key::Backspace | Key::ArrowLeft | Key::Escape | Key::Char('b')
    )
}

/// Pengguna menekan Ctrl-C.
#[derive(Debug)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cancelled")
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
    brand: &'static str,
    radio_on: &'static str,
    radio_off: &'static str,
    check_on: &'static str,
    check_off: &'static str,
    left: &'static str,
    link: &'static str,
    dot: &'static str,
    ellipsis: &'static str,
    updown: &'static str,
    up: &'static str,
    down: &'static str,
}

impl Glyphs {
    fn for_charset(cs: Charset) -> Self {
        match cs {
            Charset::Full => Glyphs {
                tl: "╭",
                tr: "╮",
                bl: "╰",
                br: "╯",
                h: "─",
                v: "│",
                cursor: "❯",
                done: "✔",
                current: "●",
                todo: "○",
                brand: "◆",
                radio_on: "●",
                radio_off: "○",
                check_on: "■",
                check_off: "□",
                left: "←",
                link: "──",
                dot: "·",
                ellipsis: "…",
                updown: "↑↓",
                up: "↑",
                down: "↓",
            },
            Charset::Tty => Glyphs {
                tl: "┌",
                tr: "┐",
                bl: "└",
                br: "┘",
                h: "─",
                v: "│",
                cursor: ">",
                done: "[x]",
                current: "[>]",
                todo: "[ ]",
                brand: "*",
                radio_on: "(*)",
                radio_off: "( )",
                check_on: "[x]",
                check_off: "[ ]",
                left: "←",
                link: "──",
                dot: "·",
                ellipsis: "...",
                updown: "↑↓",
                up: "↑",
                down: "↓",
            },
            Charset::Ascii => Glyphs {
                tl: "+",
                tr: "+",
                bl: "+",
                br: "+",
                h: "-",
                v: "|",
                cursor: ">",
                done: "[x]",
                current: "[>]",
                todo: "[ ]",
                brand: "*",
                radio_on: "(*)",
                radio_off: "( )",
                check_on: "[x]",
                check_off: "[ ]",
                left: "<-",
                link: "--",
                dot: "-",
                ellipsis: "...",
                updown: "Up/Down",
                up: "^",
                down: "v",
            },
        }
    }
}

/// Warna per peran. Ganti di satu tempat ini kalau brand berubah.
struct Palette {
    accent: Style,   // cyan brand accent
    accent_b: Style, // sama, tebal
    ok: Style,
    warn: Style,
    dim: Style,
    bold: Style,
    sel: Style,  // baris yang sedang disorot
    key: Style,  // tombol di footer
    pill: Style, // lencana (DRY-RUN, FIRST BOOT)
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
                sel: Style::new().bold(),
                key: Style::new().bold(),
                pill: Style::new().bold(),
            },
            // Console VT: bright cyan (VGA #00FFFF). Dim text uses bright black
            // (abu-abu tua) supaya tetap terbaca di latar hitam.
            ColorLevel::Vt16 => Palette {
                accent: Style::new().cyan().bright(),
                accent_b: Style::new().cyan().bright().bold(),
                ok: Style::new().green(),
                warn: Style::new().yellow().bold(),
                dim: Style::new().black().bright(),
                bold: Style::new().white().bold(),
                sel: Style::new().black().on_cyan().on_bright(),
                key: Style::new().white().bold(),
                pill: Style::new().black().on_yellow().bold(),
            },
            ColorLevel::Xterm256 => Palette {
                accent: Style::new().color256(51),
                accent_b: Style::new().color256(51).bold(),
                ok: Style::new().color256(108),
                warn: Style::new().color256(220),
                dim: Style::new().color256(245),
                bold: Style::new().bold(),
                sel: Style::new().color256(51).on_color256(237).bold(),
                key: Style::new().color256(255).bold(),
                pill: Style::new().color256(16).on_color256(220).bold(),
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

fn wrap_words(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let word_width = measure_text_width(word);
        if !line.is_empty() && measure_text_width(&line) + 1 + word_width > width {
            lines.push(std::mem::take(&mut line));
        }
        if word_width > width {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            lines.push(truncate_str(word, width, "…").to_string());
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

pub struct Ui {
    color: ColorLevel,
    pal: Palette,
    g: Glyphs,
    width: usize,
    height: usize,
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
        let (rows, cols) = term.size();
        let width = (cols as usize).saturating_sub(2).clamp(40, 78);
        let height = (rows as usize).max(12);

        Ui {
            color,
            pal: Palette::for_level(color),
            g: Glyphs::for_charset(charset),
            width,
            height,
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
            edge.apply_to(format!(
                "{}{}{}",
                self.g.tl,
                self.rule(self.width - 2),
                self.g.tr
            ))
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
            edge.apply_to(format!(
                "{}{}{}",
                self.g.bl,
                self.rule(self.width - 2),
                self.g.br
            ))
        ));
        out
    }

    /// Lencana kecil; di terminal tanpa warna jadi `[TEKS]`.
    fn pill(&self, text: &str) -> String {
        if self.color == ColorLevel::None {
            format!("[{text}]")
        } else {
            self.pal.pill.apply_to(format!(" {text} ")).to_string()
        }
    }

    fn header(&self) -> Vec<String> {
        let inner = self.width - 4;
        let p = &self.pal;

        let brand = format!(
            "{} {}{}",
            p.accent_b.apply_to(self.g.brand),
            p.accent_b.apply_to("Raku"),
            p.bold.apply_to("OS Setup")
        );
        let ver = p.dim.apply_to(format!("v{}", self.version)).to_string();
        let gap = inner.saturating_sub(measure_text_width(&brand) + measure_text_width(&ver));
        let line1 = format!("{brand}{}{ver}", " ".repeat(gap));

        let tagline_text = "Post-install setup: choose your desktop and apps";
        let line2 = match &self.badge {
            Some(b) => {
                let b = self.pill(b);
                let room = inner.saturating_sub(measure_text_width(&b) + 1);
                let tagline = p.dim.apply_to(self.fit(tagline_text, room)).to_string();
                let gap =
                    inner.saturating_sub(measure_text_width(&tagline) + measure_text_width(&b));
                format!("{tagline}{}{b}", " ".repeat(gap))
            }
            None => p.dim.apply_to(self.fit(tagline_text, inner)).to_string(),
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
                // Garis penghubung ikut berwarna untuk langkah yang sudah lewat.
                let link_style = if phase >= 5 || i < phase {
                    &p.ok
                } else {
                    &p.dim
                };
                out.push_str(&format!(" {} ", link_style.apply_to(self.g.link)));
            }
        }
        out
    }

    fn frame(&self, phase: usize, body: Vec<String>, footer: String) -> Vec<String> {
        let mut lines = vec![String::new()];
        lines.extend(self.header());
        lines.push(String::new());
        lines.push(self.stepper(phase));
        lines.push(String::new());
        lines.extend(body);
        lines.push(String::new());
        lines.push(format!(" {footer}"));
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

    /// Footer: tombol tebal, keterangan redup. Kalau terminal sempit, petunjuk
    /// dibuang dari kanan supaya yang terpenting tetap terlihat.
    fn hints(&self, hints: &[(&str, &str)]) -> String {
        let max = self.width - 2;
        let mut shown = hints.len();
        loop {
            let text = hints[..shown]
                .iter()
                .map(|(key, label)| {
                    format!(
                        "{} {}",
                        self.pal.key.apply_to(key),
                        self.pal.dim.apply_to(label)
                    )
                })
                .collect::<Vec<_>>()
                .join("  ");
            if shown <= 1 || measure_text_width(&text) <= max {
                return text;
            }
            shown -= 1;
        }
    }

    fn footer_select(&self) -> String {
        let back = format!("Esc/{}", self.g.left);
        self.hints(&[
            (self.g.updown, "Move"),
            ("Enter", "Continue"),
            (&back, "Back"),
            ("1-9", "Jump"),
            ("Ctrl-C", "Quit"),
        ])
    }

    fn footer_multi(&self) -> String {
        let back = format!("Esc/{}", self.g.left);
        self.hints(&[
            (self.g.updown, "Move"),
            ("Space", "Toggle"),
            ("Enter", "Continue"),
            (&back, "Back"),
            ("Ctrl-C", "Quit"),
        ])
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
        initial: Option<usize>,
    ) -> Result<Nav<usize>> {
        if choices.iter().all(|c| !c.available) {
            bail!("no available choices for {title}");
        }
        let _guard = CursorGuard::hide(term);
        // Kembali ke fase ini: sorot lagi pilihan sebelumnya.
        let mut sel = initial
            .filter(|&i| i < choices.len() && choices[i].available)
            .or_else(|| choices.iter().position(|c| c.available && c.default))
            .or_else(|| choices.iter().position(|c| c.available))
            .unwrap_or(0);
        let mut msg: Option<String> = None;

        loop {
            let body = self.select_body(
                phase,
                title,
                hint,
                choices,
                sel,
                picked,
                msg.as_deref(),
                None,
            );
            let lines = self.frame(phase, body, self.footer_select());
            self.paint(term, &lines)?;

            let key = term.read_key()?;
            if is_back(&key) {
                if phase == 0 {
                    msg = Some("You are on the first step. Press Ctrl-C to quit.".to_string());
                    continue;
                }
                return Ok(Nav::Back);
            }
            match key {
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
                        return Ok(Nav::Next(sel));
                    }
                    msg = Some(format!(
                        "{} is not available. Choose another option.",
                        choices[sel].name
                    ));
                }
                Key::CtrlC => return Err(Cancelled.into()),
                _ => {}
            }
        }
    }

    /// Layar pilihan berganda (boleh lebih dari satu). Mengembalikan indeks
    /// yang dicentang, urut sesuai tampilan. Minimal satu harus dipilih.
    pub fn multi_select(
        &self,
        term: &Term,
        phase: usize,
        title: &str,
        hint: &str,
        choices: &[Choice],
        picked: &[String],
        initial: Option<&[usize]>,
    ) -> Result<Nav<Vec<usize>>> {
        if choices.iter().all(|c| !c.available) {
            bail!("no available choices for {title}");
        }
        let _guard = CursorGuard::hide(term);

        // Kembali ke fase ini: centang lagi pilihan sebelumnya.
        let mut checked: Vec<bool> = match initial {
            Some(prev) if !prev.is_empty() => (0..choices.len())
                .map(|i| choices[i].available && prev.contains(&i))
                .collect(),
            _ => choices.iter().map(|c| c.available && c.default).collect(),
        };
        if !checked.iter().any(|&b| b) {
            if let Some(i) = choices.iter().position(|c| c.available) {
                checked[i] = true;
            }
        }
        let mut sel = checked.iter().position(|&b| b).unwrap_or(0);
        let mut msg: Option<String> = None;

        loop {
            let body = self.select_body(
                phase,
                title,
                hint,
                choices,
                sel,
                picked,
                msg.as_deref(),
                Some(&checked),
            );
            let lines = self.frame(phase, body, self.footer_multi());
            self.paint(term, &lines)?;

            let key = term.read_key()?;
            if is_back(&key) {
                if phase == 0 {
                    msg = Some("You are on the first step. Press Ctrl-C to quit.".to_string());
                    continue;
                }
                return Ok(Nav::Back);
            }
            match key {
                Key::ArrowUp | Key::Char('k') => {
                    sel = (sel + choices.len() - 1) % choices.len();
                    msg = None;
                }
                Key::ArrowDown | Key::Char('j') | Key::Tab => {
                    sel = (sel + 1) % choices.len();
                    msg = None;
                }
                Key::Char(' ') => {
                    if choices[sel].available {
                        checked[sel] = !checked[sel];
                        msg = None;
                    } else {
                        msg = Some(format!("{} is not available.", choices[sel].name));
                    }
                }
                Key::Char(c) if c.is_ascii_digit() && c != '0' => {
                    let n = c as usize - '1' as usize;
                    if n < choices.len() {
                        if choices[n].available {
                            checked[n] = !checked[n];
                            msg = None;
                        } else {
                            msg = Some(format!("{} is not available.", choices[n].name));
                        }
                    }
                }
                Key::Enter => {
                    if checked.iter().any(|&b| b) {
                        return Ok(Nav::Next(
                            checked
                                .iter()
                                .enumerate()
                                .filter(|(_, &b)| b)
                                .map(|(i, _)| i)
                                .collect(),
                        ));
                    }
                    msg = Some("Select at least one option.".to_string());
                }
                Key::CtrlC => return Err(Cancelled.into()),
                _ => {}
            }
        }
    }

    /// Sel awal satu baris daftar (kursor, nomor lompat, penanda) sebagai
    /// `(polos, berwarna)`. Versi polos dipakai saat baris disorot.
    fn row_lead(
        &self,
        index: usize,
        active: bool,
        multi: bool,
        on: bool,
        available: bool,
    ) -> (String, String) {
        let p = &self.pal;
        let cursor = if active {
            self.g.cursor.to_string()
        } else {
            " ".repeat(measure_text_width(self.g.cursor))
        };
        let num = if index < 9 {
            (index + 1).to_string()
        } else {
            " ".to_string()
        };
        let mark = match (multi, on) {
            (true, true) => self.g.check_on,
            (true, false) => self.g.check_off,
            (false, true) => self.g.radio_on,
            (false, false) => self.g.radio_off,
        };
        let mark_style = if !available {
            &p.dim
        } else if multi && on {
            &p.ok
        } else if on {
            &p.accent
        } else {
            &p.dim
        };
        let plain = format!("{cursor} {num} {mark} ");
        let styled = format!(
            "{cursor} {} {} ",
            p.dim.apply_to(&num),
            mark_style.apply_to(mark)
        );
        (plain, styled)
    }

    /// Baris jejak di bawah daftar (pesan kesalahan menggantikan jejak).
    fn trail_line(&self, msg: Option<&str>, parts: &[String]) -> String {
        let p = &self.pal;
        let budget = self.width.saturating_sub(2);
        match msg {
            Some(message) => format!(" {}", p.warn.apply_to(self.fit(message, budget))),
            None if !parts.is_empty() => format!(
                " {}",
                p.dim
                    .apply_to(self.fit(&parts.join(&format!("  {}  ", self.g.dot)), budget))
            ),
            None => String::new(),
        }
    }

    /// Wide terminals use a list pane and a detail pane, similar to the
    /// greeter picker. Narrow TTYs retain the compact inline layout.
    fn split_select_body(
        &self,
        choices: &[Choice],
        sel: usize,
        picked: &[String],
        msg: Option<&str>,
        checked: Option<&[bool]>,
    ) -> Vec<String> {
        // Tinggi panel tetap (tidak bergantung pada item yang disorot) supaya
        // layar tidak melompat saat berpindah pilihan.
        const DETAIL_ROWS: usize = 12;

        let p = &self.pal;
        let multi = checked.is_some();
        let budget = self.width.saturating_sub(2);
        let panes = budget.saturating_sub(4);
        let left_w = (panes * 2 / 5).clamp(24, 30);
        let right_w = panes.saturating_sub(left_w);
        let max_rows = self.height.saturating_sub(17).max(3);
        let rows = choices.len().max(DETAIL_ROWS).min(max_rows);
        let start = if choices.len() > rows {
            sel.saturating_sub(rows / 2)
                .min(choices.len().saturating_sub(rows))
        } else {
            0
        };
        let end = (start + rows).min(choices.len());

        let mut options_title = "Options".to_string();
        if start > 0 {
            options_title.push_str(&format!("  {}{start}", self.g.up));
        }
        if end < choices.len() {
            options_title.push_str(&format!("  {}{}", self.g.down, choices.len() - end));
        }
        let mut body = vec![format!(
            " {} {} {}",
            p.accent_b.apply_to(self.pad(&options_title, left_w)),
            p.dim.apply_to(self.g.v),
            p.accent_b.apply_to(self.pad("Details", right_w))
        )];
        body.push(format!(
            " {}{}{}",
            p.dim.apply_to(self.rule(left_w + 1)),
            p.dim.apply_to(if self.g.v == "|" { "+" } else { "┼" }),
            p.dim.apply_to(self.rule(right_w + 1)),
        ));

        // --- panel detail untuk pilihan yang disorot ---
        let choice = &choices[sel];
        let mut details = vec![p
            .accent_b
            .apply_to(self.fit(&choice.name, right_w))
            .to_string()];
        let mut tags: Vec<String> = Vec::new();
        if !choice.available {
            tags.push(p.warn.apply_to("Not available").to_string());
        } else {
            if let Some(state) = checked {
                tags.push(if state[sel] {
                    p.ok.apply_to(format!("{} Selected", self.g.done))
                        .to_string()
                } else {
                    p.dim.apply_to("Not selected").to_string()
                });
            }
            if choice.default {
                tags.push(p.accent.apply_to("Default").to_string());
            }
        }
        if !tags.is_empty() {
            details.push(tags.join(&format!("  {}  ", self.g.dot)));
        }
        details.push(String::new());
        details.extend(wrap_words(&choice.description, right_w).into_iter().take(4));
        if !choice.packages.is_empty() {
            details.push(String::new());
            details.push(format!(
                "{} {}",
                p.bold.apply_to("Packages"),
                p.dim.apply_to(format!("({})", choice.packages.len()))
            ));
            let room = rows.saturating_sub(details.len());
            if room > 0 {
                for line in self.wrap_list(&choice.packages, right_w.saturating_sub(2), room.min(3))
                {
                    details.push(p.dim.apply_to(self.fit(&line, right_w)).to_string());
                }
            }
        }
        details.truncate(rows);

        for row in 0..rows {
            let left = if let Some(index) = (start..end).nth(row) {
                let item = &choices[index];
                let active = index == sel;
                let on = match checked {
                    Some(state) => state[index],
                    None => active,
                };
                let (lead_plain, lead_styled) =
                    self.row_lead(index, active, multi, on, item.available);
                if active {
                    let text = self.fit(&format!("{lead_plain}{}", item.name), left_w);
                    p.sel.apply_to(self.pad(&text, left_w)).to_string()
                } else {
                    let name = self.fit(
                        &item.name,
                        left_w.saturating_sub(measure_text_width(&lead_plain)),
                    );
                    let name = if !item.available {
                        p.dim.apply_to(name).to_string()
                    } else if multi && on {
                        p.bold.apply_to(name).to_string()
                    } else {
                        name
                    };
                    self.pad(&format!("{lead_styled}{name}"), left_w)
                }
            } else {
                " ".repeat(left_w)
            };
            let right = details
                .get(row)
                .map(|line| self.pad(line, right_w))
                .unwrap_or_else(|| " ".repeat(right_w));
            body.push(format!(" {} {} {}", left, p.dim.apply_to(self.g.v), right));
        }

        let mut trail = Vec::new();
        if let Some(state) = checked {
            let count = state.iter().filter(|&&selected| selected).count();
            trail.push(format!("{count} selected"));
        }
        if !picked.is_empty() {
            trail.push(format!("So far: {}", picked.join(", ")));
        }
        body.push(self.trail_line(msg, &trail));
        body
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
        checked: Option<&[bool]>,
    ) -> Vec<String> {
        let p = &self.pal;
        let inner = self.width - 2;

        let step = format!("Step {} of {}", phase + 1, STEPS.len());
        let gap = inner.saturating_sub(measure_text_width(title) + measure_text_width(&step));
        let mut body = vec![
            format!(
                " {}{}{}",
                p.accent_b.apply_to(title),
                " ".repeat(gap),
                p.dim.apply_to(&step)
            ),
            format!(" {}", p.dim.apply_to(self.fit(hint, inner))),
            format!(" {}", p.dim.apply_to(self.rule(inner))),
        ];

        if self.width >= 68 {
            body.extend(self.split_select_body(choices, sel, picked, msg, checked));
            return body;
        }

        // --- tata letak ringkas untuk terminal sempit ---
        let multi = checked.is_some();
        let name_w = 16;
        let lead_w = measure_text_width(self.g.cursor)
            + 1
            + 1
            + 1
            + measure_text_width(self.g.check_on).max(measure_text_width(self.g.radio_on))
            + 1;
        let desc_w = inner.saturating_sub(lead_w + name_w + 1);

        // Jendela daftar: sisakan ruang untuk header, stepper, jejak, dan
        // footer, supaya layar 80x25 tidak menggulung saat pilihan banyak.
        let max_items = self.height.saturating_sub(15).max(3);
        let mut start = 0usize;
        let mut end = choices.len();
        if choices.len() > max_items {
            start = sel.saturating_sub(max_items / 2);
            if start + max_items > choices.len() {
                start = choices.len() - max_items;
            }
            end = start + max_items;
        }

        for i in start..end {
            let c = &choices[i];
            let active = i == sel;
            let on = match checked {
                Some(state) => state[i],
                None => active,
            };
            let (lead_plain, lead_styled) = self.row_lead(i, active, multi, on, c.available);
            let name = self.pad(&self.fit(&c.name, name_w), name_w);
            let desc = if c.available {
                self.fit(&c.description, desc_w)
            } else {
                format!(
                    "(not available) {}",
                    self.fit(&c.description, desc_w.saturating_sub(16))
                )
            };
            if active {
                let text = self.fit(&format!("{lead_plain}{name} {desc}"), inner);
                body.push(format!(" {}", p.sel.apply_to(self.pad(&text, inner))));
            } else if !c.available {
                body.push(format!(
                    " {lead_styled}{} {}",
                    p.dim.apply_to(&name),
                    p.dim.apply_to(&desc)
                ));
            } else {
                let name = if multi && on {
                    p.bold.apply_to(&name).to_string()
                } else {
                    name
                };
                body.push(format!(" {lead_styled}{name} {}", p.dim.apply_to(&desc)));
            }
        }

        body.push(match choices.get(sel) {
            Some(c) if c.available && measure_text_width(&c.description) > desc_w => {
                format!("   {}", p.dim.apply_to(self.fit(&c.description, inner - 2)))
            }
            _ => String::new(),
        });
        let mut trail_parts: Vec<String> = Vec::new();
        if start > 0 {
            trail_parts.push(format!("{}{} more", self.g.up, start));
        }
        if end < choices.len() {
            trail_parts.push(format!("{}{} more", self.g.down, choices.len() - end));
        }
        if let Some(selected) = checked {
            let count = selected.iter().filter(|&&is_selected| is_selected).count();
            trail_parts.push(format!("{count} selected"));
        }
        if !picked.is_empty() {
            trail_parts.push(format!("So far: {}", picked.join(", ")));
        }
        body.push(self.trail_line(msg, &trail_parts));
        body
    }

    /// Layar ringkasan + konfirmasi. `true` = lanjut.
    pub fn confirm_summary(
        &self,
        term: &Term,
        rows: &[(&str, String)],
        packages: &[String],
    ) -> Result<Confirm> {
        let _guard = CursorGuard::hide(term);
        let p = &self.pal;
        let inner = self.width - 2;
        let label_w = 14;
        let value_w = inner.saturating_sub(label_w + 1);
        let indent = " ".repeat(label_w);

        let mut body = vec![
            format!(" {}", p.accent_b.apply_to("Review your setup")),
            format!(" {}", p.dim.apply_to(self.rule(inner))),
        ];
        for (k, v) in rows {
            // Nilai panjang (mis. banyak browser) dibungkus, maksimal dua baris.
            let mut lines = wrap_words(v, value_w);
            if lines.len() > 2 {
                lines.truncate(2);
                if let Some(last) = lines.pop() {
                    lines.push(self.fit(&format!("{last} {}", self.g.ellipsis), value_w));
                }
            }
            if lines.is_empty() {
                lines.push(String::new());
            }
            for (i, line) in lines.iter().enumerate() {
                let label = if i == 0 {
                    p.dim.apply_to(self.pad(k, label_w)).to_string()
                } else {
                    indent.clone()
                };
                body.push(format!(" {label} {}", p.bold.apply_to(line)));
            }
        }
        body.push(format!(
            " {} {}",
            p.dim.apply_to(self.pad("Packages", label_w)),
            p.bold.apply_to(format!("{} RPM packages", packages.len()))
        ));
        for line in self.wrap_list(packages, value_w, 2) {
            body.push(format!(
                " {indent} {}",
                p.dim.apply_to(self.fit(&line, value_w))
            ));
        }
        body.push(String::new());
        body.push(format!(
            " {} {} {}",
            p.accent_b.apply_to("?"),
            p.bold.apply_to("Continue with installation?"),
            p.dim.apply_to("[Y/n]")
        ));

        let back = format!("Esc/{}", self.g.left);
        let footer = self.hints(&[
            ("Enter/Y", "Continue"),
            (&back, "Back"),
            ("N", "Cancel"),
            ("Ctrl-C", "Quit"),
        ]);
        let lines = self.frame(5, body, footer);
        self.paint(term, &lines)?;
        loop {
            let key = term.read_key()?;
            if is_back(&key) {
                return Ok(Confirm::Back);
            }
            match key {
                Key::Enter | Key::Char('y') | Key::Char('Y') => return Ok(Confirm::Yes),
                Key::Char('n') | Key::Char('N') => return Ok(Confirm::No),
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

    /// A yes/no prompt on one line without clearing the screen (e.g. reboot).
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
                writeln!(out, "{}", if a { "yes" } else { "no" })?;
                return Ok(a);
            }
        }
    }

    pub fn success(&self, text: &str) -> String {
        self.pal
            .ok
            .apply_to(format!("{} {text}", self.g.done))
            .to_string()
    }

    pub fn note(&self, text: &str) -> String {
        self.pal.dim.apply_to(text).to_string()
    }

    /// Wrap a list into at most `max_lines`; show the remainder as "+N more".
    fn wrap_list(&self, items: &[String], width: usize, max_lines: usize) -> Vec<String> {
        let mut lines: Vec<Vec<&String>> = vec![Vec::new()];
        let mut used = 0usize;
        for item in items {
            let need = measure_text_width(item) + 2; // ", "
            if used + need > width + 2 && !lines.last().is_none_or(|l| l.is_empty()) {
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
                "{}, {} +{} more",
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
