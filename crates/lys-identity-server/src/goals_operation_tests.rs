use std::error::Error;

use super::{
    Answered, Change, Changed, Delivery, Event, Evented, Fired, Goal, Held, Holder, HolderKind,
    Kind, Line, Marked, Remind, Sent, Standing,
};
use crate::folded_work::{Work, count, reset};

fn goal(id: &str) -> Goal {
    Goal {
        id: id.to_owned(),
        holder: Holder {
            kind: HolderKind::Agent,
            id: "agent".to_owned(),
        },
        kind: Kind::Goal,
        words: "keep available".to_owned(),
        deadline: None,
        active: true,
        evidence: None,
        judged_by: None,
        reminders: vec![Remind::Every { seconds: 1 }],
        responsible: "person".to_owned(),
        set_by: "person".to_owned(),
        at: 1,
    }
}

#[test]
fn goal_operation_and_entity_lookups_visit_no_history_after_restore_and_tail_writes()
-> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    for n in 0..256 {
        let id = format!("goal-{n}");
        held.hold(Line::Set(goal(&id)))?;
        held.hold(Line::Changed(Changed {
            operation: format!("change-{n}"),
            goal: id.clone(),
            change: Change::Active { active: true },
            by: "person".to_owned(),
            at: 1,
        }))?;
        held.hold(Line::Fired(Fired {
            operation: format!("firing-{n}"),
            goal: id.clone(),
            reminder: 0,
            due: 2,
            fired: 2,
            late: false,
            text: "keep available".to_owned(),
            refused: None,
            sent: vec![Sent {
                session: "session".to_owned(),
                operation: format!("delivery-{n}"),
                state: Delivery::Pending,
                words: String::new(),
                at: 2,
            }],
        }))?;
        held.hold(Line::Marked(Marked {
            operation: format!("mark-{n}"),
            goal: id,
            standing: Standing::Met,
            by: "person".to_owned(),
            words: "met".to_owned(),
            evidence: None,
            at: 3,
        }))?;
    }
    held.hold(Line::Evented(Evented {
        operation: "event".to_owned(),
        event: Event::Compaction,
        goals: Vec::new(),
        at: 3,
    }))?;
    let bytes = held.encode()?;
    let mut restored = Held::decode(&bytes)?;
    assert_eq!(restored.encode()?, bytes);
    let copied = restored.clone();
    assert!(std::sync::Arc::ptr_eq(&restored.index, &copied.index));
    restored.hold(Line::Set(goal("tail")))?;
    reset();
    assert!(restored.item("goal-255").is_some());
    assert!(restored.item("tail").is_some());
    assert!(copied.item("tail").is_none());
    assert!(restored.marked("mark-255").is_some());
    assert!(restored.changed("change-255").is_some());
    assert!(restored.kept("firing-255"));
    assert!(restored.kept("event"));
    assert!(!restored.kept("missing"));
    assert!(restored.sent("delivery-255").is_some());
    restored.hold(Line::Answered(Answered {
        operation: "delivery-255".to_owned(),
        state: Delivery::Delivered,
        words: "delivered".to_owned(),
        at: 4,
    }))?;
    assert_eq!(
        restored.sent("delivery-255").map(|sent| sent.state),
        Some(Delivery::Delivered)
    );
    assert_eq!(
        count(Work::GoalOperation),
        0,
        "goal lookup walked unrelated history"
    );
    Ok(())
}

#[test]
fn reused_delivery_and_mark_ids_keep_the_first_item_location() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Line::Set(goal("one")))?;
    held.hold(Line::Set(goal("two")))?;
    for id in ["two", "one"] {
        held.hold(Line::Fired(Fired {
            operation: format!("fire-{id}"),
            goal: id.to_owned(),
            reminder: 0,
            due: 2,
            fired: 2,
            late: false,
            text: id.to_owned(),
            refused: None,
            sent: vec![Sent {
                session: id.to_owned(),
                operation: "delivery".to_owned(),
                state: Delivery::Pending,
                words: String::new(),
                at: 2,
            }],
        }))?;
        held.hold(Line::Marked(Marked {
            operation: "mark".to_owned(),
            goal: id.to_owned(),
            standing: Standing::Met,
            by: "person".to_owned(),
            words: id.to_owned(),
            evidence: None,
            at: 3,
        }))?;
    }
    held.hold(Line::Answered(Answered {
        operation: "delivery".to_owned(),
        state: Delivery::Delivered,
        words: "kept".to_owned(),
        at: 4,
    }))?;
    assert_eq!(
        held.sent("delivery").map(|sent| sent.session.as_str()),
        Some("one")
    );
    assert_eq!(held.items[1].fired[0].sent[0].state, Delivery::Pending);
    assert_eq!(
        held.marked("mark").map(|mark| mark.goal.as_str()),
        Some("one")
    );
    let restored = Held::decode(&held.encode()?)?;
    assert_eq!(restored.sent("delivery"), held.sent("delivery"));
    assert_eq!(restored.marked("mark"), held.marked("mark"));
    Ok(())
}

#[test]
fn ten_thousand_character_words_are_kept_and_reloaded_whole() -> Result<(), Box<dyn Error>> {
    let change = Change::Words {
        words: "a".repeat(10_000),
    };
    change.check()?;
    let mut held = Held::default();
    held.hold(Line::Set(goal("long")))?;
    held.hold(Line::Changed(Changed {
        operation: "reworded".to_owned(),
        goal: "long".to_owned(),
        change: change.clone(),
        by: "person".to_owned(),
        at: 2,
    }))?;
    let restored = Held::decode(&held.encode()?)?;
    assert_eq!(
        restored.changed("reworded").map(|changed| &changed.change),
        Some(&change)
    );
    Ok(())
}
