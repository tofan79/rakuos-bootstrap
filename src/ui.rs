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
//! Warna: 256 (cyan terang) kalau terminal mendukung, 16 warna ANSI di console
//! VT, dan polos kalau output bukan TTY atau `NO_COLOR` aktif.

use std::fmt;
use std::io::Write;

use anyhow::{bail, Result};
use console::{measure_text_width, truncate_str, Key, Style, Term};

use crate::catalog::Choice;

/// Nama pendek tiap fase untuk stepper.
const STEPS: [&str; 5] = ["Desktop", "Terminal", "File Manager", "Browser", "Sign-in"];

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
            // Console VT: bright cyan (VGA #00FFFF). Dim text uses bright black
            // (abu-abu tua) supaya tetap terbaca di latar hitam.
            ColorLevel::Vt16 => Palette {
                accent: Style::new().cyan().bright(),
                accent_b: Style::new().cyan().bright().bold(),
                ok: Style::new().green(),
                warn: Style::new().yellow().bold(),
                dim: Style::new().black().bright(),
                bold: Style::new().white().bold(),
            },
            ColorLevel::Xterm256 => Palette {
                accent: Style::new().color256(51),
                accent_b: Style::new().color256(51).bold(),
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

    fn header(&self) -> Vec<String> {
        let inner = self.width - 4;
        let p = &self.pal;

        let brand = format!(
            "{}{} {}",
            p.accent_b.apply_to("RAKU"),
            p.bold.apply_to("OS"),
            p.dim.apply_to("setup")
        );
        let ver = p.dim.apply_to(format!("v{}", self.version)).to_string();
        let gap = inner.saturating_sub(measure_text_width(&brand) + measure_text_width(&ver));
        let line1 = format!("{brand}{}{ver}", " ".repeat(gap));

        let tagline = p.dim.apply_to("Set up your RakuOS desktop").to_string();
        let line2 = match &self.badge {
            Some(b) => {
                let b = p.warn.apply_to(b).to_string();
                let gap =
                    inner.saturating_sub(measure_text_width(&tagline) + measure_text_width(&b));
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
        lines.push(format!(
            " {}",
            self.pal.dim.apply_to(self.fit(footer, self.width - 2))
        ));
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
            "{} Move  Enter Continue  Esc/← Back  1-9 Jump  Ctrl-C Quit",
            self.g.updown
        )
    }

    fn footer_multi(&self) -> String {
        format!(
            "{} Move  Space Toggle  Enter Continue  Esc/← Back  Ctrl-C Quit",
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
            let lines = self.frame(phase, body, &self.footer_select());
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
            let lines = self.frame(phase, body, &self.footer_multi());
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

    /// Wide terminals use a list pane and a detail pane, similar to the
    /// greeter picker. Narrow TTYs retain the existing inline layout.
    fn split_select_body(
        &self,
        choices: &[Choice],
        sel: usize,
        picked: &[String],
        msg: Option<&str>,
        checked: Option<&[bool]>,
    ) -> Vec<String> {
        let p = &self.pal;
        let budget = self.width.saturating_sub(2);
        let panes = budget.saturating_sub(4);
        let left_w = panes.div_ceil(2);
        let right_w = panes.saturating_sub(left_w);
        let max_rows = self.height.saturating_sub(17).max(3);
        let start = if choices.len() > max_rows {
            sel.saturating_sub(max_rows / 2)
                .min(choices.len().saturating_sub(max_rows))
        } else {
            0
        };
        let end = (start + max_rows).min(choices.len());

        let mut options_title = "Options".to_string();
        if start > 0 {
            options_title.push_str(&format!("  ↑{start}"));
        }
        if end < choices.len() {
            options_title.push_str(&format!("  ↓{}", choices.len() - end));
        }
        let mut body = vec![format!(
            " {} {} {}",
            p.accent_b.apply_to(self.pad(&options_title, left_w)),
            p.dim.apply_to(self.g.v),
            p.accent_b.apply_to(self.pad("Details", right_w))
        )];
        body.push(format!(
            " {} {} {}",
            p.dim.apply_to(self.rule(left_w)),
            p.dim.apply_to(self.g.v),
            p.dim.apply_to(self.rule(right_w))
        ));

        let choice = &choices[sel];
        let mut details = vec![p.bold.apply_to(&choice.name).to_string(), String::new()];
        details.extend(
            wrap_words(&choice.description, right_w)
                .into_iter()
                .take(4)
                .map(|line| p.dim.apply_to(line).to_string()),
        );
        details.push(if !choice.available {
            p.warn.apply_to("Not available").to_string()
        } else if checked.map_or(true, |state| state[sel]) {
            p.ok.apply_to("Selected").to_string()
        } else {
            p.dim.apply_to("Not selected").to_string()
        });
        if !choice.packages.is_empty() {
            details.push(p.bold.apply_to("Packages").to_string());
            for package in choice.packages.iter().take(3) {
                details.push(p.dim.apply_to(self.fit(package, right_w)).to_string());
            }
            if choice.packages.len() > 3 {
                details.push(
                    p.dim
                        .apply_to(format!("+ {} more", choice.packages.len() - 3))
                        .to_string(),
                );
            }
        }
        details.truncate(max_rows);

        for row in 0..max_rows {
            let left = if let Some(index) = (start..end).nth(row) {
                let item = &choices[index];
                let cursor = if index == sel { self.g.cursor } else { " " };
                let mark = match checked {
                    Some(state) if state[index] => "[x]",
                    Some(_) => "[ ]",
                    None => "   ",
                };
                let label = self.fit(&format!("{cursor} {mark} {}", item.name), left_w);
                let styled = if !item.available {
                    p.dim.apply_to(label).to_string()
                } else if index == sel {
                    p.accent_b.apply_to(label).to_string()
                } else {
                    p.dim.apply_to(label).to_string()
                };
                self.pad(&styled, left_w)
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
        if !picked.is_empty() {
            trail.push(picked.join(&format!("  {}  ", self.g.dot)));
        }
        if let Some(state) = checked {
            let count = state.iter().filter(|&&selected| selected).count();
            trail.push(format!("{count} selected"));
        }
        body.push(match msg {
            Some(message) => format!(" {}", p.warn.apply_to(self.fit(message, budget))),
            None if !trail.is_empty() => format!(
                " {}",
                p.dim
                    .apply_to(self.fit(&trail.join(&format!("  {}  ", self.g.dot)), budget))
            ),
            None => String::new(),
        });
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
        let mut body = Vec::new();

        body.push(format!(
            " {} {} {}",
            p.accent_b
                .apply_to(format!("STEP {} OF {}", phase + 1, STEPS.len())),
            p.dim.apply_to(self.g.dot),
            p.bold.apply_to(title)
        ));
        body.push(format!(
            " {}",
            p.dim.apply_to(self.fit(hint, self.width - 2))
        ));
        body.push(format!(" {}", p.dim.apply_to(self.rule(self.width - 2))));

        if self.width >= 68 {
            body.extend(self.split_select_body(choices, sel, picked, msg, checked));
            return body;
        }

        let name_w = 20;
        let mark_w = if checked.is_some() { 4 } else { 0 };
        let desc_w = self.width.saturating_sub(name_w + 6 + mark_w);

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
            let cursor = if active {
                p.accent_b.apply_to(self.g.cursor).to_string()
            } else {
                " ".repeat(measure_text_width(self.g.cursor))
            };
            let mark = match checked {
                Some(ch) if ch[i] => p.ok.apply_to("[x] ").to_string(),
                Some(_) => p.dim.apply_to("[ ] ").to_string(),
                None => String::new(),
            };
            let (name, desc) = if !c.available {
                (
                    p.dim.apply_to(self.pad(&c.name, name_w)).to_string(),
                    format!(
                        "{} {}",
                        p.warn.apply_to("(not available)"),
                        p.dim
                            .apply_to(self.fit(&c.description, desc_w.saturating_sub(17)))
                    ),
                )
            } else if active {
                (
                    p.accent_b.apply_to(self.pad(&c.name, name_w)).to_string(),
                    p.bold
                        .apply_to(self.fit(&c.description, desc_w))
                        .to_string(),
                )
            } else {
                (
                    self.pad(&c.name, name_w),
                    p.dim.apply_to(self.fit(&c.description, desc_w)).to_string(),
                )
            };
            body.push(format!(" {cursor} {mark}{name} {desc}"));
        }

        body.push(match choices.get(sel) {
            Some(c) if c.available && measure_text_width(&c.description) > desc_w => {
                format!(
                    " {}",
                    p.dim.apply_to(self.fit(&c.description, self.width - 2))
                )
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
            trail_parts.push(picked.join(&format!("  {}  ", self.g.dot)));
        }

        body.push(match msg {
            Some(m) => format!(" {}", p.warn.apply_to(m)),
            None if !trail_parts.is_empty() => format!(
                " {}",
                p.dim.apply_to(self.fit(
                    &trail_parts.join(&format!("  {}  ", self.g.dot)),
                    self.width - 2
                ))
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
    ) -> Result<Confirm> {
        let _guard = CursorGuard::hide(term);
        let p = &self.pal;
        let mut body = vec![
            format!(" {}", p.accent_b.apply_to("Review your setup")),
            format!(" {}", p.dim.apply_to(self.rule(self.width - 2))),
        ];
        for (k, v) in rows {
            body.push(format!(
                " {} {}",
                p.dim.apply_to(self.pad(k, 14)),
                p.bold.apply_to(v)
            ));
        }
        body.push(format!(
            " {}",
            p.dim.apply_to(format!("RPM packages ({})", packages.len()))
        ));
        for line in self.wrap_list(packages, self.width - 4, 2) {
            body.push(format!("   {}", p.dim.apply_to(line)));
        }
        body.push(format!(
            " {} {}",
            p.accent_b.apply_to("?"),
            p.bold.apply_to("Continue with installation?")
        ));

        let lines = self.frame(
            5,
            body,
            "Y/Enter Continue  Esc/← Back  n Cancel  Ctrl-C Quit",
        );
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
