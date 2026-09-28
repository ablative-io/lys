#![cfg(test)]

use std::path::Path;

use lys_install::install::layout::Layout;

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_agent_runs_the_installed_app_at_login_and_leaves_lys_running() {
    let text = definition(
        Path::new("/Users/someone/Library/Application Support/lys/identity/bin/lys-app"),
        Path::new("/Users/someone/Library/Application Support/lys/identity/logs/lys-app.log"),
    );
    assert!(text.contains(&format!("<string>{LABEL}</string>")));
    assert!(text.contains(
        "<string>/Users/someone/Library/Application Support/lys/identity/bin/lys-app</string>\n\t\t<string>--at-login</string>"
    ));
    assert!(text.contains("<key>RunAtLoad</key>\n\t<true/>"));
    assert!(text.contains("<key>AbandonProcessGroup</key>\n\t<true/>"));
    assert!(text.contains("logs/lys-app.log</string>"));
    assert!(!text.contains("KeepAlive"), "{text}");
}

#[test]
fn a_path_is_escaped_for_the_property_list() {
    let text = definition(Path::new("/Users/a&b/<lys>"), Path::new("/log"));
    assert!(text.contains("/Users/a&amp;b/&lt;lys&gt;"), "{text}");
}

#[test]
fn the_agent_is_in_the_persons_launch_agents() {
    assert_eq!(
        agent_path(Path::new("/Users/someone")),
        Path::new("/Users/someone/Library/LaunchAgents/au.com.ablative.lys.plist")
    );
}

/// Registering names the app the install keeps in its own `bin/`, never the
/// app where it was opened, and writes nothing a second time.
#[test]
fn registering_writes_the_agent_once_naming_the_installed_app() -> TestResult {
    let home = tempfile::tempdir()?;
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    assert!(register_at(home.path(), &layout)?);
    let path = agent_path(home.path());
    let text = std::fs::read_to_string(&path)?;
    let program = layout.binary("lys-app").display().to_string();
    assert!(
        text.contains(&format!("<string>{program}</string>")),
        "{text}"
    );
    assert!(!register_at(home.path(), &layout)?);
    assert_eq!(std::fs::read_to_string(&path)?, text);
    let leftovers: Vec<_> = std::fs::read_dir(path.parent().ok_or("no folder")?)?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(leftovers.len(), 1, "{leftovers:?}");
    Ok(())
}
