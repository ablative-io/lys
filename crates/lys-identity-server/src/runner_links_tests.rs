use super::{Link, Links, lost};

#[test]
fn a_link_is_begun_once_while_it_is_live_and_again_after_it_lost_its_runner() {
    let links = Links::default();
    assert!(links.begin(Link::Feed, "machine-one"));
    assert!(
        !links.begin(Link::Feed, "machine-one"),
        "a live link is not begun twice"
    );
    // The other link, and the same link to another machine, stand apart.
    assert!(links.begin(Link::Grants, "machine-one"));
    assert!(links.begin(Link::Feed, "machine-two"));
    links.ended(Link::Feed, "machine-one", true);
    assert!(
        links.begin(Link::Feed, "machine-one"),
        "a link that lost its runner is begun again"
    );
    assert!(!links.begin(Link::Grants, "machine-one"));
}

#[test]
fn a_link_that_ended_on_a_refusal_stays_ended() {
    let links = Links::default();
    assert!(links.begin(Link::Feed, "machine-one"));
    links.ended(Link::Feed, "machine-one", false);
    assert!(
        !links.begin(Link::Feed, "machine-one"),
        "begun again it would end the same way"
    );
    assert!(links.begin(Link::Feed, "machine-two"));
}

#[test]
fn only_a_runner_that_was_not_there_counts_as_lost() {
    assert!(lost("runner_unreachable"));
    assert!(lost("runner_socket_unavailable"));
    for refused in [
        "BudgetsUnavailable",
        "RuntimeSessionUnknown",
        "runner_answer_unexpected",
        "grant_channel_undialled",
        "runner_request_unsigned",
    ] {
        assert!(!lost(refused), "{refused}");
    }
}

#[test]
fn a_poisoned_table_begins_nothing_more() {
    let links = std::sync::Arc::new(Links::default());
    assert!(links.begin(Link::Feed, "machine-one"));
    let held = std::sync::Arc::clone(&links);
    let poisoner = std::thread::spawn(move || {
        let _guard = held.held.lock();
        panic!("poison the links table");
    });
    assert!(poisoner.join().is_err(), "the holder panicked");
    assert!(
        !links.begin(Link::Grants, "machine-one"),
        "nothing is begun on a poisoned table"
    );
    links.ended(Link::Feed, "machine-one", true);
    assert!(
        !links.begin(Link::Feed, "machine-one"),
        "an end is not recorded on a poisoned table, so the link is not begun again"
    );
}
