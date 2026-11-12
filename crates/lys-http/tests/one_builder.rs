#![cfg(test)]
//! Every reqwest client in the workspace is built by lys-http. A client built
//! anywhere else would take reqwest's own TLS, which in this build has no
//! crypto provider and, given one, verifies through the machine's store, not
//! the bundled roots. So every Rust file under `crates/` and `tests/` is read,
//! and any that builds a reqwest client itself is refused by name. The scanner
//! is shown to fire on each form it refuses, and the number of files it read
//! is asserted, so an empty walk cannot pass.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// The one file allowed to build a client, relative to the repository root.
const BUILDER: &str = "crates/lys-http/src/lib.rs";

/// The constructors a reqwest client is built with.
const CONSTRUCTORS: [&str; 2] = ["::new(", "::builder("];

/// The names `text` gives reqwest's client and its builder: their own paths,
/// and any name a `use` binds them to (`use reqwest::Client as Http;`,
/// `use reqwest::{Client, ...}`).
fn client_names(text: &str) -> Vec<String> {
    let mut names = vec![
        "reqwest::Client".to_owned(),
        "reqwest::ClientBuilder".to_owned(),
    ];
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("use reqwest::") else {
            continue;
        };
        let items = rest
            .trim_end_matches(';')
            .trim_start_matches('{')
            .trim_end_matches('}');
        for item in items.split(',') {
            let item = item.trim();
            let (path, bound) = item
                .split_once(" as ")
                .map_or((item, item), |(path, bound)| (path.trim(), bound.trim()));
            if path == "Client" || path == "ClientBuilder" {
                names.push(bound.to_owned());
            }
        }
    }
    names
}

/// The lines of `text` that build a reqwest client, as `(line number, line)`.
fn builds(text: &str) -> Vec<(usize, String)> {
    let names = client_names(text);
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            names.iter().any(|name| {
                CONSTRUCTORS.iter().any(|constructor| {
                    let call = format!("{name}{constructor}");
                    line.match_indices(&call).any(|(at, _)| {
                        // A name is whole: `CoreClient::new(` is not `Client::new(`.
                        !line[..at]
                            .chars()
                            .next_back()
                            .is_some_and(|before| before.is_alphanumeric() || before == '_')
                    })
                })
            })
        })
        .map(|(index, line)| (index + 1, line.trim().to_owned()))
        .collect()
}

fn rust_files(directory: &Path, found: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("{} could not be read: {error}", directory.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            rust_files(&path, found)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(())
}

#[test]
fn no_reqwest_client_is_built_outside_lys_http() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    rust_files(&root.join("crates"), &mut files)?;
    rust_files(&root.join("tests"), &mut files)?;
    let builder = root.join(BUILDER);
    let mut read = 0;
    let mut built_in_builder = 0;
    let mut elsewhere = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file)
            .map_err(|error| format!("{} could not be read: {error}", file.display()))?;
        read += 1;
        let found = builds(&text);
        if file.ends_with(BUILDER) || *file == builder {
            built_in_builder += found.len();
            continue;
        }
        for (line, code) in found {
            elsewhere.push(format!("{}:{line}: {code}", file.display()));
        }
    }
    // The workspace has hundreds of Rust files; a walk that read few is broken.
    assert!(
        read > 100,
        "only {read} Rust files were read under crates/ and tests/"
    );
    // The builder itself is seen by the scanner, or the scanner is blind.
    assert!(
        built_in_builder >= 1,
        "the scanner does not see lys-http's own builder"
    );
    assert!(
        elsewhere.is_empty(),
        "reqwest clients built outside lys-http (use lys_http::client or lys_http::builder):\n{}",
        elsewhere.join("\n")
    );
    Ok(())
}

#[test]
fn the_scanner_fires_on_every_form_it_refuses() {
    let cases = [
        "let http = reqwest::Client::new();",
        "let http = reqwest::Client::builder().build()?;",
        "let http = reqwest::ClientBuilder::new().build()?;",
        "use reqwest::Client as Http;\nlet http = Http::new();",
        "use reqwest::{Client, Url};\nlet http = Client::builder();",
    ];
    for case in cases {
        assert_eq!(builds(case).len(), 1, "the scanner missed: {case}");
    }
    let clean = [
        "let client = CoreClient::new(id);",
        "let http = lys_http::client()?;",
        "let http = lys_http::builder()?.redirect(reqwest::redirect::Policy::none());",
    ];
    for case in clean {
        assert!(builds(case).is_empty(), "the scanner fired on: {case}");
    }
}
