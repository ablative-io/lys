//! Disposable upgrade proof driver. The production CLI's own upgrade function
//! runs unchanged; its existing ready callback exits before recording Started.
//! This example is never installed and refuses roots without the proof marker.

#[path = "upgrade_window/commands.rs"]
pub mod commands;
#[path = "../src/identity/mod.rs"]
pub mod identity;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use clap::{Parser, Subcommand};
use identity::config::DeploymentConfig;
use identity::install::{self, layout::Layout};
use identity::upgrade::{self, Compose, Parts, intent::Intent, render::Templates};
use lys_log_store::{FileLeafStore, FrontierLog};
use serde_json::{Value, json};

type ProbeResult<T = ()> = Result<T, Box<dyn Error>>;
const MARKER: &str = "lys-disposable-upgrade-proof/v1";
const INTERRUPTED: i32 = 75;

#[derive(Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("LYS_BUILD"), ")"))]
struct Arguments {
    #[arg(long)]
    root: PathBuf,
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand)]
enum Mode {
    /// Restart only this marked fixture through the production unit lifecycle.
    Restart,
    /// Recover this marked fixture through the candidate's production rollback.
    Recover,
    /// Run the actual upgrade, verify its reversible window, then interrupt it.
    Window {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        surface: PathBuf,
        #[arg(long)]
        verifier: PathBuf,
    },
    /// Deliberately append a new-format team hold to a stopped disposable store.
    Poison {
        #[arg(long)]
        fixture: PathBuf,
    },
}

fn checked_layout(root: &Path) -> ProbeResult<Layout> {
    let root = root.canonicalize()?;
    let marker = root.join(".upgrade-proof");
    if std::fs::read_to_string(&marker)? != MARKER {
        return Err(format!("{} is not a disposable upgrade proof", root.display()).into());
    }
    Ok(Layout::at(root))
}

fn reversible(layout: &Layout) -> ProbeResult {
    let intent = Intent::read(layout)?.ok_or("the proof has no upgrade intent")?;
    if intent.has(upgrade::intent::Step::Started) {
        return Err("the proof window already committed Started".into());
    }
    Ok(())
}

fn window(layout: &Layout, from: &Path, surface: &Path, verifier: &Path) -> ProbeResult {
    let config = DeploymentConfig::load(&layout.deployment_config())?;
    install::server_state(layout, &config)?;
    let units = upgrade::units(layout);
    let runner = from.join("lys");
    let templates = Templates::default();
    let mut parts = Parts {
        runner: Some(&runner),
        units: &units,
        engine: &mut Compose,
        render: &templates,
    };
    upgrade::upgrade(layout, from, Some(surface), &mut parts, &mut |line| {
        println!("{line}");
        if line == "lys-identity-server started and ready" {
            let result = reversible(layout).and_then(|()| {
                let status = Command::new("python3")
                    .arg(verifier)
                    .arg("--root")
                    .arg(&layout.root)
                    .status()?;
                if status.success() {
                    Ok(())
                } else {
                    Err(format!("reversible-window verifier failed: {status}").into())
                }
            });
            match result {
                Ok(()) => std::process::exit(INTERRUPTED),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
    })?;
    Err("the upgrade completed without the expected service-ready callback".into())
}

fn required<'a>(value: &'a Value, key: &str) -> ProbeResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("legacy fixture has no string member {key}").into())
}

fn poison(layout: &Layout, fixture: &Path) -> ProbeResult {
    reversible(layout)?;
    upgrade::swap::stop_all(&upgrade::units(layout), &mut |line| println!("{line}"))?;
    let config: Value = serde_json::from_slice(&std::fs::read(layout.service_config())?)?;
    let directory = PathBuf::from(required(&config, "teams_dir")?).canonicalize()?;
    if !directory.starts_with(&layout.root) {
        return Err(format!("team log {} leaves the proof root", directory.display()).into());
    }
    let fixture: Value = serde_json::from_slice(&std::fs::read(fixture)?)?;
    let team = required(&fixture, "team")?;
    let member = fixture
        .get("foreign")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .ok_or("legacy fixture has no foreign member")?;
    let leaf = json!({"line":"held", "operation":"op-deaddeaddeaddeaddeaddeaddeaddead",
        "team":team, "member":member, "reason":"deliberate upgrade proof negative control", "at":0});
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&directory)?)?;
    let (index, _) = log.append(&serde_json::to_vec(&leaf)?)?;
    let path = directory.join("leaves").join(format!("{index:020}"));
    println!("negative control new-format record: {}", path.display());
    Ok(())
}

fn run(arguments: Arguments) -> ProbeResult {
    let layout = checked_layout(&arguments.root)?;
    match arguments.mode {
        Mode::Restart => restart(&layout),
        Mode::Recover => {
            reversible(&layout)?;
            upgrade::swap::recover(
                &layout,
                &upgrade::units(&layout),
                &mut Compose,
                &mut |line| println!("{line}"),
            )?;
            Ok(())
        }
        Mode::Window {
            from,
            surface,
            verifier,
        } => window(&layout, &from, &surface, &verifier),
        Mode::Poison { fixture } => poison(&layout, &fixture),
    }
}

fn restart(layout: &Layout) -> ProbeResult {
    if Intent::read(layout)?.is_some() {
        return Err("restart proof requires the upgrade intent to be cleared".into());
    }
    let units = upgrade::units(layout);
    upgrade::swap::stop_all(&units, &mut |line| println!("{line}"))?;
    for unit in &units {
        upgrade::launch(layout, unit, true)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Arguments::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("upgrade proof refused: {error}");
            ExitCode::FAILURE
        }
    }
}
