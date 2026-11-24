//! The values you pass in: which app, which file type, and how an app registers itself.

use std::path::{Path, PathBuf};

use crate::pure::normalize_extension;

/// Which operating system a decision is made for. Passed explicitly so every
/// platform's logic can be tested on any host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    /// macOS (LaunchServices).
    Mac,
    /// Windows (registry and the Default apps settings page).
    Windows,
    /// Linux and other freedesktop systems (`xdg-mime`).
    Linux,
}

impl Host {
    /// The OS this build runs on. Anything that is not macOS or Windows is treated as freedesktop.
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Host::Mac
        } else if cfg!(target_os = "windows") {
            Host::Windows
        } else {
            Host::Linux
        }
    }
}

/// What [`crate::set_default`] achieved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The app is now the default, and the OS confirmed it.
    Set,
    /// The OS reserves the choice for the user (Windows). Settings was opened on the Default
    /// apps page; check [`crate::is_default`] afterwards.
    NeedsUserAction,
}

/// An app, named by the identifier each OS uses for it. Fill in the ones for the
/// systems you ship on; the others may stay empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct App {
    bundle_id: Option<String>,
    desktop_id: Option<String>,
    prog_id: Option<String>,
}

impl App {
    /// An app with no identifiers yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The macOS bundle identifier, such as `com.example.editor`.
    pub fn bundle_id(mut self, id: impl Into<String>) -> Self {
        self.bundle_id = Some(id.into());
        self
    }

    /// The freedesktop desktop-file id, such as `editor.desktop`.
    pub fn desktop_id(mut self, id: impl Into<String>) -> Self {
        self.desktop_id = Some(id.into());
        self
    }

    /// The Windows ProgID the app registers, such as `Example.Editor.File`.
    pub fn prog_id(mut self, id: impl Into<String>) -> Self {
        self.prog_id = Some(id.into());
        self
    }

    /// The identifier this app uses on `host`, if one was given.
    pub fn id_for(&self, host: Host) -> Option<&str> {
        match host {
            Host::Mac => self.bundle_id.as_deref(),
            Host::Windows => self.prog_id.as_deref(),
            Host::Linux => self.desktop_id.as_deref(),
        }
    }
}

/// A kind of file, named the way each OS names it. Give every field the
/// systems you ship on need; the `markdown`, `csv`, `json` and `plain_text`
/// presets fill all three.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileType {
    extension: Option<String>,
    mime: Option<String>,
    uti: Option<String>,
}

impl FileType {
    /// A file type with nothing set yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The extension, with or without the leading dot (`"md"` and `".md"` are the same). Windows uses this.
    pub fn extension(mut self, ext: impl AsRef<str>) -> Self {
        self.extension = Some(normalize_extension(ext.as_ref()));
        self
    }

    /// The MIME type, such as `text/markdown`. Linux uses this.
    pub fn mime(mut self, mime: impl Into<String>) -> Self {
        self.mime = Some(mime.into());
        self
    }

    /// The Uniform Type Identifier, such as `net.daringfireball.markdown`. macOS uses this.
    pub fn uti(mut self, uti: impl Into<String>) -> Self {
        self.uti = Some(uti.into());
        self
    }

    /// Markdown (`.md`).
    pub fn markdown() -> Self {
        Self::new()
            .extension(".md")
            .mime("text/markdown")
            .uti("net.daringfireball.markdown")
    }

    /// Plain text (`.txt`).
    pub fn plain_text() -> Self {
        Self::new()
            .extension(".txt")
            .mime("text/plain")
            .uti("public.plain-text")
    }

    /// Comma-separated values (`.csv`).
    pub fn csv() -> Self {
        Self::new()
            .extension(".csv")
            .mime("text/csv")
            .uti("public.comma-separated-values-text")
    }

    /// JSON (`.json`).
    pub fn json() -> Self {
        Self::new()
            .extension(".json")
            .mime("application/json")
            .uti("public.json")
    }

    /// The extension with its leading dot, lower-cased.
    pub fn extension_str(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    /// The MIME type.
    pub fn mime_str(&self) -> Option<&str> {
        self.mime.as_deref()
    }

    /// The Uniform Type Identifier.
    pub fn uti_str(&self) -> Option<&str> {
        self.uti.as_deref()
    }
}

/// What an app declares about itself so the OS lists it under "Open with" and
/// in the Default apps settings, without an installer.
///
/// macOS apps declare this in `Info.plist` inside the bundle, so [`crate::register`]
/// is not available there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registration {
    pub(crate) app: App,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) exe: PathBuf,
    pub(crate) icon: Option<PathBuf>,
    pub(crate) file_types: Vec<FileType>,
}

impl Registration {
    /// A registration for `app`, shown to the user as `name`, launched from `exe`.
    pub fn new(app: App, name: impl Into<String>, exe: impl AsRef<Path>) -> Self {
        let name = name.into();
        Self {
            app,
            description: name.clone(),
            name,
            exe: exe.as_ref().to_path_buf(),
            icon: None,
            file_types: Vec::new(),
        }
    }

    /// A one-line description shown by the OS.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = text.into();
        self
    }

    /// An icon file (`.ico` on Windows, an image path or theme name on Linux).
    pub fn icon(mut self, icon: impl AsRef<Path>) -> Self {
        self.icon = Some(icon.as_ref().to_path_buf());
        self
    }

    /// Adds a file type the app can open.
    pub fn file_type(mut self, file_type: FileType) -> Self {
        self.file_types.push(file_type);
        self
    }

    /// The app being registered.
    pub fn app(&self) -> &App {
        &self.app
    }
}
