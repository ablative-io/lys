#![cfg(test)]
//! A scratch scope's names against the text `SpiceDB` answers: the schema a
//! scope writes is under its prefix only, each side sees only its own names,
//! and each side's write keeps every name of the other as it stands.

use serde_json::json;

use super::{SCRATCH, blocks, composed, scope_for, scoped_request, scoped_schema, seen};

const SCOPE: &str = "lys/b0123456789abcdef";

/// The schema text as `SpiceDB` answers it: tabs, sorted, caveats first.
const HELD: &str = "caveat lys/b0123456789abcdef/unexpired(ends_at uint, now uint) {\n\tnow < ends_at\n}\n\ncaveat unexpired(ends_at uint, now uint) {\n\tnow < ends_at\n}\n\ndefinition fixture_notes/doc {\n\trelation viewer: grant#holder\n\tpermission view = viewer\n}\n\ndefinition grant {\n\trelation holder: person | person with unexpired\n}\n\ndefinition lys/b0123456789abcdef/grant {\n\trelation holder: lys/b0123456789abcdef/person | lys/b0123456789abcdef/person with lys/b0123456789abcdef/unexpired\n}\n\ndefinition lys/b0123456789abcdef/person {}\n\ndefinition person {}";

#[test]
fn a_scope_is_short_enough_for_the_longest_kind_and_is_scratch() {
    let scope = scope_for(&"f".repeat(64));
    assert_eq!(scope, "lys/bffffffffffffffff");
    assert!(scope.starts_with(SCRATCH));
    let longest = format!("{scope}/{}/{}", "a".repeat(40), "k".repeat(64));
    assert!(longest.len() <= 128, "{} bytes", longest.len());
}

#[test]
fn every_block_is_read_with_its_name() {
    let names: Vec<String> = blocks(HELD).into_iter().map(|block| block.name).collect();
    assert_eq!(
        names,
        [
            "lys/b0123456789abcdef/unexpired",
            "unexpired",
            "fixture_notes/doc",
            "grant",
            "lys/b0123456789abcdef/grant",
            "lys/b0123456789abcdef/person",
            "person",
        ]
    );
}

#[test]
fn the_service_sees_no_scratch_name_and_a_scope_only_its_own() {
    let service = seen(None, HELD);
    assert!(!service.contains(SCRATCH), "{service}");
    assert!(service.contains("definition fixture_notes/doc"));
    let scope = seen(Some(SCOPE), HELD);
    let names: Vec<String> = blocks(&scope).into_iter().map(|block| block.name).collect();
    assert_eq!(names, ["unexpired", "grant", "person"]);
    assert!(scope.contains("relation holder: person | person with unexpired"));
    assert!(seen(Some("lys/bother"), HELD).is_empty());
    let plain = "definition person {}";
    assert_eq!(
        seen(None, plain),
        plain,
        "a text with no scratch is unchanged"
    );
}

#[test]
fn a_scoped_schema_names_every_type_and_caveat_under_the_prefix() {
    let schema = "caveat unexpired(now uint, ends_at uint) {\n  now < ends_at\n}\n\ndefinition grant {\n  relation holder: person | agent with unexpired\n}\n\ndefinition fixture_notes/doc {\n  relation parent_ws: fixture_notes/ws\n  relation viewer: grant#holder\n  permission view = viewer + parent_ws->view\n}";
    let scoped = scoped_schema(SCOPE, schema);
    for expected in [
        "caveat lys/b0123456789abcdef/unexpired(now uint",
        "definition lys/b0123456789abcdef/grant {",
        "  relation holder: lys/b0123456789abcdef/person | lys/b0123456789abcdef/agent with lys/b0123456789abcdef/unexpired",
        "definition lys/b0123456789abcdef/fixture_notes/doc {",
        "  relation parent_ws: lys/b0123456789abcdef/fixture_notes/ws",
        "  relation viewer: lys/b0123456789abcdef/grant#holder",
        "  permission view = viewer + parent_ws->view",
    ] {
        assert!(scoped.contains(expected), "{expected} is not in {scoped}");
    }
}

#[test]
fn each_sides_write_keeps_every_name_of_the_other() {
    let service = composed(None, HELD, "definition person {}");
    let names: Vec<String> = blocks(&service)
        .into_iter()
        .map(|block| block.name)
        .collect();
    assert_eq!(
        names,
        [
            "person",
            "lys/b0123456789abcdef/unexpired",
            "lys/b0123456789abcdef/grant",
            "lys/b0123456789abcdef/person",
        ]
    );
    let scope = composed(Some(SCOPE), HELD, "definition agent {}");
    let names: Vec<String> = blocks(&scope).into_iter().map(|block| block.name).collect();
    assert_eq!(
        names,
        [
            "lys/b0123456789abcdef/agent",
            "unexpired",
            "fixture_notes/doc",
            "grant",
            "person",
        ]
    );
    let emptied = composed(Some(SCOPE), HELD, "");
    assert!(!emptied.contains(SCOPE), "{emptied}");
}

#[test]
fn a_request_names_every_type_and_caveat_under_the_prefix() {
    let mut body = json!({"updates": [{"relationship": {
        "resource": {"objectType": "grant", "objectId": "g1"},
        "relation": "holder",
        "subject": {"object": {"objectType": "person", "objectId": "p1"}},
        "optionalCaveat": {"caveatName": "unexpired", "context": {"ends_at": 9}},
    }}], "optionalPreconditions": [{"filter": {"resourceType": "lys_mirror", "optionalSubjectFilter": {"subjectType": "lys_revision"}}}]});
    scoped_request(SCOPE, &mut body);
    let relationship = &body["updates"][0]["relationship"];
    assert_eq!(
        relationship["resource"]["objectType"],
        format!("{SCOPE}/grant")
    );
    assert_eq!(relationship["resource"]["objectId"], "g1");
    assert_eq!(relationship["relation"], "holder");
    assert_eq!(
        relationship["subject"]["object"]["objectType"],
        format!("{SCOPE}/person")
    );
    assert_eq!(
        relationship["optionalCaveat"]["caveatName"],
        format!("{SCOPE}/unexpired")
    );
    let filter = &body["optionalPreconditions"][0]["filter"];
    assert_eq!(filter["resourceType"], format!("{SCOPE}/lys_mirror"));
    assert_eq!(
        filter["optionalSubjectFilter"]["subjectType"],
        format!("{SCOPE}/lys_revision")
    );
}
