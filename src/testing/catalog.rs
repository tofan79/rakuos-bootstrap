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
fn login_hanya_ditanya_untuk_empat_compositor() {
    let cat = Catalog::load(None).unwrap();
    let mut ids: Vec<&str> = cat
        .desktops
        .iter()
        .filter(|d| d.choose_login)
        .map(|d| d.id.as_str())
        .collect();
    ids.sort();
    assert_eq!(ids, ["hyprland", "mangowm", "niri", "sway"]);
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
fn desktop_bawaan_tidak_punya_login() {
    let cat = Catalog::load(None).unwrap();
    for id in ["budgie", "lxqt", "umbriel"] {
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
