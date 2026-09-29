#![cfg(test)]
//! Refused preflights leave the existing running installation untouched.

use std::collections::BTreeMap;
use std::path::Path;

use super::render::{Render, RenderedFile};
use super::scratch::{B, Behaviour, Fixed, Recorder, Scratch, TestResult, build};
use super::{Parts, upgrade};
use crate::identity::error::{ErrorKind, IdentityResult};
use crate::identity::install::layout::{BINARIES, Layout};

fn intercept(program: &Path, script: &str) -> TestResult {
    let original = std::fs::read_to_string(program)?;
    std::fs::write(
        program,
        original.replacen("#!/bin/sh\n", &format!("#!/bin/sh\n{script}\n"), 1),
    )?;
    Ok(())
}

fn refuses_untouched(script: &str, renderer: &dyn Render, stage: &str) -> TestResult {
    let scratch = Scratch::new()?;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    intercept(&new.join(BINARIES[1]), script)?;
    let before = scratch.configuration()?;
    let build_before = std::fs::read(scratch.layout.build_record())?;
    let running = scratch.running()?;
    let pids: Vec<_> = scratch
        .units
        .iter()
        .map(|unit| std::fs::read(&unit.pid))
        .collect::<Result<_, _>>()?;
    let mut engine = Recorder::default();
    let mut parts = Parts {
        runner: None,
        units: &scratch.units,
        engine: &mut engine,
        render: renderer,
    };
    let error = upgrade(&scratch.layout, &new, None, &mut parts, &mut |_| {})
        .err()
        .ok_or("preflight passed")?;
    assert_eq!(error.kind(), ErrorKind::ConfigInvalid);
    assert!(error.to_string().contains(stage), "{error}");
    assert!(!error.to_string().contains("private-marker"));
    assert_eq!(scratch.configuration()?, before);
    assert_eq!(std::fs::read(scratch.layout.build_record())?, build_before);
    assert_eq!(scratch.running()?, running);
    for (unit, pid) in scratch.units.iter().zip(pids) {
        assert_eq!(std::fs::read(&unit.pid)?, pid);
    }
    assert!(engine.applied.is_empty());
    assert!(!scratch.layout.upgrade_intent().exists());
    assert!(!scratch.layout.bin_previous_dir().exists());
    Ok(())
}

struct NeverRender;
impl Render for NeverRender {
    fn render(
        &self,
        _: &Layout,
        _: &BTreeMap<String, String>,
        _: bool,
    ) -> IdentityResult<Vec<RenderedFile>> {
        panic!("render must not discard unknown original fields before preflight");
    }
}

#[test]
fn original_refusal_precedes_render_and_stops_nothing() -> TestResult {
    refuses_untouched(
        "if [ \"$1\" = --check-config ]; then cat >/dev/null; echo private-marker >&2; exit 2; fi",
        &NeverRender,
        "installed",
    )
}

#[test]
fn candidate_refusal_stops_nothing() -> TestResult {
    // Original fixture has no new_key; rendered build B does.
    refuses_untouched(
        &format!(
            r#"if [ "$1" = --check-config ]; then
 input=$(cat)
 case "$input" in *new_key*) echo private-marker >&2; exit 2;; esac
 digest=$(printf '%s\n' "$input" | shasum -a 256 | cut -d ' ' -f 1)
 printf '{{"format":"lys-config-check/1","build":"{B}","config_sha256":"%s"}}\n' "$digest"
 exit 0
fi"#
        ),
        &Fixed,
        "rendered",
    )
}

#[test]
fn success_without_matching_receipt_is_refused() -> TestResult {
    for receipt in [
        "private-marker",
        r#"{"format":"lys-config-check/1","build":"wrong","config_sha256":"wrong"}"#,
    ] {
        refuses_untouched(
            &format!(
                "if [ \"$1\" = --check-config ]; then cat >/dev/null; echo '{receipt}'; exit 0; fi"
            ),
            &NeverRender,
            "installed",
        )?;
    }
    Ok(())
}

#[test]
fn each_receipt_binding_is_required() -> TestResult {
    for (format, build, digest) in [
        ("wrong", B, "$digest"),
        ("lys-config-check/1", "wrong", "$digest"),
        ("lys-config-check/1", B, "wrong"),
    ] {
        refuses_untouched(
            &format!(
                r#"if [ "$1" = --check-config ]; then
 digest=$(shasum -a 256 | cut -d ' ' -f 1)
 printf '{{"format":"{format}","build":"{build}","config_sha256":"%s"}}\n' "{digest}"
 exit 0
fi"#
            ),
            &NeverRender,
            "installed",
        )?;
    }
    Ok(())
}

#[test]
fn unsupported_checker_fails_closed() -> TestResult {
    refuses_untouched(
        "if [ \"$1\" = --check-config ]; then exit 1; fi",
        &NeverRender,
        "installed",
    )
}

#[test]
fn a_config_changed_during_render_is_refused_before_intent() -> TestResult {
    struct ChangesOriginal;
    impl Render for ChangesOriginal {
        fn render(
            &self,
            layout: &Layout,
            builds: &BTreeMap<String, String>,
            screens: bool,
        ) -> IdentityResult<Vec<RenderedFile>> {
            let files = Fixed.render(layout, builds, screens)?;
            std::fs::write(layout.service_config(), b"changed by another owner")
                .map_err(|error| super::io("test edit", &layout.service_config(), &error))?;
            Ok(files)
        }
    }
    let scratch = Scratch::new()?;
    let new = scratch.work().join("b");
    build(&new, B, Behaviour::Serves)?;
    let mut engine = Recorder::default();
    let mut parts = Parts {
        runner: None,
        units: &scratch.units,
        engine: &mut engine,
        render: &ChangesOriginal,
    };
    let error = upgrade(&scratch.layout, &new, None, &mut parts, &mut |_| {})
        .err()
        .ok_or("changed original passed")?;
    assert!(error.to_string().contains("configuration changed"));
    scratch.running()?;
    assert!(!scratch.layout.upgrade_intent().exists());
    assert!(!scratch.layout.bin_previous_dir().exists());
    assert!(engine.applied.is_empty());
    assert_eq!(
        std::fs::read(scratch.layout.service_config())?,
        b"changed by another owner"
    );
    Ok(())
}
