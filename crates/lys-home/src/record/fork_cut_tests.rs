//! Gates on the fork's names and refusals (HOME-006 R1) and on resolving
//! and cutting (R2): the two custom types are named as the record states,
//! each refusal names itself and its ids, the cut is the chain to the last
//! assistant message in order with every side leaf left out, the bytes
//! read are the lantern's row and the point's ancestry only, a lantern
//! with a lit-in session cuts from it while an older record needs one
//! named, a `lit_in` that is not a session id is refused by name, and
//! every refusal writes nothing. The fixture home built here is
//! the one the fork and report gates build on.
//!
//! The lantern `L2` is written by hand carrying `lit_in`, the key the
//! lantern card records in a later round: the light act on this tree writes
//! none, so every lantern it lights (`L5`, `L6`, `L1`, `C5`) resolves by the
//! older-record rule, as `O2` does. No test name carries a content sentinel.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use serde_json::{Map, Value, json};
use tempfile::TempDir;

use crate::error::HomeError;
use crate::harness::claude_code::events::{HarnessEvent, KIND_ATTACHMENT, KIND_PERMISSION_MODE};
use crate::record::blocks::{BlockStore, Hash};
use crate::record::entries::{
    CUSTOM_FORK, CUSTOM_FORKED_FROM, CUSTOM_HARNESS_EVENT, CUSTOM_LANTERN, Entry, EntryBase,
    EntryBody,
};
use crate::record::fork_cut::{Cut, resolve_and_cut};
use crate::record::index::Index;
use crate::record::lantern::light;
use crate::record::{Home, now};

type Gate = Result<(), Box<dyn Error>>;

/// The fixture's parent session.
pub(crate) const PARENT: &str = "parent";
/// The fixture's compacted session.
pub(crate) const COMPACTED: &str = "compacted";
/// The eight content sentinels; none may appear in a report, an error, a
/// log line or a test name.
pub(crate) const SENTINELS: [&str; 8] = [
    "fixture-text-1",
    "fixture-text-2",
    "fixture-text-4",
    "fixture-text-5",
    "fixture-text-6",
    "fixture-text-7",
    "fixture-text-8",
    "fixture-text-9",
];
const NOTE: &str = "fixture note";
const LIGHTER: &str = "fixture-lighter";
const STAMP: &str = "2026-01-01T00:00:00.000Z";

/// The ids the fixture's lanterns were lit under.
pub(crate) struct Lanterns {
    pub l2: String,
    pub o2: String,
    pub l5: String,
    pub l6: String,
    pub l1: String,
    pub c5: String,
}

fn base(id: &str, parent: Option<&str>) -> EntryBase {
    EntryBase {
        id: id.to_owned(),
        parent_id: parent.map(str::to_owned),
        timestamp: STAMP.to_owned(),
    }
}

/// A text part as the importer stores it.
pub(crate) fn text_part(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

/// A user message entry with the given parts.
pub(crate) fn user(id: &str, parent: Option<&str>, parts: &[Value]) -> Entry {
    Entry {
        base: base(id, parent),
        body: EntryBody::Message {
            message: json!({"role": "user", "content": parts, "timestamp": 0}),
        },
    }
}

/// An assistant message entry with the given parts.
pub(crate) fn assistant(id: &str, parent: Option<&str>, parts: &[Value]) -> Entry {
    Entry {
        base: base(id, parent),
        body: EntryBody::Message {
            message: json!({"role": "assistant", "content": parts, "api": "anthropic-messages",
                "provider": "anthropic", "model": "claude-fixture", "stopReason": "stop", "timestamp": 0}),
        },
    }
}

/// A `lys.harness_event` entry of `kind` whose record names `record`.
fn event(id: &str, parent: &str, kind: &str, record: &Hash) -> Result<Entry, Box<dyn Error>> {
    let event = HarnessEvent {
        kind: kind.to_owned(),
        source_uuid: None,
        record: Some(record.as_str().to_owned()),
        detail: Map::new(),
    };
    Ok(Entry {
        base: base(id, Some(parent)),
        body: EntryBody::Custom {
            custom_type: CUSTOM_HARNESS_EVENT.to_owned(),
            data: Some(event.data()?),
        },
    })
}

/// A `lys.lantern` entry written by hand at `point`: with `lit_in`, the
/// light act's data plus that key; without, the older record's `point`,
/// `note` and `lit_by` exactly.
pub(crate) fn lantern_entry(id: &str, parent: &str, point: &str, lit_in: Option<&str>) -> Entry {
    let data = match lit_in {
        Some(session) => {
            json!({"point": point, "note": NOTE, "lit_by": LIGHTER, "lit_at": now(), "lit_in": session})
        }
        None => json!({"point": point, "note": NOTE, "lit_by": LIGHTER}),
    };
    Entry {
        base: base(id, Some(parent)),
        body: EntryBody::Custom {
            custom_type: CUSTOM_LANTERN.to_owned(),
            data: Some(data),
        },
    }
}

/// Put a message's parts through the store as the importer does, each as
/// `serde_json::to_vec` of the part as it stands in the entry.
pub(crate) fn put_parts(blocks: &BlockStore, parts: &[Value]) -> Result<(), Box<dyn Error>> {
    for part in parts {
        blocks.put(&serde_json::to_vec(part)?)?;
    }
    Ok(())
}

/// The fixture home: `parent` and `compacted`, with their lanterns lit.
pub(crate) fn fixture_home() -> Result<(TempDir, Home, Lanterns), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let blocks = home.blocks()?;
    let s2_record = blocks.put(b"fixture-record-s2")?.hash;
    let e3_record = blocks.put(b"fixture-record-e3")?.hash;
    let text = |n: usize| text_part(&format!("fixture-text-{n}"));
    {
        let mut session = home.create_session(PARENT, "/fixture", None)?;
        session.append_entry(&user("e1", None, &[text(1)]))?;
        session.append_entry(&assistant("e2", Some("e1"), &[text(2)]))?;
        session.append_entry(&event("s2", "e2", KIND_PERMISSION_MODE, &s2_record)?)?;
        session.move_head(Some("e2"))?;
    }
    {
        let mut session = home.open_session(PARENT)?;
        session.append_entry(&lantern_entry("L2", "e2", "e2", Some(PARENT)))?;
        session.append_entry(&lantern_entry("O2", "L2", "e2", None))?;
        session.append_entry(&event("e3", "O2", KIND_ATTACHMENT, &e3_record)?)?;
        session.append_entry(&user("e4", Some("e3"), &[text(4)]))?;
        session.append_entry(&assistant("e5", Some("e4"), &[text(5)]))?;
        let image = json!({"type": "image", "data": "aWdub3JlZA==", "mimeType": "image/png"});
        session.append_entry(&user("e6", Some("e5"), &[text(6), image]))?;
        session.append_entry(&assistant("e7", Some("e6"), &[text(7)]))?;
        session.append_entry(&user("e8", Some("e7"), &[text(8)]))?;
        session.append_entry(&assistant("e9", Some("e8"), &[text(9)]))?;
    }
    for n in [1, 2, 4, 5, 6, 7, 8, 9] {
        put_parts(&blocks, &[text(n)])?;
    }
    let l5 = light(&home, PARENT, "e5", NOTE, LIGHTER)?.id;
    let l6 = light(&home, PARENT, "e6", NOTE, LIGHTER)?.id;
    let l1 = light(&home, PARENT, "e1", NOTE, LIGHTER)?.id;
    {
        let mut session = home.create_session(COMPACTED, "/fixture", None)?;
        session.append_entry(&user("e1", None, &[text(1)]))?;
        session.append_entry(&assistant("e2", Some("e1"), &[text(2)]))?;
        session.append_entry(&Entry {
            base: base("c3", Some("e2")),
            body: EntryBody::Compaction {
                summary: "fixture summary".to_owned(),
                first_kept_entry_id: "e2".to_owned(),
                tokens_before: 0,
                rest: Map::new(),
            },
        })?;
        session.append_entry(&user("e4", Some("c3"), &[text(4)]))?;
        session.append_entry(&assistant("e5", Some("e4"), &[text(5)]))?;
    }
    let c5 = light(&home, COMPACTED, "e5", NOTE, LIGHTER)?.id;
    let lanterns = Lanterns {
        l2: "L2".to_owned(),
        o2: "O2".to_owned(),
        l5,
        l6,
        l1,
        c5,
    };
    Ok((dir, home, lanterns))
}

/// Every file under `sessions/` by name with the SHA-256 of its bytes.
pub(crate) fn session_files(home: &Home) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(home.root().join("sessions"))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        files.insert(name, Hash::of(&std::fs::read(entry.path())?).to_string());
    }
    Ok(files)
}

fn cut(home: &Home, lantern: &str, session: Option<&str>) -> Result<Cut, Box<dyn Error>> {
    Ok(resolve_and_cut(home, lantern, session)?)
}

#[test]
fn the_fork_custom_types_are_named_as_the_record_states() {
    assert_eq!(CUSTOM_FORK, "lys.fork");
    assert_eq!(CUSTOM_FORKED_FROM, "lys.forked_from");
}

#[test]
fn each_fork_refusal_names_itself_and_the_ids_it_was_given() {
    let mut refusals = Vec::new();

    let unknown = HomeError::NoSuchLantern {
        lantern: "no-such-lantern".into(),
    }
    .to_string();
    assert!(unknown.contains("no-such-lantern"), "{unknown}");
    refusals.push(unknown);

    let ambiguous = HomeError::LanternAmbiguous {
        lantern: "x".into(),
        sessions: vec!["a".into(), "b".into()],
    }
    .to_string();
    assert!(ambiguous.starts_with("lantern_ambiguous"), "{ambiguous}");
    for id in ["x", "a", "b"] {
        assert!(ambiguous.contains(id), "{ambiguous}");
    }
    refusals.push(ambiguous);

    let not_here = HomeError::LanternNotLitHere {
        lantern: "x".into(),
        session: "a".into(),
        lit_in: "b".into(),
    }
    .to_string();
    assert!(not_here.starts_with("lantern_not_lit_here"), "{not_here}");
    for id in ["x", "a", "b"] {
        assert!(not_here.contains(id), "{not_here}");
    }
    refusals.push(not_here);

    let nothing = HomeError::NothingToFork {
        lantern: "x".into(),
    }
    .to_string();
    assert!(nothing.starts_with("nothing_to_fork"), "{nothing}");
    assert!(nothing.contains('x'), "{nothing}");
    assert!(
        nothing.contains("before any assistant message"),
        "{nothing}"
    );
    refusals.push(nothing);

    assert_eq!(refusals.len(), 4);
}

#[test]
fn the_cut_is_the_chain_to_the_last_assistant_message_with_no_side_leaf() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let mut cuts = Vec::new();

    let l5 = cut(&home, &lanterns.l5, None)?;
    assert_eq!(l5.ids(), ["e1", "e2", "L2", "O2", "e3", "e4", "e5"]);
    assert_eq!(l5.cut_at, "e5");
    assert!(!l5.coordinate_carried());
    assert!(l5.carried.is_none());
    assert_eq!(l5.session, PARENT);
    assert_eq!(l5.point, "e5");
    cuts.push(l5);

    let l6 = cut(&home, &lanterns.l6, None)?;
    assert_eq!(l6.ids(), ["e1", "e2", "L2", "O2", "e3", "e4", "e5"]);
    assert_eq!(l6.cut_at, "e5");
    assert!(l6.coordinate_carried());
    assert_eq!(l6.carried.as_ref().map(Entry::id), Some("e6"));
    assert_eq!(l6.point, "e6");
    cuts.push(l6);

    let l2 = cut(&home, &lanterns.l2, None)?;
    assert_eq!(l2.ids(), ["e1", "e2"]);
    assert_eq!(l2.cut_at, "e2");
    assert_eq!(l2.lit_in.as_deref(), Some(PARENT));
    cuts.push(l2);

    let c5 = cut(&home, &lanterns.c5, None)?;
    assert_eq!(c5.ids(), ["e1", "e2", "c3", "e4", "e5"]);
    assert_eq!(c5.session, COMPACTED);
    cuts.push(c5);

    assert_eq!(cuts.len(), 4);
    for taken in &cuts {
        assert!(!taken.ids().contains(&"s2"));
        assert_eq!(taken.rows.len(), taken.entries.len());
        for (row, entry) in taken.rows.iter().zip(&taken.entries) {
            assert_eq!(row.id, entry.id());
        }
    }
    Ok(())
}

#[test]
fn resolving_and_cutting_reads_the_lantern_row_and_the_ancestry_only() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let file = home.session_path(PARENT)?;
    let (_, index, scanned) = Index::read(&file)?;
    assert!(!scanned);
    let mut expected = 0u64;
    for id in [
        lanterns.l5.as_str(),
        "e1",
        "e2",
        "L2",
        "O2",
        "e3",
        "e4",
        "e5",
    ] {
        expected += index.row(id).ok_or("a row")?.len;
    }
    let taken = cut(&home, &lanterns.l5, None)?;
    assert_eq!(taken.bytes_read, expected);
    assert!(taken.bytes_read < std::fs::metadata(&file)?.len());
    Ok(())
}

#[test]
fn a_lit_in_lantern_cuts_from_its_session_and_an_older_record_needs_one_named() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    {
        let reader = home.read_session(PARENT)?;
        let mut copy = home.create_session("A", "/fixture", None)?;
        for id in ["e1", "e2", "L2", "O2"] {
            copy.append_entry(&reader.entry(id)?)?;
        }
        assert_eq!(copy.len()?, 4);
    }
    let from_parent = cut(&home, &lanterns.l2, None)?;
    assert_eq!(from_parent.session, PARENT);
    assert_eq!(from_parent.ids(), ["e1", "e2"]);

    let elsewhere = resolve_and_cut(&home, &lanterns.l2, Some("A"));
    assert!(
        matches!(
            &elsewhere,
            Err(HomeError::LanternNotLitHere { lantern, session, lit_in })
                if lantern == "L2" && session == "A" && lit_in == PARENT
        ),
        "{elsewhere:?}"
    );

    let ambiguous = resolve_and_cut(&home, &lanterns.o2, None);
    assert!(
        matches!(
            &ambiguous,
            Err(HomeError::LanternAmbiguous { lantern, sessions })
                if lantern == "O2" && *sessions == ["A".to_owned(), PARENT.to_owned()]
        ),
        "{ambiguous:?}"
    );

    let named = cut(&home, &lanterns.o2, Some(PARENT))?;
    assert_eq!(named.session, PARENT);
    assert_eq!(named.ids(), ["e1", "e2"]);
    assert_eq!(named.lit_in, None);
    Ok(())
}

#[test]
fn each_refusal_names_the_lantern_and_writes_nothing() -> Gate {
    let (_dir, home, lanterns) = fixture_home()?;
    let before = session_files(&home)?;
    let mut refusals: Vec<HomeError> = Vec::new();

    for id in ["no-such-lantern", "e5"] {
        let refused = resolve_and_cut(&home, id, None);
        assert!(
            matches!(&refused, Err(HomeError::NoSuchLantern { lantern }) if lantern == id),
            "{refused:?}"
        );
        refusals.push(refused.err().ok_or("refused")?);
    }
    let elsewhere = resolve_and_cut(&home, &lanterns.o2, Some(COMPACTED));
    assert!(
        matches!(
            &elsewhere,
            Err(HomeError::UnknownLantern { session, id }) if session == COMPACTED && id == "O2"
        ),
        "{elsewhere:?}"
    );
    refusals.push(elsewhere.err().ok_or("refused")?);
    let nothing = resolve_and_cut(&home, &lanterns.l1, None);
    assert!(
        matches!(&nothing, Err(HomeError::NothingToFork { lantern }) if *lantern == lanterns.l1),
        "{nothing:?}"
    );
    refusals.push(nothing.err().ok_or("refused")?);
    let absent = resolve_and_cut(&home, &lanterns.l5, Some("no-such-session"));
    assert!(
        matches!(&absent, Err(HomeError::UnknownSession { session }) if session == "no-such-session"),
        "{absent:?}"
    );
    refusals.push(absent.err().ok_or("refused")?);
    let other_kind = resolve_and_cut(&home, "e5", Some(PARENT));
    assert!(
        matches!(&other_kind, Err(HomeError::UnknownLantern { session, id }) if session == PARENT && id == "e5"),
        "{other_kind:?}"
    );
    refusals.push(other_kind.err().ok_or("refused")?);

    assert_eq!(refusals.len(), 6);
    for refused in &refusals {
        let text = refused.to_string();
        for sentinel in SENTINELS {
            assert!(!text.contains(sentinel));
        }
        assert!(!text.contains(NOTE));
    }
    let after = session_files(&home)?;
    assert_eq!(after.len(), before.len());
    assert_eq!(after, before);
    assert!(
        !Path::new(&home.root().join("sessions"))
            .join("A.jsonl")
            .exists()
    );
    Ok(())
}

#[test]
fn a_lit_in_session_that_holds_no_copy_refuses_naming_the_holder_read() -> Gate {
    let (_dir, home, _) = fixture_home()?;
    {
        let reader = home.read_session(PARENT)?;
        let mut copy = home.create_session("A", "/fixture", None)?;
        copy.append_entry(&reader.entry("e1")?)?;
        copy.append_entry(&reader.entry("e2")?)?;
        copy.append_entry(&lantern_entry("N2", "e2", "e2", Some("elsewhere")))?;
        // A session of the home that holds no copy of the lantern.
        home.create_session("elsewhere", "/fixture", None)?;
    }
    let mut refusals = 0;
    for (session, named) in [(None, "A"), (Some("A"), "A"), (Some(PARENT), PARENT)] {
        let refused = resolve_and_cut(&home, "N2", session);
        assert!(
            matches!(
                &refused,
                Err(HomeError::LanternNotLitHere { lantern, session, lit_in })
                    if lantern == "N2" && session == named && lit_in == "elsewhere"
            ),
            "{refused:?}"
        );
        refusals += 1;
    }
    assert_eq!(refusals, 3);
    Ok(())
}

#[test]
fn a_lit_in_that_is_not_a_session_id_is_refused_by_name() -> Gate {
    let (_dir, home, _) = fixture_home()?;
    {
        let reader = home.read_session(PARENT)?;
        let mut copy = home.create_session("A", "/fixture", None)?;
        copy.append_entry(&reader.entry("e1")?)?;
        copy.append_entry(&reader.entry("e2")?)?;
        for (id, lit_in) in [
            ("N1", json!(null)),
            ("N2", json!(5)),
            ("N3", json!("../elsewhere")),
            ("N4", json!("no-such-session")),
        ] {
            let mut entry = lantern_entry(id, "e2", "e2", Some("placeholder"));
            let EntryBody::Custom {
                data: Some(data), ..
            } = &mut entry.body
            else {
                return Err("a custom entry with data".into());
            };
            data["lit_in"] = lit_in;
            copy.append_entry(&entry)?;
        }
    }
    let before = session_files(&home)?;
    let mut refusals = 0;
    for (id, what) in [
        ("N1", "is null"),
        ("N2", "is not a string"),
        ("N3", "is not a safe session name"),
        ("N4", "names no session of the home"),
    ] {
        for session in [None, Some("A")] {
            let refused = resolve_and_cut(&home, id, session);
            assert!(
                matches!(
                    &refused,
                    Err(HomeError::LitInNotASession { lantern, what: found })
                        if lantern == id && *found == what
                ),
                "{refused:?}"
            );
            assert!(
                refused
                    .err()
                    .ok_or("refused")?
                    .to_string()
                    .starts_with("lit_in_not_a_session")
            );
            refusals += 1;
        }
    }
    assert_eq!(refusals, 8);
    assert_eq!(session_files(&home)?, before);
    Ok(())
}
