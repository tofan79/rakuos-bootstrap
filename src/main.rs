// SPDX-License-Identifier: MIT
mod catalog;
mod install;
mod log;
mod ui;
mod verify;

#[cfg(test)]
mod testing;

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Result, bail};
use catalog::Catalog;
use console::Term;
use ui::{Cancelled, Confirm, Nav, Ui};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_BASE_DIR: &str = "/usr/share/rakuos/bootstrap";

struct Opts {
    dry_run: bool,
    first_boot: bool,
    ascii: bool,
    catalog: Option<PathBuf>,
    base_dir: PathBuf,
    log_file: PathBuf,
}

fn main() {
    if let Err(err) = run() {
        if err.downcast_ref::<Cancelled>().is_some() {
            println!("\nCancelled. No changes were made.");
            std::process::exit(130);
        }
        log::note(&format!("error: {err:#}"));
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let opts = parse_args()?;

    if !opts.dry_run && !is_root() {
        bail!("Run with sudo: sudo rakuos-bootstrap");
    }

    let term = Term::stdout();
    if !term.is_term() {
        bail!("An interactive terminal (TTY) is required");
    }

    if !opts.dry_run
        && let Err(msg) = log::init(&opts.log_file, &install::timestamp())
    {
        eprintln!("warning: {msg}; continuing without a log file");
    }

    let catalog = Catalog::load(opts.catalog.as_deref())?;
    let badge = if opts.dry_run {
        Some("DRY-RUN".to_string())
    } else if opts.first_boot {
        Some("FIRST BOOT".to_string())
    } else {
        None
    };
    let ui = Ui::new(&term, opts.ascii, VERSION, badge);

    // Urutan fase = urutan tampil (dan STEPS di ui.rs). Desktop dipilih dulu;
    // desktop yang dikelola installer lain dapat langsung mengambil alih.
    const PHASE_DESKTOP: usize = 0;
    const PHASE_TERMINAL: usize = 1;
    const PHASE_FILE_MANAGER: usize = 2;
    const PHASE_BROWSER: usize = 3;
    const PHASE_LOGIN: usize = 4;
    const PHASE_SUMMARY: usize = 5;

    // Login yang `hidden` dipakai kode (mis. greeter bawaan KineticWE),
    // bukan ditawarkan ke user.
    let visible_logins: Vec<catalog::Choice> = catalog
        .logins
        .iter()
        .filter(|c| !c.hidden)
        .cloned()
        .collect();
    let login_indices: Vec<usize> = catalog
        .logins
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.hidden)
        .map(|(i, _)| i)
        .collect();

    // Pilihan tiap fase disimpan supaya tetap tercentang/tersorot saat user
    // mundur (Backspace / panah kiri) lalu maju lagi.
    let mut terminals: Vec<usize> = Vec::new();
    let mut file_managers: Vec<usize> = Vec::new();
    let mut browsers: Vec<usize> = Vec::new();
    let mut desktop: Option<usize> = None;
    let mut login: Option<usize> = None; // indeks di catalog.logins

    let names = |list: &[catalog::Choice], idx: &[usize]| -> Vec<String> {
        idx.iter().map(|&i| list[i].name.clone()).collect()
    };

    let mut phase = PHASE_DESKTOP;
    while phase != PHASE_SUMMARY {
        // Jejak pilihan dari fase-fase sebelumnya, ditampilkan di bawah daftar.
        let mut picked: Vec<String> = Vec::new();
        if phase > PHASE_DESKTOP
            && let Some(d) = desktop
        {
            picked.push(catalog.desktops[d].name.clone());
        }
        if phase > PHASE_TERMINAL {
            picked.extend(names(&catalog.terminals, &terminals));
        }
        if phase > PHASE_FILE_MANAGER {
            picked.extend(names(&catalog.file_managers, &file_managers));
        }
        if phase > PHASE_BROWSER {
            picked.extend(names(&catalog.browsers, &browsers));
        }
        match phase {
            PHASE_DESKTOP => {
                let nav = ui.select(
                    &term,
                    phase,
                    "Desktop",
                    "Choose your Wayland desktop",
                    &catalog.desktops,
                    &picked,
                    desktop,
                )?;
                match nav {
                    Nav::Next(i) => {
                        desktop = Some(i);
                        login = None;
                        phase = PHASE_TERMINAL;
                    }
                    Nav::Back => {}
                }
            }
            PHASE_TERMINAL => {
                let nav = ui.multi_select(
                    &term,
                    phase,
                    "Terminal",
                    "Where you type commands. Pick one or more",
                    &catalog.terminals,
                    &picked,
                    Some(&terminals),
                )?;
                match nav {
                    Nav::Next(v) => {
                        terminals = v;
                        phase = PHASE_FILE_MANAGER;
                    }
                    Nav::Back => phase = PHASE_DESKTOP,
                }
            }
            PHASE_FILE_MANAGER => {
                let nav = ui.multi_select(
                    &term,
                    phase,
                    "File Manager",
                    "Browse and manage your files. Pick one or more",
                    &catalog.file_managers,
                    &picked,
                    Some(&file_managers),
                )?;
                match nav {
                    Nav::Next(v) => {
                        file_managers = v;
                        phase = PHASE_BROWSER;
                    }
                    Nav::Back => phase = PHASE_TERMINAL,
                }
            }
            PHASE_BROWSER => {
                let nav = ui.multi_select(
                    &term,
                    phase,
                    "Browser",
                    "Pick one or more browsers to install",
                    &catalog.browsers,
                    &picked,
                    Some(&browsers),
                )?;
                match nav {
                    Nav::Next(v) => {
                        browsers = v;
                        let d = desktop.expect("desktop was selected before apps");
                        phase = if catalog.desktops[d].choose_login {
                            PHASE_LOGIN
                        } else {
                            PHASE_SUMMARY
                        };
                    }
                    Nav::Back => phase = PHASE_FILE_MANAGER,
                }
            }
            PHASE_LOGIN => {
                let prev = login.and_then(|l| login_indices.iter().position(|&i| i == l));
                let nav = ui.select(
                    &term,
                    phase,
                    "Login Manager",
                    "The screen where you sign in after boot",
                    &visible_logins,
                    &picked,
                    prev,
                )?;
                match nav {
                    Nav::Next(pos) => {
                        login = Some(login_indices[pos]);
                        phase = PHASE_SUMMARY;
                    }
                    Nav::Back => phase = PHASE_BROWSER,
                }
            }
            _ => unreachable!(),
        }

        // Ringkasan + konfirmasi. Mundur dari sini kembali ke fase terakhir
        // yang benar-benar ditampilkan (Login hanya kalau desktop memintanya).
        if phase == PHASE_SUMMARY {
            match summarize(
                &ui,
                &term,
                &catalog,
                &terminals,
                &file_managers,
                &browsers,
                desktop.expect("desktop has been selected"),
                login,
            )? {
                Confirm::Yes => break,
                Confirm::No => {
                    println!("\nCancelled. No changes were made.");
                    return Ok(());
                }
                Confirm::Back => {
                    let d = desktop.expect("desktop has been selected");
                    phase = if catalog.desktops[d].choose_login {
                        PHASE_LOGIN
                    } else {
                        PHASE_BROWSER
                    };
                }
            }
        }
    }

    let desktop_choice = &catalog.desktops[desktop.expect("desktop has been selected")];
    let login_choice = catalog.effective_login(desktop_choice, login.map(|i| &catalog.logins[i]));
    let picks = install::Picks {
        terminals: terminals.iter().map(|&i| &catalog.terminals[i]).collect(),
        file_managers: file_managers
            .iter()
            .map(|&i| &catalog.file_managers[i])
            .collect(),
        browsers: browsers.iter().map(|&i| &catalog.browsers[i]).collect(),
        login: login_choice,
        desktop: desktop_choice,
    };

    ui.begin_log(&term)?;
    let checks = install::apply(&catalog, &picks, opts.dry_run, &opts.base_dir)?;
    let failed = checks.iter().filter(|c| !c.ok).count();

    println!("\n{}", ui.success("Setup complete."));
    if opts.dry_run {
        println!(
            "{}",
            ui.note("(Dry run: no packages installed and no files written)")
        );
        return Ok(());
    }

    if failed > 0 {
        println!(
            "\n{}",
            ui.note(&format!(
                "{failed} checks failed. Reboot postponed; see {}.",
                log::hint()
            ))
        );
        println!("Fix the issues and rerun the wizard, or reboot with: systemctl reboot");
        return Ok(());
    }

    if opts.first_boot {
        println!("Rebooting into the desktop...");
        let _ = Command::new("systemctl").arg("reboot").status();
    } else if ui.inline_confirm(&term, "Reboot now?", true)? {
        let _ = Command::new("systemctl").arg("reboot").status();
    } else {
        println!("You can reboot later with: systemctl reboot");
    }

    Ok(())
}

/// Susun baris ringkasan dari pilihan saat ini lalu minta konfirmasi.
#[allow(clippy::too_many_arguments)]
fn summarize(
    ui: &Ui,
    term: &Term,
    catalog: &Catalog,
    terminals: &[usize],
    file_managers: &[usize],
    browsers: &[usize],
    desktop: usize,
    login: Option<usize>,
) -> Result<Confirm> {
    let desktop_choice = &catalog.desktops[desktop];
    let picked_login = login.map(|i| &catalog.logins[i]);
    let login_choice = catalog.effective_login(desktop_choice, picked_login);

    let picks = install::Picks {
        terminals: terminals.iter().map(|&i| &catalog.terminals[i]).collect(),
        file_managers: file_managers
            .iter()
            .map(|&i| &catalog.file_managers[i])
            .collect(),
        browsers: browsers.iter().map(|&i| &catalog.browsers[i]).collect(),
        login: login_choice,
        desktop: desktop_choice,
    };
    let chosen = |list: &[catalog::Choice], idx: &[usize]| -> String {
        idx.iter()
            .map(|&i| list[i].name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let packages: Vec<String> = install::packages(catalog, &picks).into_iter().collect();
    let repos = if desktop_choice.id == "kineticwe" {
        catalog
            .repos
            .iter()
            .filter(|repo| repo.as_str() != "mindset")
            .cloned()
            .chain(std::iter::once("theblackdon/kwee".to_string()))
            .collect::<Vec<_>>()
            .join(", ")
    } else if catalog.repos.is_empty() {
        "None".to_string()
    } else {
        catalog.repos.join(", ")
    };
    let login_text = match (login_choice, picked_login) {
        (Some(l), Some(p)) if l.id == p.id => l.name.clone(),
        (Some(l), _) => format!("{} (automatic from {})", l.name, desktop_choice.name),
        (None, _) => format!("built-in for {}", desktop_choice.name),
    };
    let rows = [
        ("Desktop", desktop_choice.name.clone()),
        ("Terminal", chosen(&catalog.terminals, terminals)),
        (
            "File Manager",
            chosen(&catalog.file_managers, file_managers),
        ),
        ("Browser", chosen(&catalog.browsers, browsers)),
        ("Login", login_text),
        ("Repositories", repos),
        ("Target user", install::target_user()),
    ];
    ui.confirm_summary(term, &rows, &packages)
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim() == "0")
        .unwrap_or(false)
}

fn parse_args() -> Result<Opts> {
    let mut opts = Opts {
        dry_run: false,
        first_boot: false,
        ascii: false,
        catalog: None,
        base_dir: default_base_dir(),
        log_file: PathBuf::from(log::DEFAULT_PATH),
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("rakuos-bootstrap {VERSION}");
                std::process::exit(0);
            }
            "--dry-run" => opts.dry_run = true,
            "--first-boot" => opts.first_boot = true,
            "--ascii" => opts.ascii = true,
            "--catalog" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--catalog requires a path"))?;
                opts.catalog = Some(PathBuf::from(path));
            }
            "--base-dir" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--base-dir requires a path"))?;
                opts.base_dir = PathBuf::from(path);
            }
            "--log" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--log requires a path"))?;
                opts.log_file = PathBuf::from(path);
            }
            other => bail!("unknown argument: {other}\nRun `rakuos-bootstrap --help` for usage."),
        }
    }

    Ok(opts)
}

/// Lokasi data (katalog + skel). Dipilih yang benar-benar punya `skel/`:
/// terpasang di /usr/share, folder `data` di direktori kerja, atau `data`
/// di samping binary (mis. `target/release/../../data`).
fn default_base_dir() -> PathBuf {
    let mut candidates = vec![PathBuf::from(DEFAULT_BASE_DIR), PathBuf::from("data")];
    if let Ok(exe) = std::env::current_exe() {
        for up in exe.ancestors().skip(1).take(3) {
            candidates.push(up.join("data"));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.join("skel").is_dir())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BASE_DIR))
}

fn print_help() {
    println!(
        "\
rakuos-bootstrap {VERSION}
Post-install setup wizard for minimal RakuOS.

USAGE:
    sudo rakuos-bootstrap [OPTIONS]

OPTIONS:
    --dry-run           Show planned actions without changing the system
    --first-boot        Used by the systemd unit on first boot (reboots automatically)
    --ascii             Force plain ASCII output (also: RAKUOS_ASCII=1)
    --catalog <PATH>    Use a different TOML catalog
    --base-dir <PATH>   Catalog and skel directory (default: {DEFAULT_BASE_DIR})
    --log <PATH>        Log file (default: {})
    -h, --help          Show this help message
    -V, --version       Show version information
",
        log::DEFAULT_PATH
    );
}
