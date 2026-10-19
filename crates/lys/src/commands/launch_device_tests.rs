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
    let device = super::Device {
        machine: Some("this-key".to_owned()),
        server: None,
    };
    let selected = super::select(
        &device,
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

/// The labels the surface renders, read from the one file that defines them.
const LABELS: &str = include_str!("../../../../surface/identity/src/features/network/labels.ts");

/// The string the surface exports as `name`, read from its `export const`.
fn exported(name: &str) -> Option<&'static str> {
    let from = LABELS.find(&format!("export const {name} = '"))? + name.len() + 17;
    let rest = &LABELS[from..];
    Some(&rest[..rest.find('\'')?])
}

/// The host runs no dialled runner: its machine names the install's own.
fn host_view(
    key: &str,
) -> impl FnMut(&str) -> crate::commands::error::CliResult<serde_json::Value> {
    let key = key.to_owned();
    move |path| {
        Ok(match path {
            "/network" => json!({"machines":[{"id":"host","state":"in_use"}]}),
            "/network/machines/host/runner" => {
                json!({"machine":"host","runner":{"kind":"lys"},"runner_key":key,"answers":true})
            }
            _ => return Err(super::refused("fixture_route_unknown", path)),
        })
    }
}

#[test]
fn the_computer_that_runs_lys_matches_its_machine_by_the_installs_runner_key() -> Result {
    let device = super::Device {
        machine: None,
        server: Some("install-key".to_owned()),
    };
    assert_eq!(
        super::select(&device, host_view("install-key"), true)?,
        "host"
    );
    let error = super::select(&device, host_view("another-install"), true)
        .err()
        .ok_or("another install's runner accepted")?;
    assert!(error.to_string().contains("machine_not_joined"), "{error}");
    let joined = super::Device {
        machine: Some("this-key".to_owned()),
        server: None,
    };
    let error = super::select(&joined, host_view("install-key"), true)
        .err()
        .ok_or("a device without the install's key accepted")?;
    assert!(error.to_string().contains("machine_not_joined"), "{error}");
    Ok(())
}

#[test]
fn the_install_key_is_read_only_on_the_computer_that_holds_the_install() -> Result {
    let root = tempfile::tempdir()?;
    let layout = crate::identity::install::layout::Layout {
        root: root.path().to_owned(),
    };
    // Nothing on the device: not joined, and nothing is made.
    let error = super::device(root.path())
        .err()
        .ok_or("an empty device accepted")?;
    assert!(error.to_string().contains("machine_not_joined"));
    std::fs::create_dir_all(layout.run_dir())?;
    std::fs::write(layout.run_dir().join("runner-server.pub"), "install-key\n")?;
    let host = super::device(root.path())?;
    assert_eq!(host.server.as_deref(), Some("install-key"));
    assert_eq!(host.machine, None);
    // A computer joined to a Lys keeps that server's key in the same file:
    // it is that server's device by its own key, never the install's.
    std::fs::write(layout.run_dir().join("runner-join.json"), "{}")?;
    let error = super::device(root.path())
        .err()
        .ok_or("a joined device without its own key accepted")?;
    assert!(error.to_string().contains("machine_not_joined"));
    Ok(())
}

#[test]
fn every_advice_names_only_labels_the_computers_page_renders() -> Result {
    let computers = exported("COMPUTERS").ok_or("labels.ts exports no COMPUTERS")?;
    let connect = exported("CONNECT").ok_or("labels.ts exports no CONNECT")?;
    assert_eq!(computers, super::COMPUTERS);
    assert_eq!(connect, super::CONNECT);
    assert!(
        LABELS
            .contains("export const connectLabel = (name: string): string => `${CONNECT}${name}`;")
    );
    for advice in [
        super::advice(None),
        super::advice(Some("https://lys.example/#/network")),
    ] {
        assert!(advice.contains(computers), "{advice}");
        assert!(advice.contains(&format!("{connect}<")), "{advice}");
        assert!(!advice.contains("Network >"), "{advice}");
        assert!(!advice.contains("> Connect"), "{advice}");
    }
    let unjoined = super::device(tempfile::tempdir()?.path())
        .err()
        .ok_or("an empty device accepted")?
        .to_string();
    assert!(unjoined.contains(&super::advice(None)), "{unjoined}");
    Ok(())
}
