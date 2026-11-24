//! Windows: ProgID registration under HKCU, and the user's choice in Settings > Default apps.
//!
//! Windows 10 and 11 protect the default with a per-user hash, so an app cannot set it. This
//! module registers the app (so it is offered) and opens the Settings page for the user to choose.

use std::io::ErrorKind;
use std::os::windows::process::CommandExt;
use std::process::Command;

use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

use crate::error::{Error, Result};
use crate::pure::{same_handler, windows_register_plan, windows_unregister_plan};
use crate::types::{App, FileType, Host, Outcome, Registration};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[link(name = "shell32")]
extern "system" {
    fn SHChangeNotify(
        event: i32,
        flags: u32,
        item1: *const std::ffi::c_void,
        item2: *const std::ffi::c_void,
    );
}

/// Tells Explorer that file associations changed so it refreshes "Open with" and icons.
fn notify_associations_changed() {
    const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
    const SHCNF_IDLIST: u32 = 0;
    // SAFETY: documented to accept null items for SHCNE_ASSOCCHANGED.
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        )
    }
}

pub(crate) fn current_default(file_type: &FileType) -> Result<Option<String>> {
    let ext = file_type
        .extension_str()
        .ok_or(Error::MissingFileType("extension"))?;
    let path =
        format!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{ext}\UserChoice");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey(path) {
        Ok(key) => match key.get_value::<String, _>("ProgId") {
            Ok(id) => Ok(Some(id)),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        },
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub(crate) fn set_default(app: &App, file_type: &FileType) -> Result<Outcome> {
    let prog = app
        .id_for(Host::Windows)
        .ok_or(Error::MissingIdentifier("prog_id"))?;
    if current_default(file_type)?.is_some_and(|now| same_handler(Host::Windows, &now, prog)) {
        return Ok(Outcome::Set);
    }
    Command::new("explorer")
        .arg("ms-settings:defaultapps")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|err| Error::Failed(format!("could not open Windows Settings: {err}")))?;
    Ok(Outcome::NeedsUserAction)
}

pub(crate) fn register(reg: &Registration) -> Result<()> {
    let plan = windows_register_plan(reg)?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for write in plan {
        let (key, _) = hkcu.create_subkey(&write.subkey)?;
        key.set_value(&write.name, &write.data)?;
    }
    notify_associations_changed();
    Ok(())
}

pub(crate) fn unregister(reg: &Registration) -> Result<()> {
    let removal = windows_unregister_plan(reg)?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for (subkey, name) in &removal.values {
        match hkcu.open_subkey_with_flags(subkey, KEY_SET_VALUE) {
            Ok(key) => match key.delete_value(name) {
                Ok(()) => {}
                Err(err) if err.kind() == ErrorKind::NotFound => {}
                Err(err) => return Err(err.into()),
            },
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
    }
    for tree in &removal.trees {
        match hkcu.delete_subkey_all(tree) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
    }
    notify_associations_changed();
    Ok(())
}
