//! Writes and removes real keys under HKEY_CURRENT_USER with a unique ProgID and extension.
#![cfg(target_os = "windows")]

use default_rs::{App, FileType, Outcome, Registration};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

#[test]
fn register_lists_the_app_and_unregister_removes_it() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let app = App::new().prog_id("DefaultRsTest.File");
    let kind = FileType::new().extension(".defaultrstest");
    let reg = Registration::new(
        app.clone(),
        "Default RS Test",
        r"C:\\Windows\\System32\\notepad.exe",
    )
    .description("default-rs integration test")
    .file_type(kind.clone());

    default_rs::register(&reg).unwrap();
    let command: String = hkcu
        .open_subkey(r"Software\Classes\DefaultRsTest.File\shell\open\command")
        .unwrap()
        .get_value("")
        .unwrap();
    assert!(
        command.contains("notepad.exe") && command.ends_with("\"%1\""),
        "{command}"
    );
    let open_with = hkcu
        .open_subkey(r"Software\Classes\.defaultrstest\OpenWithProgids")
        .unwrap();
    assert!(open_with.get_raw_value("DefaultRsTest.File").is_ok());
    let registered = hkcu
        .open_subkey(r"Software\RegisteredApplications")
        .unwrap();
    let caps: String = registered.get_value("Default RS Test").unwrap();
    assert_eq!(caps, r"Software\DefaultRSTest\Capabilities");

    // Nothing has chosen it, so it is not the default, and Windows will not let us make it one.
    assert!(!default_rs::is_default(&app, &kind).unwrap());
    assert!(!default_rs::can_set_silently());

    default_rs::unregister(&reg).unwrap();
    assert!(hkcu
        .open_subkey(r"Software\Classes\DefaultRsTest.File")
        .is_err());
    assert!(hkcu.open_subkey(r"Software\DefaultRSTest").is_err());
    let open_with = hkcu.open_subkey(r"Software\Classes\.defaultrstest\OpenWithProgids");
    if let Ok(key) = open_with {
        assert!(key.get_raw_value("DefaultRsTest.File").is_err());
    }
    default_rs::unregister(&reg).unwrap(); // idempotent
    let _ = hkcu.delete_subkey_all(r"Software\Classes\.defaultrstest");
    let _ = Outcome::Set;
}
