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
    /// `true` = setelah memilih desktop ini, wizard menanyakan login manager.
    /// Desktop tanpa flag ini memakai login bawaannya sendiri (KineticWE lewat
    /// `login_override`, sisanya display manager yang ikut terpasang).
    #[serde(default)]
    pub choose_login: bool,
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
    /// `None` = desktop memakai login bawaannya sendiri dan wizard tidak
    /// mengonfigurasi login manager apa pun.
    pub fn effective_login<'a>(
        &'a self,
        desktop: &Choice,
        picked: Option<&'a Choice>,
    ) -> Option<&'a Choice> {
        desktop
            .login_override
            .as_ref()
            .and_then(|id| self.logins.iter().find(|c| &c.id == id))
            .or(picked)
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
