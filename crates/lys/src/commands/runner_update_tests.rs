#![cfg(test)]

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const OLD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NEW: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Fixture {
    root: tempfile::TempDir,
    layout: Layout,
    source: PathBuf,
    release: ClientRelease,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let layout = Layout::at(root.path().join("install"));
        let source = root.path().join("incoming");
        for directory in [
            layout.root.clone(),
            layout.run_dir(),
            layout.bin_dir(),
            source.clone(),
        ] {
            private_files::ensure_dir(&directory)?;
        }
        let mut files = BTreeMap::new();
        for name in NAMES {
            std::fs::write(layout.binary(name), format!("old {name}"))?;
            std::fs::write(source.join(name), format!("new {name}"))?;
            files.insert(name.to_owned(), digest(&source.join(name))?);
        }
        Ok(Self {
            root,
            layout,
            source,
            release: ClientRelease {
                commit: NEW.to_owned(),
                files,
            },
        })
    }

    fn stage(&self) -> CliResult<()> {
        stage(&self.layout, &self.release, &self.source)
    }
}

#[test]
fn digest_mismatch_refuses_before_either_executable_or_backup_changes() -> TestResult {
    let fixture = Fixture::new()?;
    std::fs::write(fixture.source.join("lys"), b"different release")?;
    let error = fixture.stage().expect_err("digest mismatch accepted");
    assert!(error.to_string().contains("runner_update_digest_mismatch"));
    for name in NAMES {
        assert_eq!(
            std::fs::read_to_string(fixture.layout.binary(name))?,
            format!("old {name}")
        );
        assert!(
            !fixture
                .layout
                .bin_dir()
                .join(format!("{name}.previous"))
                .exists()
        );
    }
    assert!(!record_path(&fixture.layout).exists());
    Ok(())
}

#[test]
fn helper_digest_mismatch_also_keeps_both_installed_executables() -> TestResult {
    let fixture = Fixture::new()?;
    std::fs::write(
        fixture.source.join("lys-containment-entry"),
        b"altered helper",
    )?;
    assert!(
        fixture
            .stage()
            .expect_err("altered helper accepted")
            .to_string()
            .contains("runner_update_digest_mismatch")
    );
    assert_eq!(std::fs::read(fixture.layout.binary("lys"))?, b"old lys");
    assert!(!record_path(&fixture.layout).exists());
    Ok(())
}

#[test]
fn unready_release_restores_the_pair_and_retains_its_rejection() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.stage()?;
    for name in NAMES {
        assert_eq!(
            std::fs::read_to_string(fixture.layout.binary(name))?,
            format!("new {name}")
        );
        assert_eq!(
            std::fs::read_to_string(fixture.layout.bin_dir().join(format!("{name}.previous")))?,
            format!("old {name}")
        );
    }
    assert!(rollback(&fixture.layout)?);
    for name in NAMES {
        assert_eq!(
            std::fs::read_to_string(fixture.layout.binary(name))?,
            format!("old {name}")
        );
    }
    let record = read(&fixture.layout)?.ok_or("rollback lost its rejection")?;
    assert!(record.rejected);
    assert_eq!(record.commit, NEW);
    assert!(!rollback(&fixture.layout)?);
    assert!(fixture.root.path().is_dir());
    Ok(())
}

#[test]
fn interrupted_pair_swap_is_restored_before_starting() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.stage()?;
    std::fs::write(
        fixture.layout.binary("lys-containment-entry"),
        b"old lys-containment-entry",
    )?;
    assert!(recover(&fixture.layout)?);
    for name in NAMES {
        assert_eq!(
            std::fs::read_to_string(fixture.layout.binary(name))?,
            format!("old {name}")
        );
    }
    assert!(
        read(&fixture.layout)?
            .ok_or("recovery lost its rejection")?
            .rejected
    );
    Ok(())
}

#[test]
fn corrupt_previous_helper_refuses_before_restoring_either_executable() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.stage()?;
    std::fs::write(
        fixture
            .layout
            .bin_dir()
            .join("lys-containment-entry.previous"),
        b"corrupt backup",
    )?;
    let error = rollback(&fixture.layout).expect_err("corrupt backup accepted");
    assert!(error.to_string().contains("runner_update_digest_mismatch"));
    for name in NAMES {
        assert_eq!(
            std::fs::read_to_string(fixture.layout.binary(name))?,
            format!("new {name}")
        );
    }
    assert!(
        !read(&fixture.layout)?
            .ok_or("update record missing")?
            .rejected
    );
    Ok(())
}

#[test]
fn readiness_from_another_commit_does_not_accept_the_update() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.stage()?;
    let error = ready(&fixture.layout, OLD).expect_err("another commit accepted");
    assert!(error.to_string().contains("runner_update_ready_mismatch"));
    assert!(record_path(&fixture.layout).is_file());
    assert!(rollback(&fixture.layout)?);
    Ok(())
}

#[test]
fn verified_readiness_promotes_the_controller_and_keeps_previous_binaries() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.stage()?;
    ready(&fixture.layout, &format!("{NEW}; stated"))?;
    assert!(!record_path(&fixture.layout).exists());
    assert_eq!(
        std::fs::read(fixture.layout.bin_dir().join("runner-service/lys"))?,
        b"new lys"
    );
    for name in NAMES {
        assert_eq!(
            std::fs::read_to_string(fixture.layout.bin_dir().join(format!("{name}.previous")))?,
            format!("old {name}")
        );
    }
    assert!(!rollback(&fixture.layout)?);
    Ok(())
}

#[test]
fn a_ready_line_cannot_accept_changed_executable_bytes() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.stage()?;
    std::fs::write(fixture.layout.binary("lys"), b"changed after verification")?;
    assert!(
        ready(&fixture.layout, NEW)
            .expect_err("changed runner accepted")
            .to_string()
            .contains("runner_update_ready_mismatch")
    );
    assert!(rollback(&fixture.layout)?);
    Ok(())
}

#[test]
fn client_listing_requires_this_platform_and_both_sha256_digests() -> TestResult {
    let fixture = Fixture::new()?;
    let platform = "aarch64-apple-darwin";
    let bytes = serde_json::to_vec(&serde_json::json!({"platforms": {platform: fixture.release}}))?;
    assert_eq!(release(&bytes, platform)?.commit, NEW);
    assert!(
        release(&bytes, "x86_64-unknown-linux-gnu")
            .expect_err("missing platform accepted")
            .to_string()
            .contains("runner_update_platform_missing")
    );
    let malformed = serde_json::to_vec(
        &serde_json::json!({"platforms": {platform: {"commit": NEW, "files": {"lys": "0".repeat(64)}}}}),
    )?;
    assert!(
        release(&malformed, platform)
            .expect_err("missing helper accepted")
            .to_string()
            .contains("runner_update_manifest_invalid")
    );
    Ok(())
}

#[test]
fn dirty_or_untracked_builds_cannot_claim_the_current_release() {
    assert_eq!(stamp_commit(OLD).expect("clean commit refused"), OLD);
    assert_eq!(
        stamp_commit(&format!("{OLD}; stated")).expect("stated commit refused"),
        OLD
    );
    for stamp in [
        format!("{OLD}; dirty"),
        "not built from a git commit".to_owned(),
        String::new(),
    ] {
        assert!(stamp_commit(&stamp).is_err());
    }
}

#[test]
fn unreachable_update_checks_return_ok_and_leave_the_installed_pair_untouched() -> TestResult {
    for code in [
        Some(6),
        Some(7),
        Some(22),
        Some(28),
        Some(35),
        Some(60),
        None,
    ] {
        let fixture = Fixture::new()?;
        SavedJoin {
            server: "https://lys.example.test/api".to_owned(),
            machine: "op-example".to_owned(),
            server_ca: None,
        }
        .save(&fixture.layout)?;
        let mut requests = 0;
        let updated = prepare_with(&fixture.layout, &mut |_, route, _, _| {
            assert_eq!(route, "clients");
            requests += 1;
            Err(download_failure(code))
        })?;
        assert!(!updated);
        assert_eq!(requests, 1);
        for name in NAMES {
            assert_eq!(
                std::fs::read_to_string(fixture.layout.binary(name))?,
                format!("old {name}")
            );
            assert!(
                !fixture
                    .layout
                    .bin_dir()
                    .join(format!("{name}.previous"))
                    .exists()
            );
        }
        assert!(!record_path(&fixture.layout).exists());
        assert!(!fixture.layout.bin_dir().join("runner-next").exists());
    }
    Ok(())
}

#[test]
fn an_oversized_download_remains_an_error_without_replacing_the_pair() -> TestResult {
    let fixture = Fixture::new()?;
    SavedJoin {
        server: "https://lys.example.test/api".to_owned(),
        machine: "op-example".to_owned(),
        server_ca: None,
    }
    .save(&fixture.layout)?;
    let error = prepare_with(&fixture.layout, &mut |_, _, _, _| {
        Err(download_failure(Some(63)))
    })
    .expect_err("oversized release accepted");
    assert!(error.to_string().contains("runner_update_download_invalid"));
    for name in NAMES {
        assert_eq!(
            std::fs::read_to_string(fixture.layout.binary(name))?,
            format!("old {name}")
        );
    }
    Ok(())
}
