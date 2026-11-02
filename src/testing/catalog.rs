use crate::catalog::{Catalog, Choice};

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
    assert_eq!(sddm.packages, ["sddm-wayland-miriway"]);
    assert!(sddm.no_recommends.is_empty());
    assert_eq!(sddm.sddm_theme, None);

    let noctalia = cat
        .logins
        .iter()
        .find(|l| l.id == "noctalia-greeter")
        .unwrap();
    assert_eq!(
        noctalia.greetd_cmd.as_deref(),
        Some("/usr/bin/noctalia-greeter-session")
    );

    let sysc = cat.logins.iter().find(|l| l.id == "sysc-greet").unwrap();
    assert!(sysc.available);
    assert_eq!(
        sysc.greetd_cmd.as_deref(),
        Some("cagebreak -e -c /etc/greetd/cagebreak-greeter-config")
    );
}

#[test]
fn login_tampil_empat_pilihan_tanpa_gdm_dan_plasma() {
    let cat = Catalog::load(None).unwrap();
    let visible = cat.logins.iter().filter(|l| !l.hidden).count();
    assert_eq!(visible, 4);
    assert!(!cat
        .logins
        .iter()
        .any(|l| l.id == "gdm" || l.id == "plasma-login"));
}

#[test]
fn login_hanya_ditanya_untuk_desktop_yang_memilih_login() {
    let cat = Catalog::load(None).unwrap();
    let mut ids: Vec<&str> = cat
        .desktops
        .iter()
        .filter(|d| d.choose_login)
        .map(|d| d.id.as_str())
        .collect();
    ids.sort();
    assert_eq!(ids, ["budgie", "hyprland", "mangowm", "sway"]);
}

#[test]
fn kineticwe_memaksa_greeter_bawaannya() {
    let cat = Catalog::load(None).unwrap();
    let desktop = cat.desktops.iter().find(|d| d.id == "kineticwe").unwrap();
    let sddm = cat.logins.iter().find(|l| l.id == "sddm").unwrap();
    let login = cat.effective_login(desktop, Some(sddm)).unwrap();
    assert_eq!(login.id, "kineticwe-greeter");
    assert!(login.hidden);
}

#[test]
fn umbriel_memaksa_greeter_noctalia() {
    let cat = Catalog::load(None).unwrap();
    let desktop = cat.desktops.iter().find(|d| d.id == "umbriel").unwrap();
    let noctalia = cat
        .logins
        .iter()
        .find(|l| l.id == "noctalia-greeter")
        .unwrap();
    let login = cat.effective_login(desktop, Some(noctalia)).unwrap();
    assert_eq!(login.id, "noctalia-greeter");
}

#[test]
fn ryoku_mengambil_alih_instalasi_dan_memakai_sddm_sendiri() {
    let cat = Catalog::load(None).unwrap();
    let ryoku = cat.desktops.iter().find(|d| d.id == "ryoku").unwrap();
    assert!(!ryoku.choose_login);
    assert_eq!(ryoku.login_override.as_deref(), Some("ryoku-sddm"));
    for package in [
        "ryoku",
        "ryoku-keyring",
        "ryoku-desktop",
        "ryoku-desktop-hyprland",
        "ryoku-desktop-niri",
        "kitty",
        "nautilus",
        "firefox",
        "fish",
    ] {
        assert!(ryoku.packages.iter().any(|p| p == package), "{package}");
    }
    let sddm = cat.logins.iter().find(|l| l.id == "ryoku-sddm").unwrap();
    assert!(sddm.hidden);
    assert_eq!(sddm.unit.as_deref(), Some("sddm.service"));
    assert_eq!(sddm.sddm_theme.as_deref(), Some("ryoku"));
}

#[test]
fn desktop_bawaan_tidak_punya_login() {
    let cat = Catalog::load(None).unwrap();
    for id in ["lxqt"] {
        let d = cat.desktops.iter().find(|d| d.id == id).unwrap();
        assert!(!d.choose_login);
        assert!(cat.effective_login(d, None).is_none());
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

#[test]
fn skel_yang_disebut_desktop_harus_ada_dan_tidak_kosong() {
    use crate::verify::skel_files;
    let cat = Catalog::load(None).unwrap();
    for d in cat.desktops.iter().filter(|d| d.skel.is_some()) {
        let name = d.skel.as_deref().unwrap();
        let dir = std::path::Path::new("data/skel").join(name);
        assert!(dir.is_dir(), "skel {name} (desktop {}) tidak ada", d.id);
        let n = skel_files(&dir, std::path::Path::new("")).len();
        assert!(n > 0, "skel {name} (desktop {}) kosong", d.id);
    }
}

#[test]
fn skel_umbriel_valid_toml_dan_pakai_terminal_pilihan() {
    let raw = std::fs::read_to_string("data/skel/umbriel/.config/umbriel/config.toml").unwrap();
    let value: toml::Value = toml::from_str(&raw).unwrap();
    let autostart = value["general"]["autostart"].as_array().unwrap();
    assert!(autostart.iter().any(|v| v.as_str() == Some("noctalia")));
    assert!(autostart.iter().any(|v| v.as_str() == Some("@TERMINAL@")));
    assert_eq!(value["general"]["mod_key"].as_str().unwrap(), "Super");
}
