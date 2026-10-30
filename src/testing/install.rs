use crate::install::{parse_regular_user, sddm_config_body, sddm_sessions_body};

#[test]
fn config_sddm_sesuai_rakuos_hyprland() {
    let body = sddm_config_body("maldives", Some("Bibata-Modern-Ice"));
    assert_eq!(
        body,
        "[Theme]\nCurrent=maldives\nCursorTheme=Bibata-Modern-Ice\n\n[General]\nInputMethod=\n"
    );

    let tanpa_cursor = sddm_config_body("maldives", None);
    assert_eq!(
        tanpa_cursor,
        "[Theme]\nCurrent=maldives\n\n[General]\nInputMethod=\n"
    );
}

#[test]
fn config_sddm_mendaftarkan_sesi_wayland() {
    let body = sddm_sessions_body();
    // `DisplayServer` tidak boleh ditulis: varian greeter (`sddm-x11` /
    // `sddm-wayland-miriway`) yang menentukan lewat drop-in di /usr/lib.
    assert!(!body.contains("DisplayServer"));
    assert!(body.contains(
        "[Wayland]\nSessionDir=/usr/local/share/wayland-sessions,/usr/share/wayland-sessions"
    ));
    assert!(body.contains("[X11]\nSessionDir=/usr/local/share/xsessions,/usr/share/xsessions"));
}

const PASSWD: &str = "root:x:0:0:root:/root:/bin/bash\n\
daemon:x:2:2::/sbin:/usr/sbin/nologin\n\
greeter:x:978:978::/var/lib/greeter:/usr/sbin/nologin\n\
andi:x:1001:1001::/home/andi:/bin/bash\n\
budi:x:1000:1000::/home/budi:/bin/zsh\n\
nobody:x:65534:65534::/:/sbin/nologin\n";

#[test]
fn user_target_tanpa_sudo_user_adalah_user_biasa_bukan_root() {
    assert_eq!(
        parse_regular_user(PASSWD, |_| true).as_deref(),
        Some("budi")
    );
    // home tidak ada -> dilewati
    let hanya_andi = parse_regular_user(PASSWD, |h| h == "/home/andi");
    assert_eq!(hanya_andi.as_deref(), Some("andi"));
    assert_eq!(
        parse_regular_user("root:x:0:0::/root:/bin/bash\n", |_| true),
        None
    );
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rakuos-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn skel_menyalin_mengganti_placeholder_dan_mencadangkan_config_lama() {
    use crate::install::copy_dir;
    let src = tmp("src");
    let dst = tmp("dst");
    std::fs::create_dir_all(src.join(".config/app")).unwrap();
    std::fs::write(src.join(".config/app/rc"), "term=@TERMINAL@\n").unwrap();
    let subs = [("@TERMINAL@", "kitty")];

    // Pertama: file baru, tanpa cadangan.
    let r = copy_dir(&src, &dst, &subs).unwrap();
    assert_eq!(r.len(), 1);
    assert!(r[0].backup.is_none());
    let rc = dst.join(".config/app/rc");
    assert_eq!(std::fs::read_to_string(&rc).unwrap(), "term=kitty\n");

    // Kedua: isi sama -> tidak disentuh, tidak ada cadangan.
    assert!(copy_dir(&src, &dst, &subs).unwrap().is_empty());
    assert!(!dst.join(".config/app/rc.bak").exists());

    // Ketiga: user sudah mengedit -> dicadangkan, bukan hilang.
    std::fs::write(&rc, "term=edit-saya\n").unwrap();
    let r = copy_dir(&src, &dst, &subs).unwrap();
    assert_eq!(r.len(), 1);
    let bak = r[0].backup.clone().unwrap();
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), "term=edit-saya\n");
    assert_eq!(std::fs::read_to_string(&rc).unwrap(), "term=kitty\n");

    // Keempat: edit lagi -> cadangan pertama tidak ditimpa.
    std::fs::write(&rc, "term=edit-lagi\n").unwrap();
    let r = copy_dir(&src, &dst, &subs).unwrap();
    assert_ne!(r[0].backup.clone().unwrap(), bak);
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), "term=edit-saya\n");

    let _ = std::fs::remove_dir_all(&src);
    let _ = std::fs::remove_dir_all(&dst);
}

#[test]
fn log_membuang_warna_dan_progress_bar() {
    use crate::log::strip_ansi;
    assert_eq!(strip_ansi("\x1b[1;32mOK\x1b[0m selesai"), "OK selesai");
}

#[test]
fn verifikasi_mendaftar_semua_file_skel() {
    use crate::verify::skel_files;
    let mut f = skel_files(
        std::path::Path::new("data/skel/sway"),
        std::path::Path::new(""),
    );
    f.sort();
    assert_eq!(f, [std::path::PathBuf::from(".config/sway/config")]);
}
