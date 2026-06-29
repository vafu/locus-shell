use super::{decode_hex, source::clean_menu_label, tray_icon_names};

#[test]
fn network_manager_wired_icon_has_standard_fallbacks() {
    let names = tray_icon_names("nm-device-wired");

    assert_eq!(names[0], "nm-device-wired");
    assert!(names.contains(&"network-wired-symbolic".to_owned()));
    assert!(names.contains(&"network-wired".to_owned()));
    assert!(names.contains(&"application-x-executable-symbolic".to_owned()));
}

#[test]
fn wireless_strength_icon_has_symbolic_fallback() {
    let names = tray_icon_names("nm-signal-75");

    assert!(names.contains(&"network-wireless-signal-good-symbolic".to_owned()));
}

#[test]
fn hex_decode_rejects_bad_pixmap_data() {
    assert_eq!(decode_hex("000fFf"), Some(vec![0, 15, 255]));
    assert_eq!(decode_hex("123"), None);
    assert_eq!(decode_hex("zz"), None);
}

#[test]
fn dbusmenu_mnemonics_are_removed() {
    assert_eq!(clean_menu_label("_Report Issue..."), "Report Issue...");
    assert_eq!(
        clean_menu_label("Show _Logs in File Manager"),
        "Show Logs in File Manager"
    );
    assert_eq!(clean_menu_label("Reset _App Data..."), "Reset App Data...");
    assert_eq!(
        clean_menu_label("Literal __underscore"),
        "Literal _underscore"
    );
}
