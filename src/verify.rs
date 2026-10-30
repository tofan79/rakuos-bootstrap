//! Pemeriksaan setelah instalasi: pastikan hasilnya benar-benar ada sebelum
//! sistem di-reboot. Hanya membaca; tidak mengubah apa pun.

use std::fs;
use std::path::{Path, PathBuf};

use crate::catalog::Catalog;
use crate::install::{
    command_exists, home_for, package_installed, packages, unit_enabled, wayland_session_exists,
    Picks,
};

pub struct Check {
    pub ok: bool,
    pub label: String,
}

fn check(ok: bool, label: impl Into<String>) -> Check {
    Check {
        ok,
        label: label.into(),
    }
}

pub fn verify(cat: &Catalog, picks: &Picks, user: &str, base_dir: &Path) -> Vec<Check> {
    let mut out = Vec::new();

    // Paket: grup (`@nama`) dilewati karena bukan nama paket rpm.
    if command_exists("rpm") {
        let missing: Vec<String> = packages(cat, picks)
            .into_iter()
            .filter(|p| !p.starts_with('@') && !package_installed(p))
            .collect();
        out.push(if missing.is_empty() {
            check(true, "semua paket terpasang")
        } else {
            check(
                false,
                format!("paket belum terpasang: {}", missing.join(", ")),
            )
        });
    }

    // Display manager.
    match picks.login.and_then(|l| l.unit.as_deref()) {
        Some(unit) => out.push(check(unit_enabled(unit), format!("{unit} aktif saat boot"))),
        None if picks.login.is_none() => {
            let dm = ["lightdm", "sddm", "gdm", "greetd"]
                .iter()
                .map(|n| format!("{n}.service"))
                .find(|u| unit_enabled(u));
            out.push(match dm {
                Some(u) => check(true, format!("{u} aktif saat boot (bawaan desktop)")),
                None => check(false, "tidak ada display manager yang aktif"),
            });
        }
        None => {}
    }

    // Sesi Wayland (hanya desktop yang menawarkan pilihan login).
    let d = picks.desktop;
    if d.choose_login {
        let by_file = d
            .session_file
            .as_deref()
            .is_some_and(|f| Path::new("/usr/share/wayland-sessions").join(f).is_file());
        let by_cmd = d
            .autologin_cmd
            .as_deref()
            .is_some_and(wayland_session_exists);
        out.push(check(
            by_file || by_cmd,
            format!("sesi Wayland {} terdaftar", d.name),
        ));
    }

    // Skel: setiap file di skel harus ada di home user.
    if let Some(name) = &d.skel {
        let src = base_dir.join("skel").join(name);
        match home_for(user) {
            Ok(home) => {
                let missing: Vec<PathBuf> = skel_files(&src, Path::new(""))
                    .into_iter()
                    .filter(|rel| !home.join(rel).is_file())
                    .collect();
                out.push(if missing.is_empty() {
                    check(true, format!("config {} ada di {}", d.name, home.display()))
                } else {
                    check(
                        false,
                        format!(
                            "config {} belum tersalin: {}",
                            d.name,
                            missing
                                .iter()
                                .map(|p| p.display().to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    )
                });
            }
            Err(e) => out.push(check(false, format!("home {user} tidak ditemukan: {e}"))),
        }
    }
    out
}

/// Path relatif semua file di dalam skel.
pub(crate) fn skel_files(root: &Path, rel: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join(rel)) {
        for e in entries.flatten() {
            let r = rel.join(e.file_name());
            match e.file_type() {
                Ok(t) if t.is_dir() => files.extend(skel_files(root, &r)),
                Ok(t) if t.is_file() => files.push(r),
                _ => {}
            }
        }
    }
    files
}
