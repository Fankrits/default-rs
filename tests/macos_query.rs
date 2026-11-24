//! Read-only: asks LaunchServices who opens a type, and never changes a user's defaults.
#![cfg(target_os = "macos")]

use default_rs::{App, Error, FileType};

#[test]
fn the_default_for_json_can_be_read_and_a_made_up_app_is_not_it() {
    let current = default_rs::current_default(&FileType::json()).unwrap();
    if let Some(id) = &current {
        assert!(
            id.contains('.'),
            "a bundle id looks like com.vendor.app, got {id}"
        );
    }
    let nobody = App::new().bundle_id("com.invalid.default-rs-nobody");
    assert!(!default_rs::is_default(&nobody, &FileType::json()).unwrap());
    assert!(default_rs::can_set_silently());
}

#[test]
fn registering_is_unsupported_and_missing_names_are_clear_errors() {
    let app = App::new().bundle_id("com.invalid.default-rs-nobody");
    let reg =
        default_rs::Registration::new(app.clone(), "X", "/bin/true").file_type(FileType::json());
    assert!(matches!(
        default_rs::register(&reg),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        default_rs::current_default(&FileType::new().mime("text/plain")),
        Err(Error::MissingFileType("uti"))
    ));
    assert!(matches!(
        default_rs::is_default(&App::new(), &FileType::json()),
        Err(Error::MissingIdentifier("bundle_id"))
    ));
}
