# default-rs

Query and set the **default app for a file type** on macOS, Windows and Linux from one Rust API, and
register an app as a handler **without an installer**.

Every OS names apps and file types differently, and Windows will not let an app make itself the
default. This crate hides the naming differences and is honest about what each OS allows.

| | App identifier | File type | Set the default | Register without an installer |
|---|---|---|---|---|
| **macOS** | `bundle_id` | `uti` | Yes, silently (LaunchServices). The app must be installed and known to macOS. | No: declare types in `Info.plist` |
| **Linux** | `desktop_id` | `mime` | Yes, silently (`xdg-mime`). Needs the `.desktop` file. | Yes: writes `~/.local/share/applications/<id>` |
| **Windows** | `prog_id` | `extension` | **No.** Opens Settings > Default apps and returns `NeedsUserAction`. | Yes: ProgID, "Open with" and Capabilities under `HKEY_CURRENT_USER` |

```toml
[dependencies]
default-rs = "0.1"
```

```rust
use default_rs::{App, FileType, Outcome};

let app = App::new()
    .bundle_id("com.example.editor")        // macOS
    .desktop_id("editor.desktop")           // Linux
    .prog_id("Example.Editor.File");        // Windows

// Who opens Markdown now?
println!("{:?}", default_rs::current_default(&FileType::markdown())?);

// Is it us?
if !default_rs::is_default(&app, &FileType::markdown())? {
    match default_rs::set_default(&app, &FileType::markdown())? {
        Outcome::Set => println!("done"),
        Outcome::NeedsUserAction => println!("choose the app in the Settings window that opened"),
    }
}
# Ok::<(), default_rs::Error>(())
```

`FileType::markdown()`, `plain_text()`, `csv()` and `json()` fill in all three names. Build your own with
`FileType::new().extension(".log").mime("text/x-log").uti("public.log")`.

## Registering a portable app (Linux AppImage, Windows without an installer)

```rust
use default_rs::{App, FileType, Registration};

let reg = Registration::new(
    App::new().desktop_id("editor.desktop").prog_id("Example.Editor.File"),
    "Example Editor",
    std::env::current_exe()?,
)
.description("A text editor")
.file_type(FileType::markdown())
.file_type(FileType::csv());

default_rs::register(&reg)?;   // now listed under "Open with" and in Default apps
// ...
default_rs::unregister(&reg)?; // removes exactly what register() added
# Ok::<(), Box<dyn std::error::Error>>(())
```

After `register`, `set_default` works on Linux, and on Windows the app appears by name in
Settings > Default apps.

## Try it

```sh
cargo run --example default_app -- query markdown
cargo run --example default_app -- set markdown --bundle com.example.editor --desktop editor.desktop --prog Example.Editor.File
```

## What is tested

- The registry layout, the `.desktop` file, `xdg-mime` output parsing and XDG directory rules are pure
  functions with unit tests that run on every OS.
- CI runs the integration tests on all three: the real `xdg-mime` on Linux (private `HOME`), the real
  registry on Windows (a unique ProgID, removed afterwards), and read-only LaunchServices queries on macOS.
- Setting a default on macOS is not exercised in CI because it would change the runner user's settings.

## Limits

- **Windows cannot be set silently.** Windows 10 and 11 guard the choice with a per-user hash. Anything that
  claims to set it directly is writing around that protection and is brittle.
- **macOS needs an installed app.** LaunchServices only accepts bundle ids it has seen.
- **Linux desktops differ.** `xdg-mime` writes `mimeapps.list`; some desktops cache it until the next login.

## License

MIT
