//! A small CLI to try the crate by hand.
//!
//!   cargo run --example default_app -- query markdown
//!   cargo run --example default_app -- is-default markdown --bundle com.example.app --desktop app.desktop --prog Example.File
//!   cargo run --example default_app -- set markdown --bundle com.example.app --desktop app.desktop --prog Example.File
//!
//! File types: markdown, text, csv, json.

use default_rs::{App, FileType, Outcome};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(command), Some(kind)) = (args.first(), args.get(1)) else {
        eprintln!("usage: default_app <query|is-default|set> <markdown|text|csv|json> [--bundle ID] [--desktop ID] [--prog ID]");
        std::process::exit(2);
    };
    let file_type = match kind.as_str() {
        "markdown" => FileType::markdown(),
        "text" => FileType::plain_text(),
        "csv" => FileType::csv(),
        "json" => FileType::json(),
        other => {
            eprintln!("unknown file type {other}");
            std::process::exit(2);
        }
    };
    let mut app = App::new();
    let mut rest = args[2..].iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().cloned().unwrap_or_default();
        app = match flag.as_str() {
            "--bundle" => app.bundle_id(value),
            "--desktop" => app.desktop_id(value),
            "--prog" => app.prog_id(value),
            other => {
                eprintln!("unknown flag {other}");
                std::process::exit(2);
            }
        };
    }
    let result = match command.as_str() {
        "query" => default_rs::current_default(&file_type)
            .map(|id| println!("{}", id.as_deref().unwrap_or("(none)"))),
        "is-default" => default_rs::is_default(&app, &file_type).map(|yes| println!("{yes}")),
        "set" => default_rs::set_default(&app, &file_type).map(|outcome| match outcome {
            Outcome::Set => println!("set"),
            Outcome::NeedsUserAction => {
                println!("needs you: choose the app in the Settings window that opened")
            }
        }),
        other => {
            eprintln!("unknown command {other}");
            std::process::exit(2);
        }
    };
    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
