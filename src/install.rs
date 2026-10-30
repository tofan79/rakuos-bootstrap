use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use console::style;

use crate::catalog::{Catalog, Choice};
use crate::log;
use crate::verify::{verify, Check};

/// Peringatan: tampil di layar dan masuk log.
macro_rules! warn {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        eprintln!("{} {msg}", style("peringatan:").yellow().bold());
        log::note(&format!("peringatan: {msg}"));
    }};
}

/// Judul tahap: tampil tebal di layar dan masuk log.
fn heading(text: &str) {
    println!("\n{}", style(text).bold());
    log::note(&format!("\n## {text}"));
}

pub struct Picks<'a> {
    pub terminals: Vec<&'a Choice>,
    pub browsers: Vec<&'a Choice>,
    pub file_managers: Vec<&'a Choice>,
    pub desktop: &'a Choice,
    /// `None` = desktop memakai login bawaannya sendiri.
    pub login: Option<&'a Choice>,
}

impl Picks<'_> {
    /// Terminal yang dipakai untuk placeholder `@TERMINAL@` di skel:
    /// pilihan pertama menurut abjad (daftar sudah terurut).
    fn primary_terminal(&self) -> &str {
        self.terminals
            .first()
            .map(|c| c.command())
            .unwrap_or("foot")
    }

    fn primary_browser(&self) -> &str {
        self.browsers
            .first()
            .map(|c| c.command())
            .unwrap_or("firefox")
    }

    fn primary_file_manager(&self) -> &str {
        self.file_managers
            .first()
            .map(|c| c.command())
            .unwrap_or("nautilus")
    }
}

/// Semua paket RPM: lapisan dasar + paket dari tiap pilihan.
pub fn packages(cat: &Catalog, p: &Picks) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = cat.base.iter().cloned().collect();
    for choice in p
        .terminals
        .iter()
        .chain(&p.browsers)
        .chain(&p.file_managers)
        .copied()
        .chain([p.desktop])
        .chain(p.login)
    {
        set.extend(choice.packages.iter().cloned());
    }
    set
}

pub fn apply(cat: &Catalog, picks: &Picks, dry_run: bool, base_dir: &Path) -> Result<Vec<Check>> {
    let user = target_user();
    let pkgs = packages(cat, picks);

    heading("Menyiapkan repo pihak ketiga");
    enable_repos(cat, dry_run)?;

    heading("Menyegarkan metadata repo");
    refresh_metadata(dry_run)?;

    heading("Memasang paket");
    let no_recommends: &[String] = picks.login.map_or(&[], |l| l.no_recommends.as_slice());
    if let Some(login) = picks.login {
        resolve_greeter_conflict(login, dry_run)?;
        resolve_dms_quickshell(login, dry_run)?;
    }
    install_packages(&pkgs, no_recommends, dry_run)?;

    heading("Menyiapkan config desktop");
    install_skel(
        picks.desktop,
        picks.primary_terminal(),
        picks.primary_browser(),
        picks.primary_file_manager(),
        base_dir,
        &user,
        dry_run,
    )?;
    install_session_file(picks.desktop, base_dir, dry_run)?;
    ensure_wayland_session(picks.desktop, dry_run)?;

    heading("Menyiapkan login manager");
    configure_login(picks.login, picks.desktop, &user, dry_run)?;

    write_state(cat, picks, &user, dry_run)?;

    heading("Verifikasi");
    if dry_run {
        println!("  + cek paket, display manager, sesi Wayland, dan skel");
        return Ok(Vec::new());
    }
    let checks = verify(cat, picks, &user, base_dir);
    for c in &checks {
        let mark = if c.ok {
            style("✔").green()
        } else {
            style("✘").red().bold()
        };
        println!("  {mark} {}", c.label);
        log::note(&format!(
            "verifikasi: {} {}",
            if c.ok { "OK  " } else { "GAGAL" },
            c.label
        ));
    }
    Ok(checks)
}

/// Menyalakan repo pihak ketiga yang diminta katalog.
///
/// Terra dan RPM Fusion dibawa image dalam keadaan `enabled=0`. Perilakunya
/// disamakan dengan rakuos-welcome: Terra cukup satu id, RPM Fusion memakai
/// daftar di `/usr/share/rakuos/rpmfusion-default-repos` (ditulis
/// rakuos-base) supaya tidak ikut menyalakan repo source/debuginfo.
fn enable_repos(cat: &Catalog, dry_run: bool) -> Result<()> {
    if cat.repos.is_empty() {
        println!("  (tidak ada repo pihak ketiga yang diminta)");
        return Ok(());
    }

    if !command_exists("rum") {
        warn!("rum tidak ditemukan; repo pihak ketiga dilewati",);
        return Ok(());
    }

    for repo in &cat.repos {
        let args = match repo.as_str() {
            "terra" => vec![
                "config-manager".into(),
                "--set-enabled".into(),
                "terra".into(),
            ],
            "rpmfusion" => rpmfusion_args(),
            "mindset" => vec![
                "copr".into(),
                "enable".into(),
                "-y".into(),
                "mindset/Mindset-Apps".into(),
                "fedora-45-x86_64".into(),
            ],
            other => bail!("repo pihak ketiga tidak dikenal: {other}"),
        };
        run("rum", &args, dry_run)?;
    }
    Ok(())
}

/// Segarkan metadata setelah repo pihak ketiga dinyalakan. `rum make-cache`
/// hanya mengambil metadata (tidak menyentuh paket overlay seperti
/// `rum update`), jadi aman dijalankan sebelum instalasi.
fn refresh_metadata(dry_run: bool) -> Result<()> {
    if command_exists("rum") {
        run("rum", &["make-cache".into()], dry_run)
    } else {
        run("dnf", &["makecache".into()], dry_run)
    }
}

/// Argumen `rum config-manager` untuk RPM Fusion, mengikuti rakuos-welcome.
fn rpmfusion_args() -> Vec<String> {
    const DEFAULTS: &str = "/usr/share/rakuos/rpmfusion-default-repos";

    let mut args: Vec<String> = vec!["config-manager".into(), "--set-enabled".into()];
    let defaults: Vec<String> = fs::read_to_string(DEFAULTS)
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    if defaults.is_empty() {
        args.push("rpmfusion-*".into());
    } else {
        args.extend(defaults);
    }
    args
}

fn install_packages(
    pkgs: &BTreeSet<String>,
    no_recommends: &[String],
    dry_run: bool,
) -> Result<()> {
    if pkgs.is_empty() {
        return Ok(());
    }

    let (pm, overlay) = package_manager();
    if !overlay {
        warn!("rum tidak ditemukan; memakai dnf (overlay RakuOS tidak aktif)",);
    }

    let tanpa_rekomendasi: BTreeSet<&str> = no_recommends.iter().map(String::as_str).collect();
    let khusus: Vec<String> = pkgs
        .iter()
        .filter(|p| tanpa_rekomendasi.contains(p.as_str()))
        .cloned()
        .collect();
    let biasa: Vec<String> = pkgs
        .iter()
        .filter(|p| !tanpa_rekomendasi.contains(p.as_str()))
        .cloned()
        .collect();

    if !khusus.is_empty() {
        install_set(pm, &khusus, true, dry_run)?;
    }
    if !biasa.is_empty() {
        install_set(pm, &biasa, false, dry_run)?;
    }
    Ok(())
}

fn package_manager() -> (&'static str, bool) {
    if command_exists("rum") {
        ("rum", true)
    } else {
        ("dnf", false)
    }
}

fn install_set(pm: &str, pkgs: &[String], no_recommends: bool, dry_run: bool) -> Result<()> {
    let mut args: Vec<String> = vec!["install".into(), "-y".into()];
    if no_recommends {
        args.push("--no-recommends".into());
    }
    args.extend(pkgs.iter().cloned());
    run(pm, &args, dry_run)
}

/// DMS Greeter butuh `quickshell` atau `quickshell-git`; paket resmi memilih
/// yang git agar satu runtime dengan shell DMS. Jangan masukkan langsung ke
/// daftar paket: kalau `quickshell` biasa sudah ada, keduanya bertabrakan.
fn resolve_dms_quickshell(login: &Choice, dry_run: bool) -> Result<()> {
    if login.id != "dms-greeter"
        || package_installed("quickshell")
        || package_installed("quickshell-git")
    {
        return Ok(());
    }
    let (pm, _) = package_manager();
    run(
        pm,
        &["install".into(), "-y".into(), "quickshell-git".into()],
        dry_run,
    )
}

/// `noctalia-greeter` (Terra) dan `kineticwe-greeter` memiliki file yang
/// sama, jadi tidak boleh terpasang bersamaan. Saat wizard dijalankan ulang
/// dengan pilihan lain, singkirkan paket yang bertabrakan lebih dulu.
fn resolve_greeter_conflict(login: &Choice, dry_run: bool) -> Result<()> {
    let conflicting = match login.id.as_str() {
        "kineticwe-greeter" => "noctalia-greeter",
        "noctalia-greeter" => "kineticwe-greeter",
        _ => return Ok(()),
    };
    if !package_installed(conflicting) {
        return Ok(());
    }
    let pm = if command_exists("rum") { "rum" } else { "dnf" };
    run(
        pm,
        &["remove".into(), "-y".into(), conflicting.to_string()],
        dry_run,
    )
}

pub(crate) fn package_installed(name: &str) -> bool {
    Command::new("rpm")
        .args(["-q", "--quiet", name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn install_skel(
    desktop: &Choice,
    terminal: &str,
    browser: &str,
    file_manager: &str,
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
        bail!(
            "skel {} tidak ditemukan; config desktop tidak bisa disalin. \
             Cek --base-dir (sekarang {})",
            src.display(),
            base_dir.display()
        );
    }

    let home = home_for(user)?;
    if dry_run {
        println!("  + salin {} -> {}", src.display(), home.display());
        println!("  + config yang sudah ada dicadangkan ke <nama>.bak");
        println!("  + ganti @TERMINAL@ dengan {terminal}");
        println!("  + ganti @BROWSER@ dengan {browser}");
        println!("  + ganti @FILEMANAGER@ dengan {file_manager}");
        return Ok(());
    }

    let subs = [
        ("@TERMINAL@", terminal),
        ("@BROWSER@", browser),
        ("@FILEMANAGER@", file_manager),
    ];
    let copied = copy_dir(&src, &home, &subs)?;
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
    for file in &copied {
        let line = match &file.backup {
            Some(bak) => format!(
                "{} (config lama dicadangkan ke {})",
                file.path.display(),
                bak.display()
            ),
            None => file.path.display().to_string(),
        };
        println!("  {} {line}", style("\u{2192}").cyan());
        log::note(&format!("skel: {line}"));
    }
    Ok(())
}

/// Timpa file sesi Wayland milik paket (mis. `hyprland-uwsm.desktop`) dengan
/// versi yang kita bawa di `data/sessions/`. Dipakai kalau file bawaan paket
/// kurang pas — mis. `hyprland-uwsm` belum menyetel `UWSM_SILENT_START=2`,
/// sehingga log uwsm sempat tampil di VT sebelum compositor muncul.
fn install_session_file(desktop: &Choice, base_dir: &Path, dry_run: bool) -> Result<()> {
    let Some(name) = &desktop.session_file else {
        return Ok(());
    };

    let src = base_dir.join("sessions").join(name);
    if !src.is_file() {
        warn!("sesi {} tidak ada; file sesi dilewati", src.display());
        return Ok(());
    }

    let dst = Path::new("/usr/share/wayland-sessions").join(name);
    if dry_run {
        println!("  + salin {} -> {}", src.display(), dst.display());
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            if is_immutable_fs_error(&err) {
                warn!("skel sesi {} tidak bisa dipasang ke {} karena {} (root immutable); versi paket tetap dipakai",
                    name,
                    dst.display(),
                    err
                );
                return Ok(());
            }
            return Err(err).with_context(|| format!("tidak bisa membuat {}", parent.display()));
        }
    }
    if let Err(err) = fs::copy(&src, &dst) {
        if is_immutable_fs_error(&err) {
            warn!("skel sesi {} tidak bisa memodifikasi {} karena {} (root immutable); versi paket tetap dipakai",
                    name,
                dst.display(),
                err
            );
            return Ok(());
        }
        return Err(err).with_context(|| format!("tidak bisa menulis {}", dst.display()));
    }
    Ok(())
}

/// Menentukan apakah galat dari sistem berkas immutable (bootc/RakuOS).
/// Kita sengaja memperlakukan ini sebagai peringatan, bukan kegagalan fatal,
/// karena sesi bawaan paket di `/usr/share/wayland-sessions` tetap bisa dipakai.
fn is_immutable_fs_error(err: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    matches!(
        err.kind(),
        ErrorKind::PermissionDenied | ErrorKind::Unsupported
    ) || err
        .to_string()
        .to_ascii_lowercase()
        .contains("read-only file system")
}

/// Aktifkan satu display manager: matikan yang lain, enable unit ini, target
/// grafis, dan bersihkan autologin tty1 dari pilihan lama.
fn activate_display_manager(unit: &str, dry_run: bool) -> Result<()> {
    disable_other_managers(Some(unit), dry_run)?;
    run("systemctl", &["enable".into(), unit.to_string()], dry_run)?;
    run(
        "systemctl",
        &["set-default".into(), "graphical.target".into()],
        dry_run,
    )?;
    let dropin = Path::new("/etc/systemd/system/getty@tty1.service.d/autologin.conf");
    if !dry_run && dropin.exists() {
        let _ = fs::remove_file(dropin);
    }
    Ok(())
}

/// Desktop tanpa pilihan login (Budgie, LXQt, Umbriel): pakai display manager
/// yang ikut terpasang bersama desktop. Config login tidak disentuh.
fn use_builtin_login(dry_run: bool) -> Result<()> {
    const CANDIDATES: &[&str] = &[
        "lightdm.service",
        "sddm.service",
        "gdm.service",
        "greetd.service",
    ];
    if dry_run {
        println!("  + pakai login bawaan desktop (lightdm/sddm/gdm/greetd yang terpasang)");
        return Ok(());
    }
    match CANDIDATES.iter().find(|u| unit_exists(u)) {
        Some(unit) => activate_display_manager(unit, false),
        None => {
            warn!("desktop ini tidak membawa display manager; login manager tidak diubah",);
            Ok(())
        }
    }
}

fn unit_exists(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["cat", unit])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn configure_login(
    login: Option<&Choice>,
    desktop: &Choice,
    user: &str,
    dry_run: bool,
) -> Result<()> {
    let Some(login) = login else {
        return use_builtin_login(dry_run);
    };

    if let Some(unit) = &login.unit {
        activate_display_manager(unit, dry_run)?;
        if let Some(theme) = &login.sddm_theme {
            configure_sddm(theme, dry_run)?;
        }
        if login.greetd_cmd.is_some() {
            configure_greetd(login, desktop, dry_run)?;
        }
        return Ok(());
    }

    disable_other_managers(None, dry_run)?;

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

/// Tulis config SDDM yang dipakai `rakuos-hyprland greeter`: tema Maldives,
/// cursor Bibata kalau ada, dan `InputMethod=` kosong supaya keyboard virtual
/// tidak menutupi kolom password.
fn configure_sddm(theme: &str, dry_run: bool) -> Result<()> {
    let conf = Path::new("/etc/sddm.conf.d/50-rakuos-theme.conf");
    let cursor = ["Bibata-Modern-Ice", "Bibata-Modern-Classic", "Adwaita"]
        .iter()
        .find(|name| {
            Path::new("/usr/share/icons")
                .join(name)
                .join("cursors")
                .is_dir()
        })
        .copied();
    let body = sddm_config_body(theme, cursor);

    let sessions = Path::new(SDDM_SESSIONS_CONF);

    if dry_run {
        println!(
            "  + tulis {} (Current={theme}, CursorTheme={}, InputMethod=)",
            conf.display(),
            cursor.unwrap_or("tidak ada")
        );
        println!(
            "  + tulis {} (greeter x11, SessionDir wayland-sessions)",
            sessions.display()
        );
        return Ok(());
    }

    if let Some(parent) = conf.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(conf, body)?;
    fs::write(sessions, sddm_sessions_body())?;
    Ok(())
}

const SDDM_SESSIONS_CONF: &str = "/etc/sddm.conf.d/60-rakuos-sessions.conf";

/// Greeter tetap `x11` (paket `sddm-x11`), tetapi daftar sesinya ditulis
/// eksplisit supaya sesi Wayland (Hyprland, Niri, MangoWM, Sway) selalu
/// terbaca. Nomor 60 mengalahkan drop-in vendor yang mungkin memaksa
/// `DisplayServer=wayland` padahal greeter Wayland tidak terpasang.
pub(crate) fn sddm_sessions_body() -> String {
    "[General]\nDisplayServer=x11\n\n\
     [Wayland]\nSessionDir=/usr/local/share/wayland-sessions,/usr/share/wayland-sessions\n\
     SessionCommand=/usr/share/sddm/scripts/wayland-session\n\n\
     [X11]\nSessionDir=/usr/local/share/xsessions,/usr/share/xsessions\n"
        .to_string()
}

/// Ada file sesi di `/usr/share/wayland-sessions` yang menjalankan `cmd`?
pub(crate) fn wayland_session_exists(cmd: &str) -> bool {
    fs::read_dir("/usr/share/wayland-sessions")
        .map(|entries| {
            entries.flatten().any(|e| {
                fs::read_to_string(e.path())
                    .map(|t| {
                        t.lines()
                            .any(|l| l.strip_prefix("Exec=").is_some_and(|x| x.trim() == cmd))
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Pastikan desktop pilihan punya file sesi di `/usr/share/wayland-sessions`.
/// Kalau paket tidak membawanya, tulis `rakuos-<id>.desktop` dari
/// `autologin_cmd` supaya login manager apa pun bisa menampilkannya.
/// Hanya untuk desktop yang menawarkan pilihan login.
fn ensure_wayland_session(desktop: &Choice, dry_run: bool) -> Result<()> {
    if !desktop.choose_login || desktop.session_file.is_some() {
        return Ok(());
    }
    let Some(cmd) = desktop.autologin_cmd.as_deref() else {
        return Ok(());
    };
    let dir = Path::new("/usr/share/wayland-sessions");

    if dry_run {
        println!(
            "  + pastikan ada sesi Wayland untuk {} (Exec={cmd})",
            desktop.name
        );
        return Ok(());
    }

    let already = wayland_session_exists(cmd);
    if already {
        return Ok(());
    }

    let dst = dir.join(format!("rakuos-{}.desktop", desktop.id));
    let body = format!(
        "[Desktop Entry]\nName={}\nComment={}\nExec={cmd}\nTryExec={}\nType=Application\nDesktopNames={}\n",
        desktop.name,
        desktop.description,
        cmd.split_whitespace().next().unwrap_or(cmd),
        desktop.name
    );
    let write = fs::create_dir_all(dir).and_then(|_| fs::write(&dst, body));
    match write {
        Ok(()) => Ok(()),
        Err(err) if is_immutable_fs_error(&err) => {
            warn!(
                "sesi {} tidak bisa ditulis ({err}); root immutable",
                dst.display()
            );
            Ok(())
        }
        Err(err) => Err(err).with_context(|| format!("tidak bisa menulis {}", dst.display())),
    }
}

pub(crate) fn sddm_config_body(theme: &str, cursor: Option<&str>) -> String {
    let mut body = format!("[Theme]\nCurrent={theme}\n");
    if let Some(cursor) = cursor {
        body.push_str(&format!("CursorTheme={cursor}\n"));
    }
    body.push_str("\n[General]\nInputMethod=\n");
    body
}

/// Matikan display manager lain agar hanya satu yang start saat boot.
/// Menjalankan wizard ulang dengan pilihan berbeda tidak boleh meninggalkan
/// dua DM aktif. `active = None` berarti autologin tanpa DM.
fn disable_other_managers(active: Option<&str>, dry_run: bool) -> Result<()> {
    const UNITS: &[&str] = &[
        "cosmic-greeter.service",
        "gdm.service",
        "greetd.service",
        "lightdm.service",
        "ly.service",
        "plasmalogin.service",
        "sddm.service",
    ];

    for unit in UNITS {
        if active == Some(unit) || !unit_enabled(unit) {
            continue;
        }
        run(
            "systemctl",
            &["disable".into(), (*unit).to_string()],
            dry_run,
        )?;
    }
    Ok(())
}

pub(crate) fn unit_enabled(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["is-enabled", unit])
        .output()
        .map(|out| {
            let state = String::from_utf8_lossy(&out.stdout);
            state.trim().starts_with("enabled")
        })
        .unwrap_or(false)
}

/// Tulis `/etc/greetd/config.toml` untuk greeter berbasis greetd (DMS,
/// Noctalia, sysc-greet). Config lama disimpan sekali sebagai `config.toml.bak`.
fn configure_greetd(login: &Choice, desktop: &Choice, dry_run: bool) -> Result<()> {
    let cmd = login
        .greetd_cmd
        .as_deref()
        .unwrap_or_default()
        .replace("@COMPOSITOR@", compositor_for(desktop));
    let user = login.greetd_user.as_deref().unwrap_or("greeter");
    let conf = Path::new("/etc/greetd/config.toml");

    if user != "greetd" {
        if !user_exists(user) {
            run(
                "useradd",
                &[
                    "-r".into(),
                    "-m".into(),
                    "-d".into(),
                    format!("/var/lib/{user}"),
                    "-s".into(),
                    "/usr/sbin/nologin".into(),
                    user.to_string(),
                ],
                dry_run,
            )?;
        }
        if dry_run {
            println!("  + tambah {user} ke grup video, render, input");
        } else {
            add_to_groups(user);
        }
    }

    if dry_run {
        println!(
            "  + tulis {} (command = {cmd}, user = {user})",
            conf.display()
        );
        if let Some(setup) = &login.greetd_setup {
            println!("  + jalankan {setup}");
        }
        return Ok(());
    }

    let backup = Path::new("/etc/greetd/config.toml.bak");
    if conf.exists() && !backup.exists() {
        fs::copy(conf, backup).context("gagal mencadangkan config greetd")?;
    }
    if let Some(parent) = conf.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = format!(
        "# ditulis oleh rakuos-bootstrap\n[terminal]\nvt = 1\n\n[default_session]\ncommand = \"{cmd}\"\nuser = \"{user}\"\n"
    );
    fs::write(conf, body)?;

    if let Some(setup) = &login.greetd_setup {
        run(setup, &[], false)?;
    }
    Ok(())
}

fn compositor_for(desktop: &Choice) -> &'static str {
    match desktop.id.as_str() {
        "budgie" => "startbudgielabwc",
        "lxqt" => "startlxqtwayland",
        "niri" => "niri",
        "sway" => "sway",
        "umbriel" => "start-umbriel",
        "labwc" => "labwc",
        "mangowm" => "mango",
        _ => "hyprland",
    }
}

fn user_exists(user: &str) -> bool {
    Command::new("id")
        .args(["-u", user])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Greeter grafis butuh akses GPU; gagal satu grup tidak fatal.
fn add_to_groups(user: &str) {
    for group in ["video", "render", "input"] {
        let _ = Command::new("usermod").args(["-aG", group, user]).status();
    }
}

fn write_state(cat: &Catalog, picks: &Picks, user: &str, dry_run: bool) -> Result<()> {
    let dir = Path::new("/var/lib/rakuos");
    let state = dir.join("bootstrap.toml");
    let done = dir.join("bootstrap.done");

    let ids = |list: &[&Choice]| -> String {
        list.iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let body = format!(
        "# ditulis oleh rakuos-bootstrap\ntime = \"{}\"\ntarget_user = \"{}\"\nterminals = \"{}\"\nbrowsers = \"{}\"\ndesktop = \"{}\"\nfile_managers = \"{}\"\nlogin = \"{}\"\nrepos = \"{}\"\n",
        iso8601_now(),
        user,
        ids(&picks.terminals),
        ids(&picks.browsers),
        picks.desktop.id,
        ids(&picks.file_managers),
        picks.login.map_or("bawaan", |l| l.id.as_str()),
        cat.repos.join(" "),
    );

    if dry_run {
        println!(
            "  + tulis {}\n  + tulis marker {}",
            state.display(),
            done.display()
        );
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

    let line = format!("{program} {}", args.join(" "));
    println!("  {} {}", style("→").cyan(), style(&line).dim());
    log::note(&format!("$ {line}"));

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("gagal menjalankan {program}"))?;

    // Keluaran ditampilkan apa adanya di layar dan dicatat ke log.
    let out = child.stdout.take().map(|r| tee(r, false));
    let err = child.stderr.take().map(|r| tee(r, true));
    let status = child.wait()?;
    for t in [out, err].into_iter().flatten() {
        let _ = t.join();
    }

    log::note(&format!("  -> {status}"));
    if !status.success() {
        bail!("{program} keluar dengan {status} (lihat {})", log::hint());
    }
    Ok(())
}

/// Salin aliran keluaran proses ke layar (stdout/stderr) dan ke log.
fn tee<R: Read + Send + 'static>(mut reader: R, to_stderr: bool) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut logger = log::LineLogger::default();
        let mut buf = [0u8; 4096];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            if to_stderr {
                let mut e = std::io::stderr();
                let _ = e.write_all(&buf[..n]);
                let _ = e.flush();
            } else {
                let mut o = std::io::stdout();
                let _ = o.write_all(&buf[..n]);
                let _ = o.flush();
            }
            logger.feed(&buf[..n]);
        }
        logger.finish();
    })
}

pub(crate) fn command_exists(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd}"))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// User yang menerima config desktop. Urutan: `SUDO_USER`; kalau tidak ada
/// (mis. unit systemd `--first-boot`, atau `su -`), user biasa pertama di
/// sistem (UID 1000-59999, shell login, home ada); terakhir `root`.
/// Tanpa fallback ini config mendarat di `/root/.config`.
pub fn target_user() -> String {
    std::env::var("SUDO_USER")
        .ok()
        .filter(|u| !u.is_empty() && u != "root")
        .or_else(first_regular_user)
        .unwrap_or_else(|| "root".to_string())
}

fn first_regular_user() -> Option<String> {
    let output = Command::new("getent").arg("passwd").output().ok()?;
    parse_regular_user(&String::from_utf8_lossy(&output.stdout), |home| {
        Path::new(home).is_dir()
    })
}

/// Pilih UID terkecil dari keluaran `getent passwd` yang berupa user biasa.
pub(crate) fn parse_regular_user(
    passwd: &str,
    home_exists: impl Fn(&str) -> bool,
) -> Option<String> {
    passwd
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(':').collect();
            let uid: u32 = f.get(2)?.parse().ok()?;
            let (home, shell) = (*f.get(5)?, *f.get(6)?);
            let login_shell = !shell.ends_with("nologin") && !shell.ends_with("/false");
            ((1000..60000).contains(&uid) && login_shell && home_exists(home))
                .then(|| (uid, f[0].to_string()))
        })
        .min()
        .map(|(_, name)| name)
}

pub(crate) fn home_for(user: &str) -> Result<PathBuf> {
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

/// Satu file hasil salinan skel.
pub(crate) struct Copied {
    pub path: PathBuf,
    /// Lokasi cadangan kalau sudah ada file lain di tempat yang sama.
    pub backup: Option<PathBuf>,
}

/// Salin pohon `src` ke `dst`, mengganti placeholder `subs` di isi file.
/// File yang sudah ada dan isinya sama dibiarkan; yang isinya beda
/// dicadangkan dulu ke `<nama>.bak` (`.bak.1`, `.bak.2`, ... kalau sudah
/// ada), supaya hasil edit user tidak hilang saat wizard dijalankan ulang.
pub(crate) fn copy_dir(src: &Path, dst: &Path, subs: &[(&str, &str)]) -> Result<Vec<Copied>> {
    let mut written = Vec::new();
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            written.extend(copy_dir(&entry.path(), &target, subs)?);
        } else if file_type.is_file() {
            let mut data = fs::read(entry.path())?;
            if let Ok(text) = std::str::from_utf8(&data) {
                let mut text = text.to_string();
                for (needle, value) in subs {
                    text = text.replace(needle, value);
                }
                data = text.into_bytes();
            }
            let mut backup = None;
            if target.is_file() {
                if fs::read(&target).ok().as_deref() == Some(data.as_slice()) {
                    continue; // sama persis, tidak perlu disentuh
                }
                let bak = backup_path(&target);
                fs::rename(&target, &bak)
                    .with_context(|| format!("gagal mencadangkan {}", target.display()))?;
                backup = Some(bak);
            }
            fs::write(&target, data)
                .with_context(|| format!("gagal menulis {}", target.display()))?;
            written.push(Copied {
                path: target,
                backup,
            });
        }
    }
    Ok(written)
}

fn backup_path(target: &Path) -> PathBuf {
    let base = format!("{}.bak", target.display());
    let mut candidate = PathBuf::from(&base);
    let mut n = 1;
    while candidate.exists() {
        candidate = PathBuf::from(format!("{base}.{n}"));
        n += 1;
    }
    candidate
}

pub fn timestamp() -> String {
    iso8601_now()
}

fn iso8601_now() -> String {
    // Tanpa crate waktu: `date -Iseconds` selalu ada di Fedora.
    Command::new("date")
        .arg("+%Y-%m-%dT%H:%M:%S%z")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
