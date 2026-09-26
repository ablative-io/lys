#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on the `template_render` event: under the 512-byte cap with long
//! paths riding in the manifest block, the five earlier kinds and the cap
//! unchanged, and RECORD.md naming the kind and the manifest's fields.

use serde_json::Value;

use crate::harness::claude_code::events::{
    KIND_ATTACHMENT, KIND_HOOK, KIND_PERMISSION_MODE, KIND_SYSTEM, KIND_TEMPLATE_RENDER,
    KIND_TOOL_COMPLETED, MAX_DATA_BYTES, ManifestFile, RenderManifest, template_render,
};
use crate::record::Home;
use crate::record::blocks::Hash;

const RECORD_MD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/design/home/RECORD.md"
);

fn hex(byte: u8) -> Hash {
    Hash::of(&[byte])
}

#[test]
fn a_template_render_event_from_three_hashes_fits_the_cap_with_no_source_uuid() {
    let event = template_render(&hex(1), &hex(2), &hex(3), 5);
    let data = event.data().unwrap();
    let len = serde_json::to_vec(&data).unwrap().len();
    assert!(len <= MAX_DATA_BYTES, "{len}");
    assert_eq!(data["kind"], KIND_TEMPLATE_RENDER);
    assert_eq!(data["kind"], "template_render");
    assert_eq!(data["source_uuid"], Value::Null);
    assert_eq!(data["harness"], "claude-code");
    assert_eq!(data["record"], hex(3).as_str());
    assert_eq!(data["detail"]["template"], hex(1).as_str());
    assert_eq!(data["detail"]["session_head"], hex(2).as_str());
    assert_eq!(data["detail"]["files"], 5);
    assert_eq!(data["detail"].as_object().unwrap().len(), 3);
}

#[test]
fn five_long_paths_ride_in_the_manifest_block_and_the_event_stays_under_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    let home = Home::open(dir.path().join("home")).unwrap();
    let blocks = home.blocks().unwrap();
    let files: Vec<ManifestFile> = (0..5u8)
        .map(|n| ManifestFile {
            path: std::path::PathBuf::from(format!("/{}", "p".repeat(239))).join(format!("{n}")),
            sha256: hex(n).as_str().to_owned(),
        })
        .collect();
    assert_eq!(files.len(), 5);
    for f in &files {
        assert!(f.path.as_os_str().len() >= 240, "{}", f.path.display());
    }
    let manifest = RenderManifest {
        template: hex(10).as_str().to_owned(),
        session_head: hex(11).as_str().to_owned(),
        head: Some("e4".to_owned()),
        uuid: "00000000-0000-4000-8000-000000000001".to_owned(),
        files,
    };
    let put = manifest.store(&blocks).unwrap();
    assert!(put.new);
    let stored: RenderManifest = serde_json::from_slice(&blocks.get(&put.hash).unwrap()).unwrap();
    assert_eq!(stored, manifest);
    assert!(serde_json::to_vec(&manifest).unwrap().len() > MAX_DATA_BYTES);
    let event = template_render(&hex(10), &hex(11), &put.hash, 5);
    let data = event.data().unwrap();
    assert!(serde_json::to_vec(&data).unwrap().len() <= MAX_DATA_BYTES);
    assert_eq!(data["record"], put.hash.as_str());
    assert!(!data.to_string().contains("ppp"), "no path in the event");
    dir.close().unwrap();
}

#[test]
fn the_five_earlier_kinds_and_the_cap_keep_their_values() {
    assert_eq!(KIND_HOOK, "hook");
    assert_eq!(KIND_PERMISSION_MODE, "permission_mode");
    assert_eq!(KIND_TOOL_COMPLETED, "tool_completed");
    assert_eq!(KIND_ATTACHMENT, "attachment");
    assert_eq!(KIND_SYSTEM, "system");
    assert_eq!(MAX_DATA_BYTES, 512);
    let kinds = [
        KIND_HOOK,
        KIND_PERMISSION_MODE,
        KIND_TOOL_COMPLETED,
        KIND_ATTACHMENT,
        KIND_SYSTEM,
        KIND_TEMPLATE_RENDER,
    ];
    let distinct: std::collections::BTreeSet<&str> = kinds.iter().copied().collect();
    assert_eq!(distinct.len(), 6);
}

#[test]
fn record_md_names_the_sixth_kind_and_the_manifest_fields() {
    let text = std::fs::read_to_string(RECORD_MD).unwrap();
    assert!(text.contains("`template_render`"), "kind named");
    let fields = ["template", "session_head", "head", "uuid", "files"];
    let section: String = text
        .lines()
        .skip_while(|l| !l.starts_with("- `template_render`"))
        .take(14)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(section.contains("manifest block"), "{section}");
    assert!(
        section.contains("`{template, session_head, head, uuid, files}`"),
        "{section}"
    );
    let found = fields.iter().filter(|f| section.contains(*f)).count();
    assert_eq!(found, fields.len(), "{section}");
    assert!(section.contains("side leaf"));
}
