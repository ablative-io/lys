#![cfg(test)]
//! Every reqwest client in the workspace is built by lys-http. A client built
//! anywhere else would take reqwest's own TLS, which in this build has no
//! crypto provider and, given one, verifies through the machine's store, not
//! the bundled roots. So every Rust file under `crates/` and `tests/` is read,
//! and any that builds a reqwest client itself is refused by name. The scanner
//! is shown to fire on each form it refuses and to stay quiet on the forms it
//! must not refuse, and the number of files it read is asserted, so an empty
//! walk cannot pass.
//!
//! What is refused:
//!
//! - a constructor (`new`, `builder`, `default`) on reqwest's `Client` or
//!   `ClientBuilder`, called or passed as a function, by any path a file gives
//!   them: their own path, from the crate root (`::reqwest::`), a `use` of the
//!   item, a `use` alias, a crate alias (`use reqwest as r;`,
//!   `use reqwest::{self as r};`), and a `use` spread over lines;
//! - a line naming one of those types and calling `::default`
//!   (`let http: reqwest::Client = Default::default();`,
//!   `<reqwest::Client as Default>::default()`);
//! - a struct deriving `Default` with a field holding a client, whose derived
//!   `default()` builds one, unless the field holds it inside a container
//!   whose own default is empty (`Option`, `Vec`, a map, a once-cell).
//!
//! Comments and literals are not code. Each file is read whole, and every
//! comment (line or block, nested) and the inside of every string (over any
//! number of lines, raw or not) and character literal is blanked before a
//! line is read, newlines kept, so a line number still names the file's line.
//! So this file's own cases, which are strings, are not refused, and it is
//! walked like any other file.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// The one file allowed to build a client, relative to the repository root.
const BUILDER: &str = "crates/lys-http/src/lib.rs";

/// The constructors a reqwest client is built with.
const CONSTRUCTORS: [&str; 3] = ["new", "builder", "default"];

/// Containers whose own `Default` holds nothing, so a struct deriving
/// `Default` over a client inside one builds no client.
const EMPTY_BY_DEFAULT: [&str; 7] = [
    "Option<",
    "Vec<",
    "VecDeque<",
    "HashMap<",
    "BTreeMap<",
    "OnceLock<",
    "OnceCell<",
];

/// Whether `c` can be part of a Rust identifier.
fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `text` with every comment and the inside of every string and character
/// literal blanked to spaces, newlines kept, so each line of the result is the
/// code of the same line of `text`. Read whole: line comments, nested block
/// comments, strings over several lines, raw strings (`r"…"`, `r#"…"#`, `br`,
/// `cr`, a backslash inside being no escape) and character literals (`'"'`,
/// `'\''`). A lifetime (`'a`) is code.
fn code(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let at = |index: usize| chars.get(index).copied();
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut kept = String::with_capacity(text.len());
    let mut index = 0;
    while let Some(c) = at(index) {
        let next = at(index + 1);
        if c == '/' && next == Some('/') {
            while let Some(c) = at(index) {
                if c == '\n' {
                    break;
                }
                kept.push(' ');
                index += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0_usize;
            while let Some(c) = at(index) {
                if c == '/' && at(index + 1) == Some('*') {
                    depth += 1;
                    kept.push_str("  ");
                    index += 2;
                } else if c == '*' && at(index + 1) == Some('/') {
                    depth = depth.saturating_sub(1);
                    kept.push_str("  ");
                    index += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    kept.push(blank(c));
                    index += 1;
                }
            }
        } else if let Some((hashes, opener)) = raw_string(&chars, index) {
            kept.extend(chars.iter().skip(index).take(opener));
            index += opener;
            while let Some(c) = at(index) {
                if c == '"' && (1..=hashes).all(|ahead| at(index + ahead) == Some('#')) {
                    kept.push('"');
                    kept.extend(std::iter::repeat_n('#', hashes));
                    index += 1 + hashes;
                    break;
                }
                kept.push(blank(c));
                index += 1;
            }
        } else if c == '"' {
            kept.push('"');
            index += 1;
            while let Some(c) = at(index) {
                index += 1;
                if c == '\\' {
                    kept.push(' ');
                    if let Some(escaped) = at(index) {
                        kept.push(blank(escaped));
                        index += 1;
                    }
                } else if c == '"' {
                    kept.push('"');
                    break;
                } else {
                    kept.push(blank(c));
                }
            }
        } else if c == '\'' && (next == Some('\\') || at(index + 2) == Some('\'')) {
            // A character literal; a lifetime has no closing quote two on.
            kept.push('\'');
            index += 1;
            if next == Some('\\') {
                // The backslash and the character it escapes.
                for _ in 0..2 {
                    if at(index).is_some() {
                        kept.push(' ');
                        index += 1;
                    }
                }
            }
            while let Some(c) = at(index) {
                index += 1;
                if c == '\'' {
                    kept.push('\'');
                    break;
                }
                kept.push(blank(c));
            }
        } else {
            kept.push(c);
            index += 1;
        }
    }
    kept
}

/// Where a raw string literal opens at `index` (`r"`, `r#"`, `br"`, `cr#"`):
/// its number of `#`, and the length of its opener, quote included. A raw
/// identifier (`r#type`) and an identifier ending in `r` open none.
fn raw_string(chars: &[char], index: usize) -> Option<(usize, usize)> {
    if index
        .checked_sub(1)
        .and_then(|before| chars.get(before))
        .is_some_and(|&before| is_ident(before))
    {
        return None;
    }
    let mut at = index;
    if matches!(chars.get(at), Some('b' | 'c')) {
        at += 1;
    }
    if chars.get(at) != Some(&'r') {
        return None;
    }
    at += 1;
    let mut hashes = 0;
    while chars.get(at) == Some(&'#') {
        hashes += 1;
        at += 1;
    }
    (chars.get(at) == Some(&'"')).then_some((hashes, at + 1 - index))
}

/// The text before and after each place `name` occurs in `text` as a whole
/// path: not the tail of a longer identifier (`CoreClient`) or of another path
/// (`openidconnect::Client`), and not the head of a longer identifier
/// (`ClientConfig`). A leading `::` (`::reqwest::Client`) is the crate root
/// and counts.
fn occurrences<'t>(text: &'t str, name: &str) -> Vec<(&'t str, &'t str)> {
    text.match_indices(name)
        .filter_map(|(at, _)| {
            let before = text.get(..at)?;
            let after = text.get(at + name.len()..)?;
            let head = before.strip_suffix("::").unwrap_or(before);
            let whole =
                !head.ends_with(|c: char| is_ident(c) || c == ':') && !after.starts_with(is_ident);
            whole.then_some((before, after))
        })
        .collect()
}

/// The bodies of the `use` statements of `code` (what follows `use `), each
/// joined into one line up to its `;`.
fn use_statements(code: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut open: Option<String> = None;
    for line in code.lines() {
        let line = line.trim();
        let current = match open.take() {
            Some(mut joined) => {
                joined.push(' ');
                joined.push_str(line);
                joined
            }
            None => {
                let body = line.strip_prefix("use ").or_else(|| {
                    line.starts_with("pub")
                        .then(|| line.split_once(" use ").map(|(_, body)| body))
                        .flatten()
                });
                match body {
                    Some(body) => body.to_owned(),
                    None => continue,
                }
            }
        };
        match current.strip_suffix(';') {
            Some(statement) => statements.push(statement.trim().to_owned()),
            None => open = Some(current),
        }
    }
    statements
}

/// The items a `use` body brings in from `krate`, as `(path, bound name)`:
/// `Client as Http` is `("Client", "Http")`, `Client` is `("Client", "Client")`.
fn items<'s>(body: &'s str, krate: &str) -> Vec<(&'s str, &'s str)> {
    let Some(rest) = body
        .strip_prefix(krate)
        .and_then(|rest| rest.strip_prefix("::"))
    else {
        return Vec::new();
    };
    rest.trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .split(',')
        .map(|item| {
            let item = item.trim();
            item.split_once(" as ")
                .map_or((item, item), |(path, bound)| (path.trim(), bound.trim()))
        })
        .collect()
}

/// The names `code` gives reqwest's client and its builder: their own paths,
/// the paths through any alias of the crate, and any name a `use` binds them to.
fn client_names(code: &str) -> Vec<String> {
    let statements = use_statements(code);
    let mut crates = vec!["reqwest".to_owned()];
    for body in &statements {
        if let Some(alias) = body.strip_prefix("reqwest as ") {
            crates.push(alias.trim().to_owned());
        }
        for (path, bound) in items(body, "reqwest") {
            if path == "self" && bound != "self" {
                crates.push(bound.to_owned());
            }
        }
    }
    let mut names = Vec::new();
    for krate in &crates {
        names.push(format!("{krate}::Client"));
        names.push(format!("{krate}::ClientBuilder"));
    }
    for body in &statements {
        for krate in &crates {
            for (path, bound) in items(body, krate) {
                if path == "Client" || path == "ClientBuilder" {
                    names.push(bound.to_owned());
                }
            }
        }
    }
    names
}

/// Whether the code `line` builds a client through one of `names`: a
/// constructor called or passed on it, or `::default` on a line that names it.
fn constructs(line: &str, names: &[String]) -> bool {
    names.iter().any(|name| {
        let found = occurrences(line, name);
        !found.is_empty()
            && (line.contains("::default")
                || found.iter().any(|(_, after)| {
                    after.strip_prefix("::").is_some_and(|call| {
                        CONSTRUCTORS.iter().any(|constructor| {
                            call.strip_prefix(constructor)
                                .is_some_and(|rest| !rest.starts_with(is_ident))
                        })
                    })
                }))
    })
}

/// Whether a struct's text holds one of `names` in a field, outside a
/// container whose own default is empty. A field is read on its own line, as
/// rustfmt writes it.
fn holds_client(fields: &str, names: &[String]) -> bool {
    names.iter().any(|name| {
        occurrences(fields, name).iter().any(|(before, _)| {
            let field = before.rsplit('\n').next().unwrap_or_default();
            !EMPTY_BY_DEFAULT
                .iter()
                .any(|container| field.contains(container))
        })
    })
}

/// The line numbers of the structs in `code` that derive `Default` over a
/// field holding a client. An attribute spread over lines is joined first, so
/// `#[derive(\n    Debug,\n    Default,\n)]` is read as one.
fn default_structs(code: &str, names: &[String]) -> Vec<usize> {
    let lines: Vec<&str> = code.lines().map(str::trim).collect();
    let mut rest = lines.iter().enumerate();
    let mut found = Vec::new();
    let mut deriving_default = false;
    while let Some((index, &line)) = rest.next() {
        if line.starts_with("#[") {
            let mut attribute = line.to_owned();
            while attribute.matches('[').count() > attribute.matches(']').count() {
                let Some((_, &next)) = rest.next() else {
                    break;
                };
                attribute.push(' ');
                attribute.push_str(next);
            }
            if attribute.starts_with("#[derive(")
                && attribute
                    .split(|c: char| !is_ident(c))
                    .any(|word| word == "Default")
            {
                deriving_default = true;
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if deriving_default && line.split_whitespace().any(|word| word == "struct") {
            let mut fields = String::new();
            let mut depth = 0_usize;
            let mut opened = false;
            let mut current = line;
            loop {
                for c in current.chars() {
                    match c {
                        '{' | '(' => {
                            depth += 1;
                            opened = true;
                        }
                        '}' | ')' => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                }
                fields.push_str(current);
                fields.push('\n');
                if (opened && depth == 0) || (!opened && current.ends_with(';')) {
                    break;
                }
                let Some((_, &next)) = rest.next() else {
                    break;
                };
                current = next;
            }
            if holds_client(&fields, names) {
                found.push(index + 1);
            }
        }
        deriving_default = false;
    }
    found
}

/// The lines of `text` that build a reqwest client, as `(line number, line)`.
fn builds(text: &str) -> Vec<(usize, String)> {
    let code = code(text);
    let names = client_names(&code);
    let lines: Vec<&str> = text.lines().collect();
    let mut found: Vec<(usize, String)> = code
        .lines()
        .enumerate()
        .filter(|(_, line)| constructs(line, &names))
        .map(|(index, _)| {
            let line = lines.get(index).map_or("", |line| line.trim());
            (index + 1, line.to_owned())
        })
        .collect();
    for number in default_structs(&code, &names) {
        let line = lines
            .get(number.saturating_sub(1))
            .map_or("", |line| line.trim());
        found.push((number, format!("derives Default over a client: {line}")));
    }
    found
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
    let mut read = 0;
    let mut built_in_builder = 0;
    let mut elsewhere = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file)
            .map_err(|error| format!("{} could not be read: {error}", file.display()))?;
        read += 1;
        let found = builds(&text);
        if file.ends_with(BUILDER) {
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
        "let http = reqwest::Client::default();",
        "let http = reqwest::ClientBuilder::default().build()?;",
        "let http = ::reqwest::Client::new();",
        "let http = cached.unwrap_or_else(reqwest::Client::new);",
        "let http: reqwest::Client = Default::default();",
        "let http = <reqwest::Client as Default>::default();",
        "use reqwest::Client as Http;\nlet http = Http::new();",
        "use reqwest::{Client, Url};\nlet http = Client::builder();",
        "use reqwest::Client; // the one client\nlet http = Client::new();",
        "pub(crate) use reqwest::Client;\nlet http = Client::new();",
        "use reqwest::{\n    Client,\n    Url,\n};\nlet http = Client::new();",
        "use reqwest::{\n    Url,\n    ClientBuilder as Make,\n};\nlet http = Make::default();",
        "use reqwest as r;\nlet http = r::Client::new();",
        "use reqwest as r;\nuse r::Client as C;\nlet http = C::default();",
        "use reqwest::{self as r, Url};\nlet http = r::Client::builder();",
        "#[derive(Debug, Default)]\nstruct Holder {\n    http: reqwest::Client,\n}",
        "use reqwest::Client;\n#[derive(Default)]\npub struct Holder {\n    pub http: Client,\n    name: String,\n}",
        "#[derive(\n    Debug,\n    Default,\n)]\nstruct Holder {\n    http: std::sync::Arc<reqwest::Client>,\n}",
        "#[derive(Default)]\nstruct Holder(reqwest::Client);",
        "let page = fetch(\"https://lys.example/\"); let http = reqwest::Client::new();",
        "let quote = '\"'; let http = reqwest::Client::new(); // the one client",
        "let doc = \"first\nsecond\";\nlet http = reqwest::Client::new();",
        "/* a comment */ let http = reqwest::Client::new();",
        "let said = r#\"say \"hi\" // not a comment\"#; let http = reqwest::Client::new();",
        "let path = r\"C:\\\"; let http = reqwest::Client::new();",
        "let quote = '\\''; let tick: &'static str = \"\"; let http = reqwest::Client::new();",
        "#[derive(Default)]\n#[serde(\n    rename_all = \"snake_case\"\n)]\npub(crate) struct Holder {\n    /// The client.\n    http: reqwest::Client,\n}",
    ];
    for case in cases {
        assert_eq!(builds(case).len(), 1, "the scanner missed: {case}");
    }
    let clean = [
        "let client = CoreClient::new(id);",
        "let http = lys_http::client()?;",
        "let http = lys_http::builder()?.redirect(reqwest::redirect::Policy::none());",
        "use reqwest::Client;\nlet oidc = openidconnect::Client::new(id);",
        "let config = reqwest::ClientConfig::new();",
        "// reqwest::Client::new() is never called here",
        "let refused = \"reqwest::Client::new() is refused\";",
        "let doc = \"reqwest::Client::new()\nis refused\";",
        "/*\nlet http = reqwest::Client::new();\n*/",
        "/* outer /* inner */\nlet http = reqwest::Client::new();\n*/",
        "let said = r#\"\nlet http = reqwest::Client::new();\n\"#;",
        "let said = r#\"a \"quoted\" reqwest::Client::new()\"#;",
        "let page = \"https://lys.example/\"; // reqwest::Client::new() is not called",
        "use reqwest::{\n    Url,\n    header,\n};\nlet client = Client::new();",
        "#[derive(Default)]\nstruct Counts {\n    clients: usize,\n}",
        "#[derive(Clone)]\nstruct Holder {\n    http: reqwest::Client,\n}",
        "#[derive(Default)]\nstruct Holder {\n    http: Option<reqwest::Client>,\n}",
        "#[derive(Debug, Default)]\nstruct Counts;\nstruct Holder {\n    http: reqwest::Client,\n}",
        "#[derive(Default)]\nenum Mode {\n    #[default]\n    Plain,\n}\nstruct Holder {\n    http: reqwest::Client,\n}",
    ];
    for case in clean {
        assert!(builds(case).is_empty(), "the scanner fired on: {case}");
    }
}
