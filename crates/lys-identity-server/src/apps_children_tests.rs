//! ACCESS-006 R3: a parent's placed children are read from an index, never
//! by walking every placement, and a child's first placement is the one
//! both directions name.

use super::{By, Held, Placed, Records};

fn placed(child: &str, parent: &str, restricted: bool) -> Placed {
    Placed {
        operation: format!("place-{child}-{parent}"),
        app: "rooms".to_owned(),
        child_kind: "rooms.channel".to_owned(),
        child_id: child.to_owned(),
        parent_kind: "rooms.workspace".to_owned(),
        parent_id: parent.to_owned(),
        restricted,
        by: By::Start,
        at: 1,
    }
}

fn children<'a>(held: &'a Held, parent: &str) -> Vec<&'a str> {
    held.children("rooms.workspace", parent)
        .map(|placed| placed.child_id.as_str())
        .collect()
}

#[test]
fn a_parent_answers_its_children_in_the_order_placed() {
    let held = Held::from(Records {
        apps: Vec::new(),
        placements: vec![
            placed("b", "ward", false),
            placed("x", "elsewhere", false),
            placed("a", "ward", true),
            placed("b", "elsewhere", false),
        ],
        registrars: Vec::new(),
    });
    assert_eq!(children(&held, "ward"), ["b", "a"]);
    assert_eq!(
        children(&held, "elsewhere"),
        ["x"],
        "b's first placement is in ward, as parent() answers"
    );
    assert_eq!(
        held.parent("rooms.channel", "b")
            .map(|placed| placed.parent_id.as_str()),
        Some("ward")
    );
    assert!(children(&held, "nowhere").is_empty());
    assert!(held.children("rooms.channel", "b").next().is_none());
}
