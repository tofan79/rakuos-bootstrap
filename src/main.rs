mod catalog;
mod install;

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Result};
use catalog::{Catalog, Choice};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_BASE_DIR: &str = "/usr/share/rakuos/bootstrap";

struct Opts {
    dry_run: bool,
    first_boot: bool,
    catalog: Option<PathBuf>,
    base_dir: PathBuf,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{} {err:#}", style("error:").red().bold());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let opts = parse_args()?;

    if !opts.dry_run && !is_root() {
        bail!("jalankan dengan sudo: sudo rakuos-bootstrap");
    }

    let catalog = Catalog::load(opts.catalog.as_deref())?;
    banner(&opts);

    let terminal = ask("Terminal", &catalog.terminals)?;
    let browser = ask("Browser", &catalog.browsers)?;
    let desktop = ask("Desktop", &catalog.desktops)?;
    let file_manager = ask("File manager", &catalog.file_managers)?;
    let login = ask("Login manager", &catalog.logins)?;

    let picks = install::Picks {
        terminal: &catalog.terminals[terminal],
        browser: &catalog.browsers[browser],
        desktop: &catalog.desktops[desktop],
        file_manager: &catalog.file_managers[file_manager],
        login: &catalog.logins[login],
    };

    summary(&catalog, &picks);

    let proceed = Confirm::with_theme(&theme())
        .with_prompt("Lanjutkan instalasi?")
        .default(true)
        .interact()?;
    if !proceed {
        println!("Dibatalkan. Tidak ada yang diubah.");
        return Ok(());
    }

    install::apply(&catalog, &picks, opts.dry_run, &opts.base_dir)?;

    println!("\n{}", style("Selesai.").green().bold());
    if opts.dry_run {
        println!(
            "{}",
            style("(dry-run: tidak ada paket yang dipasang, tidak ada file yang ditulis)").dim()
        );
        return Ok(());
    }

    if opts.first_boot {
        println!("Sistem akan reboot untuk masuk ke desktop...");
        let _ = Command::new("systemctl").arg("reboot").status();
    } else {
        let reboot = Confirm::with_theme(&theme())
            .with_prompt("Reboot sekarang?")
            .default(true)
            .interact()?;
        if reboot {
            let _ = Command::new("systemctl").arg("reboot").status();
        } else {
            println!("Reboot nanti dengan: systemctl reboot");
        }
    }

    Ok(())
}

fn theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

fn banner(opts: &Opts) {
    println!();
    println!(
        "{} {}",
        style("RakuOS bootstrap").bold().cyan(),
        style(format!("v{VERSION}")).dim()
    );
    println!(
        "{}",
        style("Pilih 5 hal dasar; sisanya bisa diubah kapan saja dengan `sudo rakuos-bootstrap`.").dim()
    );
    if opts.first_boot {
        println!("{}", style("Mode boot pertama: wizard berjalan di tty1.").yellow());
    }
    if opts.dry_run {
        println!("{}", style("Mode dry-run: hanya menampilkan rencana.").yellow());
    }
    println!();
}

/// Kembalikan indeks pilihan di `choices` (bukan indeks di daftar yang tampil).
fn ask(title: &str, choices: &[Choice]) -> Result<usize> {
    let available: Vec<usize> = choices
        .iter()
        .enumerate()
        .filter(|(_, choice)| choice.available)
        .map(|(index, _)| index)
        .collect();

    if available.is_empty() {
        bail!("tidak ada pilihan tersedia untuk {title}");
    }

    let items: Vec<String> = available
        .iter()
        .map(|&index| {
            let choice = &choices[index];
            format!(
                "{:<20} {}",
                choice.name,
                style(choice.description.as_str()).dim()
            )
        })
        .collect();

    let selected = Select::with_theme(&theme())
        .with_prompt(title)
        .items(&items)
        .default(0)
        .interact()?;

    Ok(available[selected])
}

fn summary(catalog: &Catalog, picks: &install::Picks) {
    let packages = install::packages(catalog, picks);

    println!("\n{}", style("Ringkasan").bold());
    println!("  {:<14}{}", "Terminal", picks.terminal.name);
    println!("  {:<14}{}", "Browser", picks.browser.name);
    println!("  {:<14}{}", "Desktop", picks.desktop.name);
    println!("  {:<14}{}", "File manager", picks.file_manager.name);
    println!("  {:<14}{}", "Login", picks.login.name);
    println!("  {:<14}{}", "User target", install::target_user());

    if let Some(app_id) = &picks.browser.flatpak {
        println!("  {:<14}{}", "Flatpak", app_id);
    }

    println!("\n  Paket RPM ({}):", packages.len());
    for package in &packages {
        println!("    - {package}");
    }
    if picks.browser.flatpak.is_some() {
        println!("    - (flatpak: unduhan pertama bisa ratusan MB)");
    }
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
        catalog: None,
        base_dir: default_base_dir(),
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
            "--catalog" => {
                let path = args.next().ok_or_else(|| anyhow::anyhow!("--catalog butuh path"))?;
                opts.catalog = Some(PathBuf::from(path));
            }
            "--base-dir" => {
                let path = args.next().ok_or_else(|| anyhow::anyhow!("--base-dir butuh path"))?;
                opts.base_dir = PathBuf::from(path);
            }
            other => bail!("argumen tidak dikenal: {other}\nCoba `rakuos-bootstrap --help`."),
        }
    }

    Ok(opts)
}

fn default_base_dir() -> PathBuf {
    let installed = PathBuf::from(DEFAULT_BASE_DIR);
    if installed.is_dir() {
        installed
    } else {
        PathBuf::from("data")
    }
}

fn print_help() {
    println!(
        "\
rakuos-bootstrap {VERSION}
Wizard pasca-install untuk RakuOS minimal.

USAGE:
    sudo rakuos-bootstrap [OPTIONS]

OPTIONS:
    --dry-run           Tampilkan rencana, jangan ubah sistem
    --first-boot        Dipakai unit systemd saat boot pertama (auto reboot)
    --catalog <PATH>    Pakai katalog TOML lain
    --base-dir <PATH>   Lokasi katalog/skel (default: {DEFAULT_BASE_DIR})
    -h, --help          Tampilkan bantuan
    -V, --version       Tampilkan versi
"
    );
}
