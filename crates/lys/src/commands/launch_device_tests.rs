use std::path::Path;

use serde_json::json;

use super::{public_key, select_with};
use crate::identity::private_files;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn write_key(path: &Path, seed: u8) -> Result {
    private_files::ensure_dir(path.parent().ok_or("key has no parent")?)?;
    private_files::write(path, &[seed; 32])?;
    Ok(())
}

#[test]
fn an_unjoined_device_does_not_mint_a_key_or_pick_a_machine() -> Result {
    let root = tempfile::tempdir()?;
    let error = public_key(root.path())
        .err()
        .ok_or("unjoined device accepted")?;
    assert!(error.to_string().contains("machine_not_joined"));
    assert!(!root.path().join("keys").exists());
    Ok(())
}

#[test]
fn a_previously_joined_device_resolves_its_old_key_without_rotation() -> Result {
    let root = tempfile::tempdir()?;
    let layout = crate::identity::install::layout::Layout {
        root: root.path().to_owned(),
    };
    let legacy = layout
        .data_dir()
        .join(crate::commands::runner_join::MACHINE_KEY);
    write_key(&legacy, 7)?;
    let key = public_key(root.path())?;
    assert_eq!(key.len(), 64);
    assert!(legacy.exists());
    assert!(!layout.runner_machine_key().exists());
    Ok(())
}

#[test]
fn conflicting_installed_and_old_machine_keys_refuse_without_repair() -> Result {
    let root = tempfile::tempdir()?;
    let layout = crate::identity::install::layout::Layout {
        root: root.path().to_owned(),
    };
    write_key(&layout.runner_machine_key(), 3)?;
    let legacy = layout
        .data_dir()
        .join(crate::commands::runner_join::MACHINE_KEY);
    write_key(&legacy, 4)?;
    let error = public_key(root.path())
        .err()
        .ok_or("conflicting keys accepted")?;
    assert!(error.to_string().contains("runner_machine_key_conflict"));
    assert_eq!(std::fs::read(layout.runner_machine_key())?, [3; 32]);
    assert_eq!(std::fs::read(legacy)?, [4; 32]);
    Ok(())
}

#[test]
fn machine_selection_matches_the_device_key_instead_of_the_first_machine() -> Result {
    let selected = select_with("this-key", |path| {
        Ok(match path {
            "/network" => {
                json!({"machines":[{"id":"other","state":"in_use"},{"id":"here","state":"in_use"}]})
            }
            "/network/machines/other/runner" => {
                json!({"machine":"other","runner":{"kind":"dialled","key":"other-key"},"answers":true})
            }
            "/network/machines/here/runner" => {
                json!({"machine":"here","runner":{"kind":"dialled","key":"this-key"},"answers":true})
            }
            _ => return Err(super::refused("fixture_route_unknown", path)),
        })
    })?;
    assert_eq!(selected, "here");
    Ok(())
}

#[test]
fn duplicate_server_machines_for_one_device_refuse_instead_of_selecting_one() -> Result {
    let error = select_with("this-key", |path| {
        Ok(match path {
            "/network" => {
                json!({"machines":[{"id":"one","state":"in_use"},{"id":"two","state":"in_use"}]})
            }
            "/network/machines/one/runner" => {
                json!({"machine":"one","runner":{"kind":"dialled","key":"this-key"},"answers":true})
            }
            "/network/machines/two/runner" => {
                json!({"machine":"two","runner":{"kind":"dialled","key":"this-key"},"answers":true})
            }
            _ => return Err(super::refused("fixture_route_unknown", path)),
        })
    })
    .err()
    .ok_or("ambiguous device accepted")?;
    assert!(error.to_string().contains("machine_identity_ambiguous"));
    Ok(())
}

#[test]
fn an_offline_joined_runner_retains_its_named_server_reason() -> Result {
    let error = select_with("this-key", |path| Ok(match path {
        "/network" => json!({"machines":[{"id":"here","state":"in_use"}]}),
        "/network/machines/here/runner" => json!({"machine":"here","runner":{"kind":"dialled","key":"this-key"},"answers":false,"refusal":"runner_not_dialled_in","reason":"this computer is disconnected"}),
        _ => return Err(super::refused("fixture_route_unknown", path)),
    })).err().ok_or("offline runner accepted")?.to_string();
    assert!(error.contains("runner_not_dialled_in"));
    assert!(error.contains("this computer is disconnected"));
    Ok(())
}

#[test]
fn an_incomplete_network_answer_refuses_without_echoing_response_values() -> Result {
    let error = select_with("this-key", |_| Ok(json!({"unexpected":"fixture-secret"})))
        .err()
        .ok_or("invalid answer accepted")?
        .to_string();
    assert!(error.contains("machine_answer_unreadable"));
    assert!(!error.contains("fixture-secret"));
    Ok(())
}

#[test]
fn agents_can_identify_a_disconnected_joined_machine_without_claiming_it_is_online() -> Result {
    let selected = super::select(
        "this-key",
        |path| {
            Ok(match path {
                "/network" => json!({"machines":[{"id":"here","state":"in_use"}]}),
                "/network/machines/here/runner" => {
                    json!({"machine":"here","runner":{"kind":"dialled","key":"this-key"},"answers":false,"refusal":"runner_not_dialled_in","reason":"disconnected"})
                }
                _ => return Err(super::refused("fixture_route_unknown", path)),
            })
        },
        false,
    )?;
    assert_eq!(selected, "here");
    Ok(())
}

#[test]
fn a_retired_previous_record_does_not_hide_the_devices_one_active_join() -> Result {
    let selected = select_with("this-key", |path| {
        Ok(match path {
            "/network" => {
                json!({"machines":[{"id":"old","state":"retired"},{"id":"here","state":"in_use"}]})
            }
            "/network/machines/old/runner" => {
                json!({"machine":"old","runner":{"kind":"dialled","key":"this-key"},"answers":false,"refusal":"runner_not_dialled_in","reason":"old record"})
            }
            "/network/machines/here/runner" => {
                json!({"machine":"here","runner":{"kind":"dialled","key":"this-key"},"answers":true})
            }
            _ => return Err(super::refused("fixture_route_unknown", path)),
        })
    })?;
    assert_eq!(selected, "here");
    Ok(())
}

#[test]
fn only_retired_key_matches_name_the_retirement_instead_of_inventing_a_join() -> Result {
    let error = select_with("this-key", |path| Ok(match path {
        "/network" => json!({"machines":[{"id":"old","state":"retired"}]}),
        "/network/machines/old/runner" => json!({"machine":"old","runner":{"kind":"dialled","key":"this-key"},"answers":false,"refusal":"runner_not_dialled_in","reason":"old record"}),
        _ => return Err(super::refused("fixture_route_unknown", path)),
    })).err().ok_or("retired join accepted")?;
    assert!(error.to_string().contains("MachineRetired"));
    Ok(())
}

#[test]
fn a_runner_view_carrying_lost_at_makes_machine_not_joined_end_with_the_computer_loss_sentence()
-> Result {
    let error = select_with("this-key", |path| Ok(match path {
        "/network" => json!({"machines":[{"id":"other","state":"in_use"}]}),
        "/network/machines/other/runner" => json!({"machine":"other","runner":null,"answers":null,"lost_at":"2026-10-19T02:00:00.000000Z"}),
        _ => return Err(super::refused("fixture_route_unknown", path)),
    })).err().ok_or("a lost join accepted")?.to_string();
    assert!(error.contains("machine_not_joined"));
    assert!(error.ends_with(
        "Lys lost its computer records at 2026-10-19T02:00:00.000000Z; connect this computer again"
    ), "{error}");
    let without = select_with("this-key", |path| {
        Ok(match path {
            "/network" => json!({"machines":[{"id":"other","state":"in_use"}]}),
            "/network/machines/other/runner" => {
                json!({"machine":"other","runner":null,"answers":null})
            }
            _ => return Err(super::refused("fixture_route_unknown", path)),
        })
    })
    .err()
    .ok_or("an unjoined device accepted")?
    .to_string();
    assert!(without.contains("machine_not_joined"));
    assert!(!without.contains("Lys lost"), "{without}");
    Ok(())
}
