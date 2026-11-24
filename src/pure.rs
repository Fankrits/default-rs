//! Logic that does not touch the OS, so every platform's behaviour is unit-tested on every host.
// Each OS backend uses a different subset of these.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::types::{Host, Registration};

/// `md`, `.MD` and ` .md ` all become `.md`.
pub(crate) fn normalize_extension(ext: &str) -> String {
    format!(
        ".{}",
        ext.trim().trim_start_matches('.').to_ascii_lowercase()
    )
}

/// Whether the handler the OS reports is the app. macOS bundle ids and Windows ProgIDs
/// compare case-insensitively; desktop-file ids do not.
pub(crate) fn same_handler(host: Host, current: &str, wanted: &str) -> bool {
    match host {
        Host::Linux => current == wanted,
        Host::Mac | Host::Windows => current.eq_ignore_ascii_case(wanted),
    }
}

/// `xdg-mime query default` prints the desktop id and a newline, or nothing.
pub(crate) fn parse_xdg_default(output: &str) -> Option<String> {
    let id = output.trim();
    (!id.is_empty()).then(|| id.to_string())
}

fn applications_in(base: &str) -> PathBuf {
    PathBuf::from(format!("{}/applications", base.trim_end_matches('/')))
}

/// The directories searched for an installed `.desktop` file, per the XDG spec.
/// XDG paths are Unix paths, so this tests for a leading `/` as text and joins with `/`:
/// it behaves the same (and is testable) on every host.
pub(crate) fn desktop_file_dirs(
    xdg_data_home: Option<&str>,
    xdg_data_dirs: Option<&str>,
    home: Option<&str>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.extend(user_applications_dir(xdg_data_home, home));
    let system = xdg_data_dirs
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("/usr/local/share:/usr/share");
    for dir in system.split(':').filter(|d| d.starts_with('/')) {
        dirs.push(applications_in(dir));
    }
    dirs
}

/// The per-user directory a registration writes its `.desktop` file to.
pub(crate) fn user_applications_dir(
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> Option<PathBuf> {
    if let Some(dir) = xdg_data_home.filter(|v| v.starts_with('/')) {
        return Some(applications_in(dir));
    }
    home.filter(|h| !h.is_empty())
        .map(|h| applications_in(&format!("{}/.local/share", h.trim_end_matches('/'))))
}

fn no_control(value: &str, what: &str) -> Result<()> {
    if value.chars().any(|c| c.is_control()) {
        Err(Error::InvalidInput(format!(
            "{what} contains a control character"
        )))
    } else {
        Ok(())
    }
}

/// An `Exec=` program path: bare when plain, double-quoted when it has spaces or shell
/// characters. Characters that need multi-level escaping (`"`, `` ` ``, `$`, `\`) are rejected
/// rather than escaped, because a wrong escape would launch the wrong thing.
pub(crate) fn quote_exec(path: &Path) -> Result<String> {
    let text = path
        .to_str()
        .ok_or_else(|| Error::InvalidInput("the executable path is not valid UTF-8".into()))?;
    no_control(text, "the executable path")?;
    if text.is_empty() {
        return Err(Error::InvalidInput("the executable path is empty".into()));
    }
    if text.chars().any(|c| matches!(c, '"' | '`' | '$' | '\\')) {
        return Err(Error::InvalidInput(
            "the executable path contains one of \" ` $ \\".into(),
        ));
    }
    let plain = text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+'));
    Ok(if plain {
        text.to_string()
    } else {
        format!("\"{text}\"")
    })
}

/// The text of the `.desktop` file for a registration.
pub(crate) fn render_desktop_entry(reg: &Registration) -> Result<String> {
    no_control(&reg.name, "the name")?;
    no_control(&reg.description, "the description")?;
    let exec = quote_exec(&reg.exe)?;
    let mimes: Vec<&str> = reg.file_types.iter().filter_map(|t| t.mime_str()).collect();
    if mimes.is_empty() {
        return Err(Error::MissingFileType("mime"));
    }
    let mut out = String::from("[Desktop Entry]\nType=Application\n");
    out.push_str(&format!("Name={}\nComment={}\n", reg.name, reg.description));
    out.push_str(&format!("Exec={exec} %F\n"));
    if let Some(icon) = &reg.icon {
        let icon = icon
            .to_str()
            .ok_or_else(|| Error::InvalidInput("the icon path is not valid UTF-8".into()))?;
        no_control(icon, "the icon path")?;
        out.push_str(&format!("Icon={icon}\n"));
    }
    out.push_str("Terminal=false\nCategories=Utility;\n");
    out.push_str(&format!("MimeType={};\n", mimes.join(";")));
    Ok(out)
}

/// One value to write under `HKEY_CURRENT_USER`. An empty `name` is the key's default value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegWrite {
    pub subkey: String,
    pub name: String,
    pub data: String,
}

/// What removing a registration deletes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RegRemoval {
    /// Keys deleted with everything under them.
    pub trees: Vec<String>,
    /// Single values deleted: `(subkey, value name)`.
    pub values: Vec<(String, String)>,
}

/// `Example Editor!` becomes `ExampleEditor`: the registry key an app's capabilities live under.
pub(crate) fn registry_app_key(name: &str) -> String {
    let key: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if key.is_empty() {
        "App".to_string()
    } else {
        key
    }
}

fn windows_parts(reg: &Registration) -> Result<(String, Vec<String>)> {
    let prog = reg
        .app
        .id_for(Host::Windows)
        .ok_or(Error::MissingIdentifier("prog_id"))?;
    if prog.is_empty() || prog.contains(['\\', '/']) || prog.chars().any(|c| c.is_control()) {
        return Err(Error::InvalidInput(
            "the prog_id is empty or contains \\ / or a control character".into(),
        ));
    }
    no_control(&reg.name, "the name")?;
    no_control(&reg.description, "the description")?;
    let exts: Vec<String> = reg
        .file_types
        .iter()
        .filter_map(|t| t.extension_str().map(str::to_string))
        .collect();
    if exts.is_empty() {
        return Err(Error::MissingFileType("extension"));
    }
    Ok((prog.to_string(), exts))
}

/// Every registry value `register` writes for Windows: the ProgID with its open command,
/// the "Open with" entries, and the Capabilities that list the app in Settings > Default apps.
pub(crate) fn windows_register_plan(reg: &Registration) -> Result<Vec<RegWrite>> {
    let (prog, exts) = windows_parts(reg)?;
    let exe = reg
        .exe
        .to_str()
        .ok_or_else(|| Error::InvalidInput("the executable path is not valid UTF-8".into()))?;
    no_control(exe, "the executable path")?;
    if exe.contains('"') {
        return Err(Error::InvalidInput(
            "the executable path contains a quote".into(),
        ));
    }
    let key = registry_app_key(&reg.name);
    let write = |subkey: String, name: &str, data: &str| RegWrite {
        subkey,
        name: name.to_string(),
        data: data.to_string(),
    };
    let mut plan = vec![write(
        format!(r"Software\Classes\{prog}"),
        "",
        &reg.description,
    )];
    if let Some(icon) = reg.icon.as_ref().and_then(|i| i.to_str()) {
        plan.push(write(
            format!(r"Software\Classes\{prog}\DefaultIcon"),
            "",
            icon,
        ));
    }
    plan.push(write(
        format!(r"Software\Classes\{prog}\shell\open\command"),
        "",
        &format!("\"{exe}\" \"%1\""),
    ));
    for ext in &exts {
        plan.push(write(
            format!(r"Software\Classes\{ext}\OpenWithProgids"),
            &prog,
            "",
        ));
    }
    let caps = format!(r"Software\{key}\Capabilities");
    plan.push(write(caps.clone(), "ApplicationName", &reg.name));
    plan.push(write(
        caps.clone(),
        "ApplicationDescription",
        &reg.description,
    ));
    for ext in &exts {
        plan.push(write(format!(r"{caps}\FileAssociations"), ext, &prog));
    }
    plan.push(write(
        r"Software\RegisteredApplications".into(),
        &reg.name,
        &caps,
    ));
    Ok(plan)
}

/// What `unregister` removes: exactly what `windows_register_plan` added.
pub(crate) fn windows_unregister_plan(reg: &Registration) -> Result<RegRemoval> {
    let (prog, exts) = windows_parts(reg)?;
    let key = registry_app_key(&reg.name);
    let mut values: Vec<(String, String)> = exts
        .iter()
        .map(|ext| {
            (
                format!(r"Software\Classes\{ext}\OpenWithProgids"),
                prog.clone(),
            )
        })
        .collect();
    values.push((r"Software\RegisteredApplications".into(), reg.name.clone()));
    Ok(RegRemoval {
        trees: vec![
            format!(r"Software\Classes\{prog}"),
            format!(r"Software\{key}"),
        ],
        values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{App, FileType};

    fn reg() -> Registration {
        Registration::new(
            App::new()
                .prog_id("Example.File")
                .desktop_id("example.desktop"),
            "Example Editor",
            "/opt/example/example",
        )
        .description("Edits things")
        .icon("/opt/example/example.ico")
        .file_type(FileType::markdown())
        .file_type(FileType::csv())
    }

    #[test]
    fn extensions_are_normalised() {
        assert_eq!(normalize_extension("md"), ".md");
        assert_eq!(normalize_extension(".MD"), ".md");
        assert_eq!(normalize_extension(" ..Csv "), ".csv");
    }

    #[test]
    fn handlers_compare_by_each_systems_rules() {
        assert!(same_handler(
            Host::Mac,
            "COM.Example.App",
            "com.example.app"
        ));
        assert!(same_handler(Host::Windows, "example.file", "Example.File"));
        assert!(!same_handler(
            Host::Linux,
            "Example.desktop",
            "example.desktop"
        ));
        assert!(same_handler(
            Host::Linux,
            "example.desktop",
            "example.desktop"
        ));
    }

    #[test]
    fn xdg_output_is_trimmed_and_empty_means_none() {
        assert_eq!(
            parse_xdg_default("example.desktop\n").as_deref(),
            Some("example.desktop")
        );
        assert_eq!(parse_xdg_default("  \n"), None);
        assert_eq!(parse_xdg_default(""), None);
    }

    #[test]
    fn desktop_dirs_follow_the_xdg_spec() {
        assert_eq!(
            desktop_file_dirs(None, None, Some("/home/me")),
            vec![
                PathBuf::from("/home/me/.local/share/applications"),
                PathBuf::from("/usr/local/share/applications"),
                PathBuf::from("/usr/share/applications"),
            ]
        );
        assert_eq!(
            desktop_file_dirs(Some("rel"), Some("/opt/a:rel:/opt/b/"), Some("/home/me")),
            vec![
                PathBuf::from("/home/me/.local/share/applications"),
                PathBuf::from("/opt/a/applications"),
                PathBuf::from("/opt/b/applications"),
            ]
        );
        assert_eq!(
            desktop_file_dirs(Some("/x/"), Some(""), None)[0],
            PathBuf::from("/x/applications")
        );
    }

    #[test]
    fn the_user_applications_dir_prefers_xdg_data_home() {
        assert_eq!(
            user_applications_dir(Some("/data"), Some("/home/me")),
            Some(PathBuf::from("/data/applications"))
        );
        assert_eq!(
            user_applications_dir(None, Some("/home/me")),
            Some(PathBuf::from("/home/me/.local/share/applications"))
        );
        assert_eq!(user_applications_dir(None, None), None);
        assert_eq!(user_applications_dir(Some("relative"), Some("")), None);
    }

    #[test]
    fn exec_paths_are_quoted_only_when_needed_and_dangerous_ones_are_rejected() {
        assert_eq!(
            quote_exec(Path::new("/usr/bin/app")).unwrap(),
            "/usr/bin/app"
        );
        assert_eq!(
            quote_exec(Path::new("/opt/My App/app")).unwrap(),
            "\"/opt/My App/app\""
        );
        for bad in ["/a\"b", "/a`b", "/a$b", "/a\\b", "/a\nb", ""] {
            assert!(quote_exec(Path::new(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_desktop_entry_lists_every_mime_type_and_the_command() {
        let text = render_desktop_entry(&reg()).unwrap();
        assert!(text.starts_with("[Desktop Entry]\nType=Application\n"));
        assert!(text.contains("Name=Example Editor\n"));
        assert!(text.contains("Exec=/opt/example/example %F\n"));
        assert!(text.contains("Icon=/opt/example/example.ico\n"));
        assert!(text.contains("MimeType=text/markdown;text/csv;\n"));
        let no_mime = Registration::new(App::new().desktop_id("a.desktop"), "A", "/a")
            .file_type(FileType::new().extension("x"));
        assert!(matches!(
            render_desktop_entry(&no_mime),
            Err(Error::MissingFileType("mime"))
        ));
        let bad_name =
            Registration::new(App::new(), "A\nExec=evil", "/a").file_type(FileType::markdown());
        assert!(
            render_desktop_entry(&bad_name).is_err(),
            "a newline must not inject a key"
        );
    }

    #[test]
    fn the_windows_plan_registers_the_progid_open_with_and_capabilities() {
        let plan = windows_register_plan(&reg()).unwrap();
        let find = |subkey: &str, name: &str| {
            plan.iter()
                .find(|w| w.subkey == subkey && w.name == name)
                .map(|w| w.data.as_str())
        };
        assert_eq!(
            find(r"Software\Classes\Example.File", ""),
            Some("Edits things")
        );
        assert_eq!(
            find(r"Software\Classes\Example.File\shell\open\command", ""),
            Some("\"/opt/example/example\" \"%1\"")
        );
        assert_eq!(
            find(r"Software\Classes\Example.File\DefaultIcon", ""),
            Some("/opt/example/example.ico")
        );
        assert_eq!(
            find(r"Software\Classes\.md\OpenWithProgids", "Example.File"),
            Some("")
        );
        assert_eq!(
            find(r"Software\Classes\.csv\OpenWithProgids", "Example.File"),
            Some("")
        );
        assert_eq!(
            find(r"Software\ExampleEditor\Capabilities", "ApplicationName"),
            Some("Example Editor")
        );
        assert_eq!(
            find(
                r"Software\ExampleEditor\Capabilities\FileAssociations",
                ".md"
            ),
            Some("Example.File")
        );
        assert_eq!(
            find(r"Software\RegisteredApplications", "Example Editor"),
            Some(r"Software\ExampleEditor\Capabilities")
        );
    }

    #[test]
    fn unregister_removes_only_what_register_added() {
        let removal = windows_unregister_plan(&reg()).unwrap();
        assert_eq!(
            removal.trees,
            vec![
                r"Software\Classes\Example.File".to_string(),
                r"Software\ExampleEditor".to_string()
            ]
        );
        assert!(removal.values.contains(&(
            r"Software\Classes\.md\OpenWithProgids".to_string(),
            "Example.File".to_string()
        )));
        assert!(removal.values.contains(&(
            r"Software\RegisteredApplications".to_string(),
            "Example Editor".to_string()
        )));
        // Every value register writes sits in a removed tree, is an OpenWith entry, or is the RegisteredApplications value.
        for w in windows_register_plan(&reg()).unwrap() {
            let covered = removal
                .trees
                .iter()
                .any(|t| w.subkey == *t || w.subkey.starts_with(&format!("{t}\\")))
                || removal
                    .values
                    .iter()
                    .any(|(k, n)| *k == w.subkey && *n == w.name);
            assert!(covered, "{} / {} would be left behind", w.subkey, w.name);
        }
    }

    #[test]
    fn the_windows_plan_rejects_unusable_input() {
        let no_prog =
            Registration::new(App::new(), "A", "C:\\a.exe").file_type(FileType::markdown());
        assert!(matches!(
            windows_register_plan(&no_prog),
            Err(Error::MissingIdentifier("prog_id"))
        ));
        let no_ext = Registration::new(App::new().prog_id("A.File"), "A", "C:\\a.exe")
            .file_type(FileType::new().mime("text/plain"));
        assert!(matches!(
            windows_register_plan(&no_ext),
            Err(Error::MissingFileType("extension"))
        ));
        let slash = Registration::new(App::new().prog_id("A\\File"), "A", "C:\\a.exe")
            .file_type(FileType::markdown());
        assert!(windows_register_plan(&slash).is_err());
        let quote = Registration::new(App::new().prog_id("A.File"), "A", "C:\\a\".exe")
            .file_type(FileType::markdown());
        assert!(windows_register_plan(&quote).is_err());
    }

    #[test]
    fn the_registry_app_key_keeps_only_letters_and_digits() {
        assert_eq!(registry_app_key("Example Editor!"), "ExampleEditor");
        assert_eq!(registry_app_key("!!!"), "App");
    }
}
