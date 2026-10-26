use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use console::style;

use crate::catalog::{Catalog, Choice};

pub struct Picks<'a> {
    pub terminal: &'a Choice,
    pub browser: &'a Choice,
    pub desktop: &'a Choice,
    pub file_manager: &'a Choice,
    pub login: &'a Choice,
}

/// Semua paket RPM: lapisan dasar + paket dari tiap pilihan.
pub fn packages(cat: &Catalog, p: &Picks) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = cat.base.iter().cloned().collect();
    for choice in [p.terminal, p.browser, p.desktop, p.file_manager, p.login] {
        set.extend(choice.packages.iter().cloned());
    }
    set
}

pub fn apply(cat: &Catalog, picks: &Picks, dry_run: bool, base_dir: &Path) -> Result<()> {
    let user = target_user();
    let pkgs = packages(cat, picks);

    println!("\n{}", style("Memasang paket").bold());
    install_packages(&pkgs, dry_run)?;

    if let Some(app_id) = &picks.browser.flatpak {
        println!("\n{}", style("Memasang Flatpak").bold());
        install_flatpak(app_id, dry_run)?;
    }

    println!("\n{}", style("Menyiapkan config desktop").bold());
    install_skel(
        picks.desktop,
        &picks.terminal.id,
        base_dir,
        &user,
        dry_run,
    )?;

    println!("\n{}", style("Menyiapkan login manager").bold());
    configure_login(picks.login, picks.desktop, &user, dry_run)?;

    write_state(picks, &user, dry_run)?;
    Ok(())
}

fn install_packages(pkgs: &BTreeSet<String>, dry_run: bool) -> Result<()> {
    if pkgs.is_empty() {
        return Ok(());
    }

    let (pm, overlay) = if command_exists("rum") {
        ("rum", true)
    } else {
        ("dnf", false)
    };
    if !overlay {
        eprintln!(
            "{} rum tidak ditemukan; memakai dnf (overlay RakuOS tidak aktif)",
            style("peringatan:").yellow().bold()
        );
    }

    let mut args: Vec<String> = vec!["install".into(), "-y".into()];
    args.extend(pkgs.iter().cloned());
    run(pm, &args, dry_run)
}

fn install_flatpak(app_id: &str, dry_run: bool) -> Result<()> {
    run(
        "flatpak",
        &[
            "remote-add".into(),
            "--if-not-exists".into(),
            "flathub".into(),
            "https://dl.flathub.org/repo/flathub.flatpakrepo".into(),
        ],
        dry_run,
    )?;
    run(
        "flatpak",
        &["install".into(), "-y".into(), "flathub".into(), app_id.into()],
        dry_run,
    )
}

fn install_skel(
    desktop: &Choice,
    terminal: &str,
    base_dir: &Path,
    user: &str,
    dry_run: bool,
) -> Result<()> {
    let Some(name) = &desktop.skel else {
        println!("  (desktop ini tidak membawa config minimal)");
        return Ok(());
    };

    let src = base_dir.join("skel").join(name);
    if !src.is_dir() {
        eprintln!(
            "{} skel {} tidak ada; config minimal dilewati",
            style("peringatan:").yellow().bold(),
            src.display()
        );
        return Ok(());
    }

    let home = home_for(user)?;
    if dry_run {
        println!("  + salin {} -> {}", src.display(), home.display());
        println!("  + ganti @TERMINAL@ dengan {terminal}");
        return Ok(());
    }

    copy_dir(&src, &home)?;
    replace_in_tree(&home.join(".config"), "@TERMINAL@", terminal)?;

    let config_dir = home.join(".config");
    let status = Command::new("chown")
        .args([
            "-R",
            &format!("{user}:{user}"),
            &config_dir.to_string_lossy(),
        ])
        .status()
        .context("chown gagal dijalankan")?;
    if !status.success() {
        bail!("chown -R {user}:{user} {} gagal", config_dir.display());
    }
    Ok(())
}

fn configure_login(
    login: &Choice,
    desktop: &Choice,
    user: &str,
    dry_run: bool,
) -> Result<()> {
    if let Some(unit) = &login.unit {
        run("systemctl", &["enable".into(), unit.clone()], dry_run)?;
        run(
            "systemctl",
            &["set-default".into(), "graphical.target".into()],
            dry_run,
        )?;

        // Bersihkan autologin kalau sebelumnya pernah dipilih login = none.
        let dropin = Path::new("/etc/systemd/system/getty@tty1.service.d/autologin.conf");
        if !dry_run && dropin.exists() {
            let _ = fs::remove_file(dropin);
        }
        return Ok(());
    }

    // login = "none": autologin user ke desktop, tanpa display manager.
    let cmd = desktop
        .autologin_cmd
        .as_deref()
        .with_context(|| format!("desktop {} tidak punya autologin_cmd", desktop.id))?;

    let dropin_dir = Path::new("/etc/systemd/system/getty@tty1.service.d");
    let dropin = dropin_dir.join("autologin.conf");
    let content = format!(
        "[Service]\nExecStart=\nExecStart=-/sbin/agetty -o '-p -- \\\\u' --noclear --autologin {user} %I $TERM\nType=idle\n"
    );

    if dry_run {
        println!("  + tulis {}", dropin.display());
        println!("  + systemctl set-default multi-user.target");
        println!("  + autologin: dbus-run-session {cmd}");
        return Ok(());
    }

    fs::create_dir_all(dropin_dir)?;
    fs::write(&dropin, content)?;
    run(
        "systemctl",
        &["set-default".into(), "multi-user.target".into()],
        false,
    )?;
    run(
        "systemctl",
        &["enable".into(), "getty@tty1.service".into()],
        false,
    )?;

    let home = home_for(user)?;
    let profile = home.join(".bash_profile");
    let snippet = format!(
        "\n# rakuos-bootstrap: autologin desktop di tty1\nif [ -z \"$WAYLAND_DISPLAY\" ] && [ \"$(tty)\" = \"/dev/tty1\" ]; then\n    exec dbus-run-session {cmd}\nfi\n"
    );
    let existing = fs::read_to_string(&profile).unwrap_or_default();
    if !existing.contains("rakuos-bootstrap: autologin") {
        fs::write(&profile, existing + &snippet)?;
    }
    let owner = format!("{user}:{user}");
    let profile_path = profile.to_string_lossy().to_string();
    let _ = Command::new("chown").args([owner, profile_path]).status();
    Ok(())
}

fn write_state(picks: &Picks, user: &str, dry_run: bool) -> Result<()> {
    let dir = Path::new("/var/lib/rakuos");
    let state = dir.join("bootstrap.toml");
    let done = dir.join("bootstrap.done");

    let body = format!(
        "# ditulis oleh rakuos-bootstrap\ntime = \"{}\"\ntarget_user = \"{}\"\nterminal = \"{}\"\nbrowser = \"{}\"\ndesktop = \"{}\"\nfile_manager = \"{}\"\nlogin = \"{}\"\n",
        iso8601_now(),
        user,
        picks.terminal.id,
        picks.browser.id,
        picks.desktop.id,
        picks.file_manager.id,
        picks.login.id,
    );

    if dry_run {
        println!("  + tulis {}\n  + tulis marker {}", state.display(), done.display());
        return Ok(());
    }

    fs::create_dir_all(dir)?;
    fs::write(&state, body)?;
    fs::write(&done, "")?;
    Ok(())
}

fn run(program: &str, args: &[String], dry_run: bool) -> Result<()> {
    if dry_run {
        println!("  + {} {}", program, args.join(" "));
        return Ok(());
    }

    println!(
        "  {} {}",
        style("→").cyan(),
        style(format!("{program} {}", args.join(" "))).dim()
    );
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("gagal menjalankan {program}"))?;
    if !status.success() {
        bail!("{program} keluar dengan {status}");
    }
    Ok(())
}

fn command_exists(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd}"))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn target_user() -> String {
    std::env::var("SUDO_USER")
        .ok()
        .filter(|u| u != "root")
        .unwrap_or_else(|| "root".to_string())
}

fn home_for(user: &str) -> Result<PathBuf> {
    let output = Command::new("getent")
        .args(["passwd", user])
        .output()
        .context("getent tidak bisa dijalankan")?;
    let line = String::from_utf8_lossy(&output.stdout);
    let home = line
        .trim()
        .split(':')
        .nth(5)
        .filter(|h| !h.is_empty())
        .with_context(|| format!("home directory untuk {user} tidak ditemukan"))?;
    Ok(PathBuf::from(home))
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn replace_in_tree(dir: &Path, needle: &str, value: &str) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            replace_in_tree(&path, needle, value)?;
        } else if path.is_file() {
            if let Ok(text) = fs::read_to_string(&path) {
                if text.contains(needle) {
                    fs::write(&path, text.replace(needle, value))?;
                }
            }
        }
    }
    Ok(())
}

fn iso8601_now() -> String {
    // Tanpa crate waktu: `date -Iseconds` selalu ada di Fedora.
    Command::new("date")
        .arg("+%Y-%m-%dT%H:%M:%S%z")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
