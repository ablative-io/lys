use super::*;
use crate::budgets_state::CallSeen;

fn call(agent: &str, id: &str, at_ms: i64) -> Usage {
    Usage {
        event: format!("native:machine:{id}"),
        agent: agent.to_owned(),
        at_ms,
        call: Some(Box::new(CallSeen {
            id: id.to_owned(),
            model: None,
            input_tokens: None,
            output_tokens: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            run: "0123456789abcdef0123456789abcdef".to_owned(),
            record: None,
        })),
        ..Usage::default()
    }
}

#[test]
fn an_agents_calls_page_newest_first_and_are_found_by_id() -> Result<(), String> {
    let mut uses: Vec<Usage> = (0..5_i64)
        .map(|at| call("a", &format!("c{at}"), at))
        .collect();
    // A use that counts no call, and another agent's call, are never listed.
    uses.push(Usage {
        agent: "a".to_owned(),
        at_ms: 9,
        ..Usage::default()
    });
    uses.push(call("b", "other", 9));
    let index = Index::from_uses(&uses)?;
    assert_eq!(index.calls("a", None, 2), (vec![4, 3], true));
    assert_eq!(index.calls("a", Some((3, 3)), 2), (vec![2, 1], true));
    assert_eq!(index.calls("a", Some((1, 1)), 2), (vec![0], false));
    assert_eq!(index.calls("a", None, 5), (vec![4, 3, 2, 1, 0], false));
    assert_eq!(index.calls("nobody", None, 2), (Vec::new(), false));
    assert_eq!(index.call("a", "c3"), Some(3));
    assert_eq!(index.call("a", "other"), None);
    assert_eq!(index.call("b", "other"), Some(6));
    assert!(index.models().is_empty());
    let mut named = call("a", "named", 10);
    if let Some(seen) = named.call.as_mut() {
        seen.model = Some("model-one".to_owned());
    }
    let mut index = index;
    index.insert(&named, 7)?;
    index.insert(&named, 8)?;
    assert_eq!(index.models().iter().collect::<Vec<_>>(), vec!["model-one"]);
    Ok(())
}
