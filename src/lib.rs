//! Query and set the default app for a file type on macOS, Windows and Linux, and register an
//! app as a handler without an installer.
//!
//! Each OS names apps and file types differently, so [`App`] and [`FileType`] carry all the
//! names and each OS reads the ones it needs:
//!
//! | | App identifier | File type | Setting the default |
//! |---|---|---|---|
//! | macOS | `bundle_id` | `uti` | silent (LaunchServices); the app must be installed |
//! | Linux | `desktop_id` | `mime` | silent (`xdg-mime`); needs the `.desktop` file |
//! | Windows | `prog_id` | `extension` | **not possible**: opens Settings > Default apps |
//!
//! ```no_run
//! use default_rs::{App, FileType, Outcome};
//!
//! let app = App::new()
//!     .bundle_id("com.example.editor")
//!     .desktop_id("editor.desktop")
//!     .prog_id("Example.Editor.File");
//! match default_rs::set_default(&app, &FileType::markdown())? {
//!     Outcome::Set => println!("done"),
//!     Outcome::NeedsUserAction => println!("choose the app in the window that opened"),
//! }
//! # Ok::<(), default_rs::Error>(())
//! ```

mod error;
mod pure;
mod types;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use linux as sys;
#[cfg(target_os = "macos")]
use macos as sys;
#[cfg(target_os = "windows")]
use windows as sys;

pub use error::{Error, Result};
pub use types::{App, FileType, Host, Outcome, Registration};

/// Whether [`set_default`] can finish without the user. True on macOS and Linux; on Windows
/// the OS reserves the choice for the user, so [`set_default`] returns [`Outcome::NeedsUserAction`].
pub const fn can_set_silently() -> bool {
    !matches!(Host::current(), Host::Windows)
}

/// The identifier of the app that opens `file_type` now (bundle id, desktop id or ProgID), or
/// `None` when nothing is set.
pub fn current_default(file_type: &FileType) -> Result<Option<String>> {
    sys::current_default(file_type)
}

/// Whether `app` is the current default for `file_type`.
pub fn is_default(app: &App, file_type: &FileType) -> Result<bool> {
    let host = Host::current();
    let wanted = app
        .id_for(host)
        .ok_or(Error::MissingIdentifier(match host {
            Host::Mac => "bundle_id",
            Host::Windows => "prog_id",
            Host::Linux => "desktop_id",
        }))?;
    Ok(current_default(file_type)?.is_some_and(|now| pure::same_handler(host, &now, wanted)))
}

/// Makes `app` the default for `file_type`. See [`Outcome`] for the Windows case.
pub fn set_default(app: &App, file_type: &FileType) -> Result<Outcome> {
    sys::set_default(app, file_type)
}

/// Declares the app to the OS: a `.desktop` file under `~/.local/share/applications` on Linux, and
/// ProgID, "Open with" and Capabilities keys under `HKEY_CURRENT_USER` on Windows. Calling it again
/// updates the registration. Not available on macOS, where the bundle's `Info.plist` does this.
pub fn register(registration: &Registration) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = registration;
        Err(Error::Unsupported(
            "macOS apps declare their file types in the bundle's Info.plist (CFBundleDocumentTypes)",
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        sys::register(registration)
    }
}

/// Removes what [`register`] added. Succeeds when there is nothing to remove.
pub fn unregister(registration: &Registration) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = registration;
        Err(Error::Unsupported("macOS has nothing to unregister"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        sys::unregister(registration)
    }
}
