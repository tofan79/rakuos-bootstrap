mod catalog;
mod install;
mod ui;

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Result};
use catalog::Catalog;
use console::Term;
use ui::{Cancelled, Ui};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_BASE_DIR: &str = "/usr/share/rakuos/bootstrap";

struct Opts {
    dry_run: bool,
    first_boot: bool,
    ascii: bool,
    catalog: Option<PathBuf>,
    base_dir: PathBuf,
}

fn main() {
    if let Err(err) = run() {
        if err.downcast_ref::<Cancelled>().is_some() {
            println!("\nDibatalkan. Tidak ada yang diubah.");
            std::process::exit(130);
        }
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let opts = parse_args()?;

    if !opts.dry_run && !is_root() {
        bail!("jalankan dengan sudo: sudo rakuos-bootstrap");
    }

    let term = Term::stdout();
    if !term.is_term() {
        bail!("butuh terminal interaktif (TTY)");
    }

    let catalog = Catalog::load(opts.catalog.as_deref())?;
    let badge = if opts.dry_run {
        Some("DRY-RUN".to_string())
    } else if opts.first_boot {
        Some("BOOT PERTAMA".to_string())
    } else {
        None
    };
    let ui = Ui::new(&term, opts.ascii, VERSION, badge);

    // Urutan fase = urutan tampil. Ubah di sini (dan STEPS di ui.rs) kalau mau ganti.
    let phases: [(&str, &str, &[catalog::Choice]); 5] = [
        ("Terminal", "Tempat mengetik perintah", &catalog.terminals),
        (
            "File manager",
            "Menjelajah file dan mount USB/HP",
            &catalog.file_managers,
        ),
        (
            "Browser",
            "Pilih satu atau lebih; semuanya paket RPM",
            &catalog.browsers,
        ),
        ("Login manager", "Layar masuk setelah boot", &catalog.logins),
        (
            "Desktop",
            "Compositor Wayland; config minimal tanpa bar",
            &catalog.desktops,
        ),
    ];

    // Tiga fase pertama boleh lebih dari satu pilihan; login dan desktop
    // tetap tunggal karena hanya satu yang bisa aktif saat boot.
    let mut picked: Vec<String> = Vec::new();

    let terminals = ui.multi_select(
        &term,
        0,
        phases[0].0,
        phases[0].1,
        &catalog.terminals,
        &picked,
    )?;
    picked.extend(terminals.iter().map(|&i| catalog.terminals[i].name.clone()));

    let file_managers = ui.multi_select(
        &term,
        1,
        phases[1].0,
        phases[1].1,
        &catalog.file_managers,
        &picked,
    )?;
    picked.extend(
        file_managers
            .iter()
            .map(|&i| catalog.file_managers[i].name.clone()),
    );

    let browsers = ui.multi_select(
        &term,
        2,
        phases[2].0,
        phases[2].1,
        &catalog.browsers,
        &picked,
    )?;
    picked.extend(browsers.iter().map(|&i| catalog.browsers[i].name.clone()));

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

    let login_pos = ui.select(&term, 3, phases[3].0, phases[3].1, &visible_logins, &picked)?;
    let login = login_indices[login_pos];
    picked.push(catalog.logins[login].name.clone());

    let desktop = ui.select(
        &term,
        4,
        phases[4].0,
        phases[4].1,
        &catalog.desktops,
        &picked,
    )?;

    let picked_login = &catalog.logins[login];
    let desktop_choice = &catalog.desktops[desktop];
    let login_choice = catalog.effective_login(desktop_choice, picked_login);

    let chosen = |list: &[catalog::Choice], idx: &[usize]| -> Vec<String> {
        idx.iter().map(|&i| list[i].name.clone()).collect()
    };

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

    let packages: Vec<String> = install::packages(&catalog, &picks).into_iter().collect();
    let repos = if catalog.repos.is_empty() {
        "tidak ada".to_string()
    } else {
        catalog.repos.join(", ")
    };
    let rows = [
        (
            "Terminal",
            chosen(&catalog.terminals, &terminals).join(", "),
        ),
        (
            "File manager",
            chosen(&catalog.file_managers, &file_managers).join(", "),
        ),
        ("Browser", chosen(&catalog.browsers, &browsers).join(", ")),
        (
            "Login",
            if picks.login.id == picked_login.id {
                picks.login.name.clone()
            } else {
                format!(
                    "{} (otomatis dari {})",
                    picks.login.name, picks.desktop.name
                )
            },
        ),
        ("Desktop", picks.desktop.name.clone()),
        ("Repo pihak ketiga", repos),
        ("User target", install::target_user()),
    ];

    if !ui.confirm_summary(&term, &rows, &packages)? {
        println!("\nDibatalkan. Tidak ada yang diubah.");
        return Ok(());
    }

    ui.begin_log(&term)?;
    install::apply(&catalog, &picks, opts.dry_run, &opts.base_dir)?;

    println!("\n{}", ui.success("Selesai."));
    if opts.dry_run {
        println!(
            "{}",
            ui.note("(dry-run: tidak ada paket yang dipasang, tidak ada file yang ditulis)")
        );
        return Ok(());
    }

    if opts.first_boot {
        println!("Sistem akan reboot untuk masuk ke desktop...");
        let _ = Command::new("systemctl").arg("reboot").status();
    } else if ui.inline_confirm(&term, "Reboot sekarang?", true)? {
        let _ = Command::new("systemctl").arg("reboot").status();
    } else {
        println!("Reboot nanti dengan: systemctl reboot");
    }

    Ok(())
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
                    .ok_or_else(|| anyhow::anyhow!("--catalog butuh path"))?;
                opts.catalog = Some(PathBuf::from(path));
            }
            "--base-dir" => {
                let path = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--base-dir butuh path"))?;
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
    --ascii             Paksa tampilan ASCII murni (juga: RAKUOS_ASCII=1)
    --catalog <PATH>    Pakai katalog TOML lain
    --base-dir <PATH>   Lokasi katalog/skel (default: {DEFAULT_BASE_DIR})
    -h, --help          Tampilkan bantuan
    -V, --version       Tampilkan versi
"
    );
}
