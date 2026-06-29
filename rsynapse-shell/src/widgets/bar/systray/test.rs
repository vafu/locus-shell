use super::{decode_hex, tray_icon_names};

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
