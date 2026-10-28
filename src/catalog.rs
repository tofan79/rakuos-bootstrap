use anyhow::{Context, Result};
use serde::Deserialize;

fn default_true() -> bool {
    true
}

/// Satu pilihan di katalog. Semua kategori memakai bentuk yang sama supaya
/// menambah pilihan cukup edit `catalog.toml`, tanpa menyentuh kode.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Choice {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Paket RPM yang dipasang lewat `rum`.
    #[serde(default)]
    pub packages: Vec<String>,
    /// `false` = disembunyikan dari daftar (belum tersedia di repo).
    #[serde(default = "default_true")]
    pub available: bool,
    /// `true` = pilihan yang disorot saat fase dibuka. Hanya satu per fase.
    #[serde(default)]
    pub default: bool,
    /// Nama direktori di `data/skel/` yang disalin ke `$HOME` user.
    #[serde(default)]
    pub skel: Option<String>,
    /// Perintah yang dijalankan `dbus-run-session` saat login = `none`.
    #[serde(default)]
    pub autologin_cmd: Option<String>,
    /// Unit systemd display manager yang di-enable (login manager).
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Catalog {
    /// Paket yang selalu dipasang, apa pun pilihannya (audio, GPU, portal...).
    #[serde(default)]
    pub base: Vec<String>,
    /// Repo pihak ketiga yang diaktifkan sebelum instalasi. Kunci yang
    /// dikenal: `terra`, `rpmfusion` (lihat `install::enable_repos`).
    #[serde(default)]
    pub repos: Vec<String>,
    #[serde(default)]
    pub terminals: Vec<Choice>,
    #[serde(default)]
    pub browsers: Vec<Choice>,
    #[serde(default)]
    pub desktops: Vec<Choice>,
    #[serde(default)]
    pub file_managers: Vec<Choice>,
    #[serde(default)]
    pub logins: Vec<Choice>,
}

impl Catalog {
    /// Tanpa `path`: pakai katalog yang ditanam di binary (fallback saat
    /// `/usr/share/rakuos/bootstrap/catalog.toml` belum ada).
    pub fn load(path: Option<&std::path::Path>) -> Result<Self> {
        let text = match path {
            Some(p) => std::fs::read_to_string(p)
                .with_context(|| format!("tidak bisa membaca katalog {}", p.display()))?,
            None => include_str!("../data/catalog.toml").to_string(),
        };
        let mut cat: Catalog = toml::from_str(&text).context("katalog TOML tidak valid")?;
        cat.sort_by_name();
        Ok(cat)
    }

    /// Urutan layar selalu A-Z, tidak bergantung urutan di TOML.
    fn sort_by_name(&mut self) {
        fn sort(list: &mut Vec<Choice>) {
            list.sort_by_key(|c| c.name.to_lowercase());
        }
        sort(&mut self.terminals);
        sort(&mut self.browsers);
        sort(&mut self.desktops);
        sort(&mut self.file_managers);
        sort(&mut self.logins);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(names: &[&str]) -> bool {
        names.windows(2).all(|w| w[0].to_lowercase() <= w[1].to_lowercase())
    }

    fn names(list: &[Choice]) -> Vec<&str> {
        list.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn katalog_selalu_abjad() {
        let cat = Catalog::load(None).unwrap();
        for list in [
            &cat.terminals,
            &cat.browsers,
            &cat.desktops,
            &cat.file_managers,
            &cat.logins,
        ] {
            let n = names(list);
            assert!(sorted(&n), "tidak abjad: {n:?}");
        }
    }

    #[test]
    fn ada_satu_default_per_fase() {
        let cat = Catalog::load(None).unwrap();
        for list in [
            &cat.terminals,
            &cat.browsers,
            &cat.desktops,
            &cat.file_managers,
            &cat.logins,
        ] {
            let n = list.iter().filter(|c| c.default).count();
            assert_eq!(n, 1, "default harus tepat satu: {n} di {:?}", names(list));
        }
    }
}
