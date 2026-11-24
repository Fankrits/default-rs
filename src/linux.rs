//! freedesktop systems: `xdg-mime` for the default, a `.desktop` file for registration.

use std::path::PathBuf;
use std::process::{Command, Output};

use crate::error::{Error, Result};
use crate::pure::{
    desktop_file_dirs, parse_xdg_default, render_desktop_entry, user_applications_dir,
};
use crate::types::{App, FileType, Host, Outcome, Registration};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

fn run(program: &'static str, args: &[&str]) -> Result<Output> {
    Command::new(program).args(args).output().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            Error::ToolMissing(program)
        } else {
            Error::Io(err)
        }
    })
}

fn mime_of(file_type: &FileType) -> Result<&str> {
    file_type.mime_str().ok_or(Error::MissingFileType("mime"))
}

fn desktop_file_installed(desktop_id: &str) -> bool {
    desktop_file_dirs(
        env("XDG_DATA_HOME").as_deref(),
        env("XDG_DATA_DIRS").as_deref(),
        env("HOME").as_deref(),
    )
    .iter()
    .any(|dir| dir.join(desktop_id).is_file())
}

fn user_desktop_file(desktop_id: &str) -> Result<PathBuf> {
    user_applications_dir(env("XDG_DATA_HOME").as_deref(), env("HOME").as_deref())
        .map(|dir| dir.join(desktop_id))
        .ok_or_else(|| Error::Failed("neither XDG_DATA_HOME nor HOME is set".into()))
}

fn refresh_desktop_database(file: &std::path::Path) {
    if let Some(dir) = file.parent() {
        // Best effort: menus and `xdg-mime` also work without it, just later.
        let _ = run("update-desktop-database", &["-q", &dir.to_string_lossy()]);
    }
}

pub(crate) fn current_default(file_type: &FileType) -> Result<Option<String>> {
    let out = run("xdg-mime", &["query", "default", mime_of(file_type)?])?;
    Ok(parse_xdg_default(&String::from_utf8_lossy(&out.stdout)))
}

pub(crate) fn set_default(app: &App, file_type: &FileType) -> Result<Outcome> {
    let id = app
        .id_for(Host::Linux)
        .ok_or(Error::MissingIdentifier("desktop_id"))?;
    let mime = mime_of(file_type)?;
    if !desktop_file_installed(id) {
        return Err(Error::NotInstalled(format!(
            "no {id} in the XDG applications directories. Install the app, or call register() first."
        )));
    }
    let out = run("xdg-mime", &["default", id, mime])?;
    if !out.status.success() {
        return Err(Error::Failed(format!(
            "xdg-mime failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    match current_default(file_type)? {
        Some(now) if now == id => Ok(Outcome::Set),
        other => Err(Error::Failed(format!(
            "xdg-mime accepted the change but the default for {mime} is {}",
            other.as_deref().unwrap_or("unset")
        ))),
    }
}

pub(crate) fn register(reg: &Registration) -> Result<()> {
    let id = reg
        .app
        .id_for(Host::Linux)
        .ok_or(Error::MissingIdentifier("desktop_id"))?;
    let text = render_desktop_entry(reg)?;
    let file = user_desktop_file(id)?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&file, text)?;
    refresh_desktop_database(&file);
    Ok(())
}

pub(crate) fn unregister(reg: &Registration) -> Result<()> {
    let id = reg
        .app
        .id_for(Host::Linux)
        .ok_or(Error::MissingIdentifier("desktop_id"))?;
    let file = user_desktop_file(id)?;
    match std::fs::remove_file(&file) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }
    refresh_desktop_database(&file);
    Ok(())
}
