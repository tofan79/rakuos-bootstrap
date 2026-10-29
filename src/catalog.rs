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
    /// Nama binary untuk placeholder skel. Default: `id`.
    #[serde(default)]
    pub command: Option<String>,
    /// `false` = disembunyikan dari daftar (belum tersedia di repo).
    #[serde(default = "default_true")]
    pub available: bool,
    /// `true` = pilihan yang disorot saat fase dibuka. Hanya satu per fase.
    #[serde(default)]
    pub default: bool,
    /// `true` = dipakai kode tetapi tidak ditampilkan di layar (mis. greeter
    /// yang dipaksa oleh sebuah desktop).
    #[serde(default)]
    pub hidden: bool,
    /// Nama direktori di `data/skel/` yang disalin ke `$HOME` user.
    #[serde(default)]
    pub skel: Option<String>,
    /// Nama file di `data/sessions/` yang ditulis ke
    /// `/usr/share/wayland-sessions/` (mis. `hyprland-uwsm.desktop`).
    #[serde(default)]
    pub session_file: Option<String>,
    /// Perintah yang dijalankan `dbus-run-session` saat login = `none`.
    #[serde(default)]
    pub autologin_cmd: Option<String>,
    /// Desktop yang selalu memakai login manager ini, apa pun pilihan user
    /// (mis. KineticWE memaksa greeter bawaannya).
    #[serde(default)]
    pub login_override: Option<String>,
    /// Unit systemd display manager yang di-enable (login manager).
    #[serde(default)]
    pub unit: Option<String>,
    /// Paket yang dipasang dengan `--no-recommends` (mis. `sddm-x11` supaya
    /// tidak menarik varian Wayland, atau `sysc-greet` supaya tidak menarik
    /// xterm dari cagebreak).
    #[serde(default)]
    pub no_recommends: Vec<String>,
    /// Tema SDDM yang ditulis ke `/etc/sddm.conf.d/50-rakuos-theme.conf`.
    #[serde(default)]
    pub sddm_theme: Option<String>,
    /// Perintah untuk `[default_session]` di `/etc/greetd/config.toml`.
    /// `@COMPOSITOR@` diganti sesuai desktop yang dipilih.
    #[serde(default)]
    pub greetd_cmd: Option<String>,
    /// User yang menjalankan greeter (default `greeter`).
    #[serde(default)]
    pub greetd_user: Option<String>,
    /// Skrip setup bawaan paket greeter, dijalankan setelah
    /// `/etc/greetd/config.toml` ditulis (mis. patch PAM + state dir).
    #[serde(default)]
    pub greetd_setup: Option<String>,
}

impl Choice {
    /// Nama binary yang dipakai di skel. Sebagian pilihan punya id yang
    /// berbeda dari nama binary (mis. `superfile` -> `spf`).
    pub fn command(&self) -> &str {
        self.command.as_deref().unwrap_or(&self.id)
    }
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

    /// Login manager yang benar-benar dipakai. Desktop boleh memaksa satu
    /// pilihan lewat `login_override` (KineticWE memakai greeter bawaannya).
    pub fn effective_login<'a>(&'a self, desktop: &Choice, picked: &'a Choice) -> &'a Choice {
        desktop
            .login_override
            .as_ref()
            .and_then(|id| self.logins.iter().find(|c| &c.id == id))
            .unwrap_or(picked)
    }

    /// Urutan layar selalu A-Z, tidak bergantung urutan di TOML.
    fn sort_by_name(&mut self) {
        fn sort(list: &mut [Choice]) {
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
        names
            .windows(2)
            .all(|w| w[0].to_lowercase() <= w[1].to_lowercase())
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
    fn command_bisa_beda_dari_id() {
        let cat = Catalog::load(None).unwrap();
        let superfile = cat
            .file_managers
            .iter()
            .find(|c| c.id == "superfile")
            .unwrap();
        assert_eq!(superfile.command(), "spf");

        let helium = cat.browsers.iter().find(|c| c.id == "helium").unwrap();
        assert_eq!(helium.command(), "helium-browser-bin");

        let firefox = cat.browsers.iter().find(|c| c.id == "firefox").unwrap();
        assert_eq!(firefox.command(), "firefox");
    }

    #[test]
    fn dbus_tools_di_base_dan_dbus_daemon_hanya_autologin() {
        let cat = Catalog::load(None).unwrap();
        assert!(cat.base.iter().any(|p| p == "dbus-tools"));
        assert!(!cat.base.iter().any(|p| p == "dbus-daemon"));

        let none = cat.logins.iter().find(|l| l.id == "none").unwrap();
        assert!(none.packages.iter().any(|p| p == "dbus-daemon"));

        for paket in [
            "rakuos-software-qt",
            "rakuos-system-qt",
            "rakuos-welcome-qt",
        ] {
            assert!(
                cat.base.iter().any(|p| p == paket),
                "{paket} harus ada di base"
            );
        }
    }

    #[test]
    fn modul_qt_untuk_aplikasi_rakuos_ada_di_base() {
        let cat = Catalog::load(None).unwrap();
        for paket in ["qt6-qtsvg", "qt6-qtwayland"] {
            assert!(
                cat.base.iter().any(|p| p == paket),
                "{paket} harus ada di base"
            );
        }
    }

    #[test]
    fn login_resmi_memakai_paket_dan_config_rakuos_hyprland() {
        let cat = Catalog::load(None).unwrap();

        let sddm = cat.logins.iter().find(|l| l.id == "sddm").unwrap();
        assert_eq!(sddm.packages, ["sddm-x11", "sddm-themes"]);
        assert_eq!(sddm.no_recommends, ["sddm-x11"]);
        assert_eq!(sddm.sddm_theme.as_deref(), Some("maldives"));

        let noctalia = cat
            .logins
            .iter()
            .find(|l| l.id == "noctalia-greeter")
            .unwrap();
        assert!(noctalia.default);
        assert_eq!(
            noctalia.greetd_cmd.as_deref(),
            Some("/usr/bin/noctalia-greeter-session")
        );

        let sysc = cat.logins.iter().find(|l| l.id == "sysc-greet").unwrap();
        assert!(!sysc.available);
        assert_eq!(
            sysc.greetd_cmd.as_deref(),
            Some("cagebreak -e -c /etc/greetd/cagebreak-greeter-config")
        );
    }

    #[test]
    fn kineticwe_memaksa_greeter_bawaannya() {
        let cat = Catalog::load(None).unwrap();
        let desktop = cat.desktops.iter().find(|d| d.id == "kineticwe").unwrap();
        let sddm = cat.logins.iter().find(|l| l.id == "sddm").unwrap();
        let login = cat.effective_login(desktop, sddm);
        assert_eq!(login.id, "kineticwe-greeter");
        assert!(login.hidden);
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
