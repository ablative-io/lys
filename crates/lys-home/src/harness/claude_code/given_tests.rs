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
            let _ = write!(s, "{b:02x}");
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
    let (w, c, o) = fixture(dir.path())?;
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
fn a_config_claude_md_is_listed_first_as_user_claude_md() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (w, c, o) = fixture(dir.path())?;
    put(&c.join("CLAUDE.md"), b"user instructions\n")?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 5);
    assert_eq!(resolution.documents[0].kind, DocumentKind::UserClaudeMd);
    assert_eq!(resolution.documents[0].path, c.join("CLAUDE.md"));
    assert_eq!(
        resolution.documents[1].kind,
        DocumentKind::AppendedInstructions
    );
    Ok(())
}

#[test]
fn an_ancestor_claude_md_is_listed_before_the_working_directory_s() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (w, c, o) = fixture(dir.path())?;
    put(&dir.path().join("CLAUDE.md"), b"ancestor instructions\n")?;
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 5);
    assert_eq!(resolution.documents[2].kind, DocumentKind::ClaudeMdChain);
    assert_eq!(resolution.documents[2].path, dir.path().join("CLAUDE.md"));
    assert_eq!(resolution.documents[3].kind, DocumentKind::ClaudeMdChain);
    assert_eq!(resolution.documents[3].path, w.join("CLAUDE.md"));
    Ok(())
}

#[test]
fn one_directory_lists_claude_md_then_dot_claude_then_local_whatever_the_creation_order() -> Outcome
{
    let dir = tempfile::tempdir()?;
    let w = dir.path().join("w");
    let c = dir.path().join("c");
    let o = dir.path().join("o");
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
    let (w, c, o) = fixture(dir.path())?;
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
    let (w, c, o) = fixture(dir.path())?;
    let h = dir.path().join("h");
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
fn home_dot_claude_claude_md_is_listed_once_first_as_the_user_file_when_home_is_the_config()
-> Outcome {
    let dir = tempfile::tempdir()?;
    let h = dir.path().join("h");
    let w = h.join("w");
    let o = dir.path().join("o");
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
    let h = dir.path().join("h");
    let w = h.join("w");
    let c = dir.path().join("c");
    let o = dir.path().join("o");
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
    let w = dir.path().join("dotted.dir").join("w");
    let c = dir.path().join("c");
    let o = dir.path().join("o");
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
    let (w, c, o) = fixture(dir.path())?;
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
    let (_, c, o) = fixture(dir.path())?;
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
    let (w, c, o) = fixture(dir.path())?;
    put(&c.join("CLAUDE.md"), b"user instructions\n")?;
    let before = (listing(&c)?, listing(&w)?);
    let resolution = resolve_given(cwd(&w)?, template(&c), &o)?;
    assert_eq!(resolution.documents.len(), 5);
    assert_eq!(before, (listing(&c)?, listing(&w)?));
    Ok(())
}
