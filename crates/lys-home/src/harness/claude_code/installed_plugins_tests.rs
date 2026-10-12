//! A run's named plugins are checked against the config directory it uses.

use super::installed_plugins::{INSTALLED_FILE, require};
use crate::error::HomeError;

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

fn installed_in(dir: &std::path::Path, body: &str) {
    let file = dir.join(INSTALLED_FILE);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, body).unwrap();
}

#[test]
fn plugins_the_config_directory_has_installed_are_accepted() {
    let dir = tempfile::tempdir().unwrap();
    installed_in(
        dir.path(),
        r#"{"version":2,"plugins":{"argus@ablative":[{}],"frontend-design@claude-plugins-official":[{}]}}"#,
    );
    require(
        dir.path(),
        &names(&["argus@ablative", "frontend-design@claude-plugins-official"]),
    )
    .unwrap();
}

#[test]
fn a_plugin_the_config_directory_lacks_is_refused_by_name_and_folder() {
    let dir = tempfile::tempdir().unwrap();
    installed_in(
        dir.path(),
        r#"{"version":2,"plugins":{"argus@ablative":[{}]}}"#,
    );
    let refused = require(dir.path(), &names(&["argus@ablative", "manifold@manifold"]));
    let Err(HomeError::PluginNotInstalled { name, config_dir }) = refused else {
        panic!("a missing plugin was not refused: {refused:?}");
    };
    assert_eq!(name, "manifold@manifold");
    assert_eq!(config_dir, dir.path());
    let words = HomeError::PluginNotInstalled { name, config_dir }.to_string();
    assert!(
        words.starts_with("plugin_not_installed: manifold@manifold"),
        "{words}"
    );
}

#[test]
fn a_config_directory_with_no_plugins_file_has_none_installed() {
    let dir = tempfile::tempdir().unwrap();
    require(dir.path(), &[]).unwrap();
    assert!(matches!(
        require(dir.path(), &names(&["argus@ablative"])),
        Err(HomeError::PluginNotInstalled { .. })
    ));
}
