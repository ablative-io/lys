#![cfg(test)]

use std::path::Path;

use lys_install::install::layout::Layout;

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn an_app_on_the_disk_image_or_moved_aside_is_refused_saying_to_move_it() {
    let refused = [
        "/Volumes/Lys/Lys.app/Contents/MacOS",
        "/private/var/folders/ab/cd/T/AppTranslocation/0A1B/d/Lys.app/Contents/MacOS",
    ];
    for path in refused {
        let bundle = Bundle {
            binaries: PathBuf::from(path),
        };
        assert!(is_translocated(Path::new(path)), "{path}");
        let refusal = bundle.require_placed().err();
        assert_eq!(
            refusal.as_ref().map(|refusal| refusal.name),
            Some("app_not_in_applications")
        );
        assert!(refusal.is_some_and(|refusal| refusal.next.contains("drag it to Applications")));
    }
    let placed = [
        "/Applications/Lys.app/Contents/MacOS",
        "/Users/someone/Applications/Lys.app/Contents/MacOS",
    ];
    for path in placed {
        assert!(!is_translocated(Path::new(path)), "{path}");
    }
}

#[test]
fn the_screens_are_in_the_bundle_resources() {
    let bundle = Bundle {
        binaries: PathBuf::from("/Applications/Lys.app/Contents/MacOS"),
    };
    assert_eq!(
        bundle.surface(),
        Path::new("/Applications/Lys.app/Contents/Resources/surface")
    );
    assert_eq!(
        bundle.binary(SERVICE),
        Path::new("/Applications/Lys.app/Contents/MacOS/lys-identity-server")
    );
}

#[test]
fn the_build_record_names_the_service_commit() -> TestResult {
    let record = r#"{"binaries":{"lys-identity-server":"abc","lys-secrets":"abc"},"surface":null}"#;
    assert_eq!(recorded_build(record)?, Some("abc".to_string()));
    assert_eq!(recorded_build(r#"{"binaries":{}}"#)?, None);
    assert!(recorded_build("not json").is_err());
    Ok(())
}

#[test]
fn an_install_is_its_configuration_and_its_service_in_place() -> TestResult {
    let root = tempfile::tempdir()?;
    let layout = Layout::at(root.path().to_path_buf());
    assert_eq!(installed(&layout)?, Installed::Nothing);
    std::fs::write(layout.deployment_config(), "")?;
    std::fs::write(layout.service_config(), "")?;
    assert_eq!(installed(&layout)?, Installed::Nothing);
    std::fs::create_dir_all(layout.bin_dir())?;
    std::fs::write(layout.binary(SERVICE), "")?;
    assert_eq!(installed(&layout)?, Installed::Build(None));
    std::fs::create_dir_all(layout.install_dir())?;
    std::fs::write(
        layout.build_record(),
        r#"{"binaries":{"lys-identity-server":"0123"}}"#,
    )?;
    assert_eq!(
        installed(&layout)?,
        Installed::Build(Some("0123".to_string()))
    );
    std::fs::write(layout.build_record(), "{")?;
    let unreadable = installed(&layout).err().map(|refusal| refusal.name);
    assert_eq!(unreadable, Some("build_record_unreadable"));
    Ok(())
}
