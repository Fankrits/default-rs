//! Runs against the real `xdg-mime` with a private HOME, so the user's settings are untouched.
#![cfg(target_os = "linux")]

use default_rs::{App, Error, FileType, Outcome, Registration};

fn have_xdg_mime() -> bool {
    std::process::Command::new("xdg-mime")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
fn register_set_query_and_unregister_round_trip() {
    if !have_xdg_mime() {
        eprintln!("skipped: xdg-mime is not installed");
        return;
    }
    let root = std::env::temp_dir().join(format!("default-rs-linux-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::env::set_var("HOME", &root);
    std::env::set_var("XDG_DATA_HOME", root.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
    std::env::remove_var("XDG_CURRENT_DESKTOP");

    let mime = "application/x-default-rs-test";
    let kind = FileType::new().mime(mime);
    let ours = App::new().desktop_id("default-rs-test.desktop");
    let other = App::new().desktop_id("default-rs-other.desktop");
    let reg_ours =
        Registration::new(ours.clone(), "Default RS Test", "/usr/bin/true").file_type(kind.clone());
    let reg_other = Registration::new(other.clone(), "Default RS Other", "/usr/bin/false")
        .file_type(kind.clone());

    // Not installed yet: refuses instead of writing a dangling default.
    assert!(matches!(
        default_rs::set_default(&ours, &kind),
        Err(Error::NotInstalled(_))
    ));

    default_rs::register(&reg_ours).unwrap();
    default_rs::register(&reg_other).unwrap();
    let desktop = root.join("data/applications/default-rs-test.desktop");
    let text = std::fs::read_to_string(&desktop).unwrap();
    assert!(
        text.contains("MimeType=application/x-default-rs-test;"),
        "{text}"
    );
    assert!(text.contains("Exec=/usr/bin/true %F"), "{text}");

    // Hand the type to the other app, then take it back.
    assert_eq!(
        default_rs::set_default(&other, &kind).unwrap(),
        Outcome::Set
    );
    assert!(default_rs::is_default(&other, &kind).unwrap());
    assert!(!default_rs::is_default(&ours, &kind).unwrap());

    assert_eq!(default_rs::set_default(&ours, &kind).unwrap(), Outcome::Set);
    assert!(default_rs::is_default(&ours, &kind).unwrap());
    assert!(!default_rs::is_default(&other, &kind).unwrap());
    assert_eq!(
        default_rs::current_default(&kind).unwrap().as_deref(),
        Some("default-rs-test.desktop")
    );

    default_rs::unregister(&reg_ours).unwrap();
    default_rs::unregister(&reg_other).unwrap();
    assert!(!desktop.exists());
    default_rs::unregister(&reg_ours).unwrap(); // idempotent

    // A file type without a MIME type is a clear error, not a panic.
    assert!(matches!(
        default_rs::current_default(&FileType::new().extension("x")),
        Err(Error::MissingFileType("mime"))
    ));
    let _ = std::fs::remove_dir_all(&root);
}
