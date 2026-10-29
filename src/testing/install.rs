use crate::install::sddm_config_body;

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
