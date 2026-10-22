#![cfg(test)]

use std::collections::BTreeMap;

use super::*;
use crate::identity::upgrade::intent::Intent;
use crate::identity::upgrade::scratch::Recorder;
use crate::identity::upgrade::swap;

#[test]
fn upgrade_places_the_helper_beside_lys_and_rollback_restores_the_pair()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().join("install"));
    let from = root.path().join("incoming");
    for directory in [layout.bin_dir(), from.clone()] {
        std::fs::create_dir_all(directory)?;
    }
    for name in ["lys", NAME] {
        std::fs::write(layout.binary(name), format!("old {name}"))?;
        std::fs::write(from.join(name), format!("new {name}"))?;
    }
    let mut intent = Intent {
        from: BTreeMap::new(),
        to: BTreeMap::new(),
        screens: false,
        screens_existed: false,
        files: Vec::new(),
        compose_changed: false,
        steps: Vec::new(),
        organisation_copy: None,
        clients_placed: crate::identity::install::layout::PlacedClients::default(),
    };
    check(&layout, &from)?;
    swap::keep_binaries(&layout, &mut intent)?;
    swap::place_binaries(&layout, &from, &["lys"], &mut intent)?;
    assert_eq!(
        std::fs::read_to_string(layout.binary(NAME))?,
        "new lys-containment-entry"
    );
    assert_eq!(std::fs::read_to_string(layout.binary("lys"))?, "new lys");
    assert_eq!(
        std::fs::read_to_string(layout.bin_previous_dir().join(NAME))?,
        "old lys-containment-entry"
    );
    swap::back(
        &layout,
        &intent,
        &[],
        &mut Recorder::default(),
        None,
        &mut |_| {},
    )?;
    assert_eq!(
        std::fs::read_to_string(layout.binary(NAME))?,
        "old lys-containment-entry"
    );
    assert_eq!(std::fs::read_to_string(layout.binary("lys"))?, "old lys");
    Ok(())
}

#[test]
fn missing_incoming_helper_refuses_before_an_existing_install_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    std::fs::create_dir(layout.bin_dir())?;
    std::fs::write(layout.binary(NAME), b"held helper")?;
    let error = check(&layout, &root.path().join("missing")).expect_err("missing helper accepted");
    assert!(error.to_string().contains("runner_helper_missing"));
    assert_eq!(std::fs::read(layout.binary(NAME))?, b"held helper");
    Ok(())
}
