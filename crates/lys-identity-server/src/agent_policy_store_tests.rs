use std::error::Error;

use lys_log_store::Tail;
use lys_runner::judge::{Authority, Policy, Rule, RuleKind};

use super::{Held, PolicyStore};

#[test]
fn an_existing_hard_policy_survives_default_initialization_and_reopen() -> Result<(), Box<dyn Error>>
{
    let dir = tempfile::tempdir()?;
    let key = std::sync::Arc::new(lys_core::Ed25519Identity::load_or_generate(
        &dir.path().join("key"),
    )?);
    let path = dir.path().join("policies");
    let mut store = PolicyStore::open(&path, std::sync::Arc::clone(&key))?;
    let mut policy = invalid_policy();
    policy.rules[0].target = Some("/private".to_owned());
    let recorded = store.set(policy, 0)?;
    store.ensure_default("agent-a")?;
    assert_eq!(store.held().latest("agent-a"), Some(&recorded));
    drop(store);
    let mut reopened = PolicyStore::open(&path, key)?;
    reopened.ensure_default("agent-a")?;
    assert_eq!(reopened.held().latest("agent-a"), Some(&recorded));
    Ok(())
}

fn invalid_policy() -> Policy {
    Policy {
        version: 1,
        agent: "agent-a".to_owned(),
        rules: vec![Rule {
            id: "relative-path".to_owned(),
            tool: "Write".to_owned(),
            kind: RuleKind::PathPrefix,
            target: Some("relative/path".to_owned()),
            authority: Authority::Hard,
        }],
    }
}

#[test]
fn an_unchecked_policy_leaf_is_refused_before_it_is_held() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    let refused = held.fold(&Tail {
        from: 0,
        leaves: vec![serde_json::to_vec(&invalid_policy())?],
    });
    assert!(refused.is_err(), "invalid stored policy was admitted");
    assert!(held.policies.is_empty());
    Ok(())
}

#[test]
fn an_unchecked_policy_snapshot_is_refused() -> Result<(), Box<dyn Error>> {
    let held = Held {
        policies: [("agent-a".to_owned(), vec![invalid_policy()])].into(),
    };
    assert!(Held::decode(&held.encode()?).is_err());
    Ok(())
}

#[test]
fn a_snapshot_cannot_assign_another_agents_policy() -> Result<(), Box<dyn Error>> {
    let held = Held {
        policies: [(
            "agent-b".to_owned(),
            vec![Policy {
                version: 1,
                agent: "agent-a".to_owned(),
                rules: Vec::new(),
            }],
        )]
        .into(),
    };
    assert!(Held::decode(&held.encode()?).is_err());
    Ok(())
}
