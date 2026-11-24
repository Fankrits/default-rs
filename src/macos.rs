//! macOS: LaunchServices content-type handlers.

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};

use crate::error::{Error, Result};
use crate::pure::same_handler;
use crate::types::{App, FileType, Host, Outcome};

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn LSSetDefaultRoleHandlerForContentType(
        content_type: CFStringRef,
        role: u32,
        handler_bundle_id: CFStringRef,
    ) -> i32;
    fn LSCopyDefaultRoleHandlerForContentType(content_type: CFStringRef, role: u32) -> CFStringRef;
}

// LaunchServices role masks: kLSRolesEditor = 0x4, kLSRolesAll = 0xFFFFFFFF.
const ROLE_ALL: u32 = 0xFFFF_FFFF;
const ROLE_EDITOR: u32 = 0x0000_0004;

fn uti_of(file_type: &FileType) -> Result<&str> {
    file_type.uti_str().ok_or(Error::MissingFileType("uti"))
}

pub(crate) fn current_default(file_type: &FileType) -> Result<Option<String>> {
    let uti = CFString::new(uti_of(file_type)?);
    for role in [ROLE_ALL, ROLE_EDITOR] {
        // SAFETY: `uti` is a live CFString; the call follows the Copy rule, so a non-null
        // result is owned by us and wrapped (and released) by `wrap_under_create_rule`.
        let handler =
            unsafe { LSCopyDefaultRoleHandlerForContentType(uti.as_concrete_TypeRef(), role) };
        if !handler.is_null() {
            // SAFETY: `handler` is non-null and owned (Copy rule).
            return Ok(Some(
                unsafe { CFString::wrap_under_create_rule(handler) }.to_string(),
            ));
        }
    }
    Ok(None)
}

pub(crate) fn set_default(app: &App, file_type: &FileType) -> Result<Outcome> {
    let bundle_id = app
        .id_for(Host::Mac)
        .ok_or(Error::MissingIdentifier("bundle_id"))?;
    let uti_text = uti_of(file_type)?;
    let handler = CFString::new(bundle_id);
    let uti = CFString::new(uti_text);
    // SAFETY: both strings are live CFStrings for the duration of the calls.
    let _ = unsafe {
        LSSetDefaultRoleHandlerForContentType(
            uti.as_concrete_TypeRef(),
            ROLE_EDITOR,
            handler.as_concrete_TypeRef(),
        )
    };
    // SAFETY: as above.
    let status = unsafe {
        LSSetDefaultRoleHandlerForContentType(
            uti.as_concrete_TypeRef(),
            ROLE_ALL,
            handler.as_concrete_TypeRef(),
        )
    };
    if status != 0 {
        return Err(Error::Failed(format!(
            "LaunchServices could not set {bundle_id} as the default for {uti_text} (status {status})"
        )));
    }
    match current_default(file_type)? {
        Some(now) if same_handler(Host::Mac, &now, bundle_id) => Ok(Outcome::Set),
        _ => Err(Error::NotInstalled(format!(
            "macOS did not accept {bundle_id} for {uti_text}. LaunchServices only knows apps it has \
             seen, so install the app in /Applications and open it once."
        ))),
    }
}
