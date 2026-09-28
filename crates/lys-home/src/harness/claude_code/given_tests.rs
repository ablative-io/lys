#![cfg(test)]
//! Gates on the resolution: the measured order, absent positions omitted,
//! one directory's three files in the request's order, the config directory
//! from the template or from HOME, the user CLAUDE.md listed once, the slug
//! the memory index is found under, lengths and hashes from one read, an
//! unreadable document refused by path, and nothing written.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::error::HomeError;
use crate::harness::claude_code::given::{
    ConfigDir, ConfigSource, DocumentKind, GivenDocument, resolve_given,
};
use crate::harness::claude_code::projects_slug;

type Outcome = Result<(), Box<dyn std::error::Error>>;

const SENTENCE: &[u8] = b"the fixture sentence that must never leave the file\n";
const INSTRUCTIONS: &[u8] = b"fixture instructions\n";
const MCP: &[u8] = b"{\"mcpServers\": {}}\n";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").expect("writing to a String cannot fail");
            s
        })
}

fn put(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, bytes)
}

/// The path of the memory index for `cwd` under `config`.
fn memory_index(config: &Path, cwd: &str) -> PathBuf {
    config
        .join("projects")
        .join(projects_slug(cwd))
        .join("memory")
        .join("MEMORY.md")
}

/// A working directory `w` holding a CLAUDE.md, a config directory `c`
/// holding the memory index for it and no CLAUDE.md, and an out directory
/// `o` holding the two written files.
fn fixture(dir: &Path) -> std::io::Result<(PathBuf, PathBuf, PathBuf)> {
    let w = dir.join("w");
    let c = dir.join("c");
    let o = dir.join("o");
    put(&w.join("CLAUDE.md"), SENTENCE)?;
    put(&memory_index(&c, &w.to_string_lossy()), b"memory index\n")?;
    put(&o.join("instructions.md"), INSTRUCTIONS)?;
    put(&o.join("mcp.json"), MCP)?;
    Ok((w, c, o))
}

fn template(c: &Path) -> ConfigDir {
    ConfigDir {
        path: c.to_path_buf(),
        source: ConfigSource::Template,
    }
}

fn cwd(w: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(w.to_str().ok_or("a UTF-8 path")?)
}

fn kinds(documents: &[GivenDocument]) -> Vec<DocumentKind> {
    documents.iter().map(|d| d.kind).collect()
}

fn paths(documents: &[GivenDocument]) -> Vec<&Path> {
    documents.iter().map(|d| d.path.as_path()).collect()
}

/// Every file under `dir`, with its length and modification time.
fn listing(dir: &Path) -> std::io::Result<Vec<(PathBuf, u64, SystemTime)>> {
    let mut out = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        for entry in std::fs::read_dir(&d)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                pending.push(entry.path());
            } else {
                out.push((entry.path(), meta.len(), meta.modified()?));
            }
        }
    }
    out.sort();
    Ok(out)
}

#[test]
fn the_measured_order_appended_mcp_chain_memory_with_relative_and_absolute_paths() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let (w, c, o) = fixture(&base)?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    let documents = &resolution.documents;
    assert_eq!(documents.len(), 4);
    assert_eq!(
        kinds(documents),
        [
            DocumentKind::AppendedInstructions,
            DocumentKind::McpConfig,
            DocumentKind::ClaudeMdChain,
            DocumentKind::MemoryIndex,
        ]
    );
    assert_eq!(
        paths(documents),
        [
            Path::new("instructions.md"),
            Path::new("mcp.json"),
            w.join("CLAUDE.md").as_path(),
            memory_index(&c, cwd(&w)?).as_path(),
        ]
    );
    assert_eq!(resolution.config_dir, template(&c));
    for (document, bytes) in
        documents
            .iter()
            .zip([INSTRUCTIONS, MCP, SENTENCE, b"memory index\n".as_slice()])
    {
        assert_eq!(
            document.length,
            bytes.len() as u64,
            "{}",
            document.kind.name()
        );
        assert_eq!(
            document.sha256,
            sha256_hex(bytes),
            "{}",
            document.kind.name()
        );
    }
    Ok(())
}

#[test]
fn a_config_claude_md_is_listed_after_the_appended_instructions_and_mcp_config() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let (w, c, o) = fixture(&base)?;
    put(&c.join("CLAUDE.md"), b"user instructions\n")?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 5);
    assert_eq!(
        kinds(&resolution.documents),
        [
            DocumentKind::AppendedInstructions,
            DocumentKind::McpConfig,
            DocumentKind::UserClaudeMd,
            DocumentKind::ClaudeMdChain,
            DocumentKind::MemoryIndex,
        ]
    );
    assert_eq!(resolution.documents[2].path, c.join("CLAUDE.md"));
    Ok(())
}

#[test]
fn an_ancestor_claude_md_is_listed_before_the_working_directory_s() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let (w, c, o) = fixture(&base)?;
    put(&base.join("CLAUDE.md"), b"ancestor instructions\n")?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 5);
    assert_eq!(resolution.documents[2].kind, DocumentKind::ClaudeMdChain);
    assert_eq!(resolution.documents[2].path, base.join("CLAUDE.md"));
    assert_eq!(resolution.documents[3].kind, DocumentKind::ClaudeMdChain);
    assert_eq!(resolution.documents[3].path, w.join("CLAUDE.md"));
    Ok(())
}

#[test]
fn one_directory_lists_claude_md_then_dot_claude_then_local_whatever_the_creation_order() -> Outcome
{
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let w = base.join("w");
    let c = base.join("c");
    let o = base.join("o");
    put(&w.join("CLAUDE.local.md"), b"local\n")?;
    put(&w.join(".claude").join("CLAUDE.md"), b"dot claude\n")?;
    put(&w.join("CLAUDE.md"), b"claude\n")?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    let chain: Vec<&GivenDocument> = resolution
        .documents
        .iter()
        .filter(|d| d.kind == DocumentKind::ClaudeMdChain)
        .collect();
    assert_eq!(resolution.documents.len(), 3);
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].path, w.join("CLAUDE.md"));
    assert_eq!(chain[1].path, w.join(".claude").join("CLAUDE.md"));
    assert_eq!(chain[2].path, w.join("CLAUDE.local.md"));
    Ok(())
}

#[test]
fn no_memory_index_gives_no_memory_document_and_the_call_succeeds() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let (w, c, o) = fixture(&base)?;
    std::fs::remove_file(memory_index(&c, cwd(&w)?))?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 3);
    assert_eq!(
        kinds(&resolution.documents)
            .iter()
            .filter(|k| **k == DocumentKind::MemoryIndex)
            .count(),
        0
    );
    Ok(())
}

#[test]
fn the_config_directory_is_the_template_s_or_else_home_dot_claude() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let (w, c, o) = fixture(&base)?;
    let h = base.join("h");
    put(
        &memory_index(&h.join(".claude"), cwd(&w)?),
        b"home memory\n",
    )?;
    let from_template = ConfigDir::resolve(c.to_str(), Some(h.as_path()))?;
    assert_eq!(from_template, template(&c));
    let listed = resolve_given(cwd(&w)?, from_template, &o)?;
    assert_eq!(listed.documents.len(), 4);
    assert_eq!(listed.documents[3].path, memory_index(&c, cwd(&w)?));
    assert_eq!(listed.config_dir.source, ConfigSource::Template);
    let from_home = ConfigDir::resolve(None, Some(h.as_path()))?;
    assert_eq!(
        from_home,
        ConfigDir {
            path: h.join(".claude"),
            source: ConfigSource::Home,
        }
    );
    let listed = resolve_given(cwd(&w)?, from_home, &o)?;
    assert_eq!(listed.documents.len(), 4);
    assert_eq!(
        listed.documents[3].path,
        memory_index(&h.join(".claude"), cwd(&w)?)
    );
    assert_eq!(
        paths(&listed.documents)
            .iter()
            .filter(|p| p.starts_with(&c))
            .count(),
        0
    );
    assert_eq!(listed.config_dir.path, h.join(".claude"));
    assert_eq!(listed.config_dir.source, ConfigSource::Home);
    Ok(())
}

#[test]
fn neither_a_template_variable_nor_home_is_refused_by_name() {
    let refused = ConfigDir::resolve(None, None);
    assert!(matches!(refused, Err(HomeError::NoConfigDir)));
}

#[test]
fn an_empty_home_is_no_home() {
    let refused = ConfigDir::resolve(None, Some(Path::new("")));
    assert!(matches!(refused, Err(HomeError::NoConfigDir)));
}

#[test]
fn a_config_dir_that_is_not_absolute_is_refused_naming_the_variable_and_its_shape() -> Outcome {
    for (value, shape) in [
        ("", "empty"),
        (".claude", "relative"),
        ("~/.claude", "beginning with `~`"),
    ] {
        let refused = ConfigDir::resolve(Some(value), Some(Path::new("/h")));
        let Err(error) = refused else {
            return Err(format!("a config dir of `{value}` must be refused").into());
        };
        let text = error.to_string();
        assert!(
            matches!(&error, HomeError::NotAbsolute { what: "CLAUDE_CONFIG_DIR", path, .. } if path == Path::new(value)),
            "{text}"
        );
        assert!(text.contains("CLAUDE_CONFIG_DIR"), "{text}");
        assert!(text.contains(shape), "{text}");
        assert!(text.contains("from the root"), "{text}");
    }
    let refused = ConfigDir::resolve(None, Some(Path::new("~")));
    let text = refused
        .as_ref()
        .map_or_else(ToString::to_string, |_| String::new());
    assert!(
        matches!(&refused, Err(HomeError::NotAbsolute { what: "HOME", .. })),
        "{text}"
    );
    assert!(
        text.contains("HOME") && text.contains("beginning with `~`"),
        "{text}"
    );
    let refused = ConfigDir::resolve(None, Some(Path::new("home/u")));
    assert!(matches!(
        &refused,
        Err(HomeError::NotAbsolute {
            what: "HOME",
            shape: "relative",
            ..
        })
    ));
    Ok(())
}

#[test]
fn home_dot_claude_claude_md_is_listed_once_as_the_user_file_when_home_is_the_config() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let h = base.join("h");
    let w = h.join("w");
    let o = base.join("o");
    put(&h.join(".claude").join("CLAUDE.md"), b"user and chain\n")?;
    put(&w.join("CLAUDE.md"), SENTENCE)?;
    let resolution = resolve_given(cwd(&w)?, ConfigDir::resolve(None, Some(h.as_path()))?, &o)?;
    assert_eq!(resolution.documents.len(), 2);
    assert_eq!(resolution.documents[0].kind, DocumentKind::UserClaudeMd);
    assert_eq!(
        resolution.documents[0].path,
        h.join(".claude").join("CLAUDE.md")
    );
    assert_eq!(resolution.documents[1].kind, DocumentKind::ClaudeMdChain);
    assert_eq!(resolution.documents[1].path, w.join("CLAUDE.md"));
    Ok(())
}

#[test]
fn home_dot_claude_claude_md_is_a_chain_position_when_the_template_names_another_config() -> Outcome
{
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let h = base.join("h");
    let w = h.join("w");
    let c = base.join("c");
    let o = base.join("o");
    std::fs::create_dir_all(&c)?;
    put(&h.join(".claude").join("CLAUDE.md"), b"chain only\n")?;
    put(&w.join("CLAUDE.md"), SENTENCE)?;
    let resolution = resolve_given(
        cwd(&w)?,
        ConfigDir::resolve(c.to_str(), Some(h.as_path()))?,
        &o,
    )?;
    assert_eq!(resolution.documents.len(), 2);
    assert_eq!(
        kinds(&resolution.documents),
        [DocumentKind::ClaudeMdChain, DocumentKind::ClaudeMdChain]
    );
    assert_eq!(
        paths(&resolution.documents),
        [
            h.join(".claude").join("CLAUDE.md").as_path(),
            w.join("CLAUDE.md").as_path()
        ]
    );
    Ok(())
}

#[test]
fn a_memory_index_under_the_slash_only_slug_of_a_dotted_directory_is_not_listed() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let w = base.join("dotted.dir").join("w");
    let c = base.join("c");
    let o = base.join("o");
    std::fs::create_dir_all(&w)?;
    let old_slug = cwd(&w)?.replace('/', "-");
    assert!(old_slug.contains('.'));
    put(
        &c.join("projects")
            .join(old_slug)
            .join("memory")
            .join("MEMORY.md"),
        b"never read\n",
    )?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 0);
    put(&memory_index(&c, cwd(&w)?), b"read\n")?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 1);
    assert_eq!(resolution.documents[0].kind, DocumentKind::MemoryIndex);
    Ok(())
}

#[cfg(unix)]
#[test]
fn an_unreadable_claude_md_fails_by_path_and_the_error_holds_no_line_of_it() -> Outcome {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let (w, c, o) = fixture(&base)?;
    let file = w.join("CLAUDE.md");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000))?;
    let refused = resolve_given(cwd(&w)?, template(&c), &o);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644))?;
    let Err(error) = refused else {
        return Err("an unreadable document is refused".into());
    };
    let text = error.to_string();
    assert!(
        matches!(&error, HomeError::Io { path, .. } if *path == file),
        "{text}"
    );
    assert!(text.contains(cwd(&file)?), "{text}");
    assert!(text.contains("reading a given document"), "{text}");
    assert!(
        !text.contains(std::str::from_utf8(SENTENCE)?.trim()),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_relative_working_directory_is_refused_by_name() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let (_, c, o) = fixture(&base)?;
    let refused = resolve_given("relative/w", template(&c), &o);
    assert!(matches!(
        refused,
        Err(HomeError::NotAbsolute {
            what: "working directory",
            ..
        })
    ));
    Ok(())
}

#[test]
fn resolution_writes_nothing_under_the_config_or_working_directory() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let (w, c, o) = fixture(&base)?;
    put(&c.join("CLAUDE.md"), b"user instructions\n")?;
    let before = (listing(&c)?, listing(&w)?);
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 5);
    assert_eq!(before, (listing(&c)?, listing(&w)?));
    Ok(())
}

/// The canonical base `B` with `B/real/w` and `B/real/c` directories, an out
/// directory `B/o` holding the two written files, and `B/link` a symlink to
/// `B/real` made here. Returns `B/real`.
#[cfg(unix)]
fn linked(base: &Path) -> std::io::Result<PathBuf> {
    let real = base.join("real");
    let link = base.join("link");
    std::fs::create_dir_all(real.join("w"))?;
    std::fs::create_dir_all(real.join("c"))?;
    put(&base.join("o").join("instructions.md"), INSTRUCTIONS)?;
    put(&base.join("o").join("mcp.json"), MCP)?;
    std::os::unix::fs::symlink(&real, &link)?;
    Ok(real)
}

/// The documents whose path is absolute, in record order.
#[cfg(unix)]
fn absolutes(documents: &[GivenDocument]) -> Vec<&GivenDocument> {
    documents.iter().filter(|d| d.path.is_absolute()).collect()
}

#[cfg(unix)]
#[test]
fn a_working_directory_through_a_symlink_records_the_chain_at_its_canonical_path() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let real = linked(&base)?;
    let link = base.join("link");
    put(&real.join("w").join("CLAUDE.md"), SENTENCE)?;
    let resolution = resolve_given(
        cwd(&link.join("w"))?,
        template(&real.join("c")),
        &base.join("o"),
    )?;
    let chain: Vec<&GivenDocument> = resolution
        .documents
        .iter()
        .filter(|d| d.kind == DocumentKind::ClaudeMdChain)
        .collect();
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].path, real.join("w").join("CLAUDE.md"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_working_directory_with_dot_dot_walks_only_the_canonical_chain() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let real = linked(&base)?;
    put(&real.join("CLAUDE.md"), b"parent\n")?;
    put(&real.join("w").join("CLAUDE.md"), SENTENCE)?;
    let dotted = real.join("w").join("..").join("w");
    let resolution = resolve_given(cwd(&dotted)?, template(&real.join("c")), &base.join("o"))?;
    let chain: Vec<&Path> = resolution
        .documents
        .iter()
        .filter(|d| d.kind == DocumentKind::ClaudeMdChain && d.path.starts_with(&base))
        .map(|d| d.path.as_path())
        .collect();
    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain,
        [
            real.join("CLAUDE.md").as_path(),
            real.join("w").join("CLAUDE.md").as_path()
        ]
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_config_directory_with_a_trailing_slash_records_without_it() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let real = linked(&base)?;
    put(&real.join("c").join("CLAUDE.md"), b"user\n")?;
    let slashed = format!("{}/", cwd(&real.join("c"))?);
    let resolution = resolve_given(
        cwd(&real.join("w"))?,
        template(Path::new(&slashed)),
        &base.join("o"),
    )?;
    assert_eq!(
        serde_json::to_string(&resolution.config_dir.path)?,
        serde_json::to_string(cwd(&real.join("c"))?)?
    );
    let users: Vec<&GivenDocument> = resolution
        .documents
        .iter()
        .filter(|d| d.kind == DocumentKind::UserClaudeMd)
        .collect();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].path, real.join("c").join("CLAUDE.md"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_config_through_a_symlink_is_still_listed_once_as_the_user_file() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let real = linked(&base)?;
    let link = base.join("link");
    put(&real.join(".claude").join("CLAUDE.md"), b"user and chain\n")?;
    put(&real.join("w").join("CLAUDE.md"), SENTENCE)?;
    let resolution = resolve_given(
        cwd(&real.join("w"))?,
        template(&link.join(".claude")),
        &base.join("o"),
    )?;
    let documents = absolutes(&resolution.documents);
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0].kind, DocumentKind::UserClaudeMd);
    assert_eq!(documents[0].path, real.join(".claude").join("CLAUDE.md"));
    assert_eq!(documents[1].kind, DocumentKind::ClaudeMdChain);
    assert_eq!(documents[1].path, real.join("w").join("CLAUDE.md"));
    let user = real.join(".claude").join("CLAUDE.md");
    let doubled = documents
        .iter()
        .filter(|d| d.kind == DocumentKind::ClaudeMdChain && d.path == user)
        .count();
    assert_eq!(doubled, 0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_config_directory_that_does_not_exist_is_recorded_as_given() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let real = linked(&base)?;
    let gone = format!("{}/", cwd(&base.join("gone").join("c"))?);
    let resolution = resolve_given(
        cwd(&real.join("w"))?,
        template(Path::new(&gone)),
        &base.join("o"),
    )?;
    assert_eq!(
        serde_json::to_string(&resolution.config_dir.path)?,
        serde_json::to_string(&gone)?
    );
    let config_kinds = [DocumentKind::UserClaudeMd, DocumentKind::MemoryIndex];
    let under_config = resolution
        .documents
        .iter()
        .filter(|d| config_kinds.contains(&d.kind))
        .count();
    assert_eq!(under_config, 0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn the_memory_index_slug_is_taken_from_the_working_directory_as_given() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let real = linked(&base)?;
    let link = base.join("link");
    let c = real.join("c");
    let given = link.join("w");
    let canonical_only = memory_index(&c, cwd(&real.join("w"))?);
    put(&canonical_only, b"canonical slug\n")?;
    let resolution = resolve_given(cwd(&given)?, template(&c), &base.join("o"))?;
    let unlisted = resolution
        .documents
        .iter()
        .filter(|d| d.kind == DocumentKind::MemoryIndex)
        .count();
    assert_eq!(unlisted, 0);
    let by_given = memory_index(&c, cwd(&given)?);
    put(&by_given, b"given slug\n")?;
    let resolution = resolve_given(cwd(&given)?, template(&c), &base.join("o"))?;
    let memory: Vec<&GivenDocument> = resolution
        .documents
        .iter()
        .filter(|d| d.kind == DocumentKind::MemoryIndex)
        .collect();
    assert_eq!(memory.len(), 1);
    assert_eq!(memory[0].path, by_given);
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_symlinked_claude_md_is_named_at_its_link_and_written_files_stay_relative() -> Outcome {
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let real = linked(&base)?;
    put(&real.join("t").join("target.md"), SENTENCE)?;
    std::os::unix::fs::symlink(
        real.join("t").join("target.md"),
        real.join("w").join("CLAUDE.md"),
    )?;
    let resolution = resolve_given(
        cwd(&real.join("w"))?,
        template(&real.join("c")),
        &base.join("o"),
    )?;
    assert_eq!(resolution.documents.len(), 3);
    assert_eq!(
        paths(&resolution.documents),
        [
            Path::new("instructions.md"),
            Path::new("mcp.json"),
            real.join("w").join("CLAUDE.md").as_path()
        ]
    );
    assert_eq!(resolution.documents[2].sha256, sha256_hex(SENTENCE));
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_working_directory_that_cannot_be_searched_is_refused_by_operation_and_path() -> Outcome {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let base = dir.path().canonicalize()?;
    let real = linked(&base)?;
    let locked = base.join("locked");
    let w = locked.join("w");
    put(&w.join("CLAUDE.md"), SENTENCE)?;
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))?;
    let refused = resolve_given(cwd(&w)?, template(&real.join("c")), &base.join("o"));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))?;
    let Err(error) = refused else {
        return Err("a working directory that cannot be searched is refused".into());
    };
    let text = error.to_string();
    assert!(
        matches!(&error, HomeError::Io { path, .. } if *path == w),
        "{text}"
    );
    assert!(text.contains("canonicalising a given path"), "{text}");
    assert!(text.contains(cwd(&w)?), "{text}");
    assert!(
        !text.contains(std::str::from_utf8(SENTENCE)?.trim()),
        "{text}"
    );
    Ok(())
}
