#![cfg(test)]
//! Argus's rules reach an import plan only as inactive transfer entries
//! owned by liminal services (AGENTS-003 R6): the full definition, its
//! `state_revision` and its standing are kept, nothing is executable in
//! Lys, a rule scoped to other seats is kept as not imported, and a
//! secret-shaped member refuses by name without its value. The bodies are
//! recorded Argus shapes; no Argus is asked.

use lys_identity_server::seat_import_monitor::{CREDENTIAL_INLINE, MEMBER_UNSUPPORTED};
use lys_identity_server::seat_import_rules::{
    LIMINAL_OWNER, RULE_TRANSFER, executable_in_lys, transfer_rules,
};
use serde_json::{Value, json};

const CANARY: &str = "sk-canary-7f3e1d0c";

/// A rule as `GET /api/rules` lists it.
fn rule(id: &str, scope: &Value, status: &str, retired_at: Option<&str>, revision: u64) -> Value {
    json!({
        "id": id,
        "name": format!("{id} rule"),
        "author": "Tom",
        "created_at": "2030-01-01T00:00:00Z",
        "expires_at": null,
        "scope": scope,
        "event": "PreToolUse",
        "tool_matcher": "Bash",
        "matcher": "rm -rf",
        "condition": null,
        "action": "block",
        "message": "Not here.",
        "rewrite": null,
        "queue": null,
        "retired_at": retired_at,
        "retired_by": retired_at.map(|_| "Tom"),
        "paused_at": null,
        "paused_by": null,
        "state_revision": revision,
        "updated_at": null,
        "updated_by": null,
        "status": status,
        "hit_count": 41,
        "hit_count_scope": "durable",
        "hits_persisted_since": "2030-01-01T00:00:00Z",
        "hits_error": null,
        "hits_queued": 0,
        "hits_dropped": 0,
        "hits_refused": null,
        "hits_durable_through": 90,
        "last_hit": "2030-03-17T17:00:00Z",
    })
}

#[test]
fn seat_import_rules_inactive() {
    let rules = [
        rule("current", &json!("all"), "active", None, 3),
        rule(
            "retired",
            &json!(["waffles"]),
            "retired",
            Some("2030-02-01T00:00:00Z"),
            5,
        ),
        rule("theirs", &json!(["gaia"]), "active", None, 1),
    ];
    let before = rules.clone();
    let fragment = transfer_rules(&rules, "waffles");
    assert_eq!(rules, before, "the source rules are not changed");
    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    assert_eq!(fragment.sources.len(), 3);
    assert_eq!(fragment.destinations.len(), 2);
    for (entry, (id, revision, status)) in fragment
        .destinations
        .iter()
        .zip([("current", 3, "active"), ("retired", 5, "retired")])
    {
        assert_eq!(entry.record_kind, RULE_TRANSFER);
        assert_eq!(entry.record_id, format!("rule:{id}"));
        assert!(!executable_in_lys(entry), "{entry:?}");
        assert_eq!(entry.change["owner"], LIMINAL_OWNER);
        assert_eq!(entry.change["active"], false);
        assert_eq!(entry.change["state_revision"], revision);
        assert_eq!(entry.change["status"], status);
        let definition = &entry.change["definition"];
        assert_eq!(definition["matcher"], "rm -rf");
        assert_eq!(definition["action"], "block");
        assert_eq!(definition["state_revision"], revision);
        assert!(definition.get("hit_count").is_none(), "{definition}");
        assert!(definition.get("last_hit").is_none(), "{definition}");
        assert_eq!(entry.source_entry_ids, [format!("monitor:rule:{id}")]);
    }
    assert_eq!(
        fragment.destinations[1].change["definition"]["retired_at"],
        "2030-02-01T00:00:00Z"
    );
    let [excluded] = fragment.excluded.as_slice() else {
        panic!("one exclusion: {:?}", fragment.excluded);
    };
    assert_eq!(excluded.source_id, "monitor:rule:theirs");
    assert_eq!(excluded.revision, "1");
}

#[test]
fn a_rule_hit_count_is_not_its_revision() {
    let mut first = rule("current", &json!("all"), "active", None, 3);
    let once = transfer_rules(std::slice::from_ref(&first), "waffles");
    first["hit_count"] = json!(900);
    first["last_hit"] = json!("2030-03-17T17:45:00Z");
    let again = transfer_rules(std::slice::from_ref(&first), "waffles");
    assert_eq!(once.sources, again.sources);
    assert_eq!(once.destinations, again.destinations);
}

#[test]
fn a_secret_shaped_rule_member_refuses_without_its_value() {
    let mut leaky = rule("leaky", &json!("all"), "active", None, 2);
    leaky["rewrite"] =
        json!({"command": format!("curl -H 'Authorization: Bearer {CANARY}'"), "api_key": CANARY});
    let mut message = rule("message", &json!("all"), "active", None, 2);
    message["message"] = json!(CANARY);
    let fragment = transfer_rules(&[leaky, message], "waffles");
    let names: Vec<&str> = fragment
        .refusals
        .iter()
        .map(|refused| refused.name.as_str())
        .collect();
    assert_eq!(names, [CREDENTIAL_INLINE, CREDENTIAL_INLINE]);
    assert_eq!(fragment.refusals[0].member, "rule leaky.rewrite.api_key");
    assert_eq!(fragment.refusals[1].member, "rule message.message");
    assert!(fragment.destinations.is_empty());
    let shown = serde_json::to_string(&fragment).unwrap_or_default();
    assert!(!shown.contains(CANARY), "{shown}");
}

#[test]
fn a_rule_without_its_revision_or_scope_is_refused_by_name() {
    let mut unrevised = rule("unrevised", &json!("all"), "active", None, 1);
    unrevised["state_revision"] = Value::Null;
    let unscoped = rule("unscoped", &json!(7), "active", None, 1);
    let fragment = transfer_rules(&[unrevised, unscoped], "waffles");
    let names: Vec<&str> = fragment
        .refusals
        .iter()
        .map(|refused| refused.name.as_str())
        .collect();
    assert_eq!(names, [MEMBER_UNSUPPORTED, MEMBER_UNSUPPORTED]);
    assert_eq!(fragment.refusals[1].member, "rule unscoped.scope");
    assert!(fragment.destinations.is_empty());
}
