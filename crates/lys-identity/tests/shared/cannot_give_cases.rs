//! Cannot-give decisions agree with delegation admission across the fixture matrix.

use super::*;

#[test]
fn cannot_give_fixture() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let a1 = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), a1)?;
    assert_eq!(list.items.len(), 4, "{list:?}");
    assert_eq!(seen(&list)?, base_for_agent());
    assert!(!names_grant(&list, g(1)), "G1 may be given to an agent");
    assert_eq!(marked(&list), 0, "G1 is not listed, so nothing is marked");
    assert_eq!((list.source, list.recipient), (g(1), a1));
    Ok(())
}

#[test]
fn cannot_give_person_recipient() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let person = IdentityId::Person(people.p2);
    let list = ask(&mut people, &book, g(1), person)?;
    assert_eq!(list.items.len(), 3, "{list:?}");
    assert_eq!(seen(&list)?, base_for_person());
    assert!(list.items.iter().all(|item| item.reason != SignInIdentity));
    Ok(())
}

#[test]
fn cannot_give_precedence() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let mut legs = 0;
    for recipient in [IdentityId::Agent(people.a1), IdentityId::Person(people.p2)] {
        let list = ask(&mut people, &book, g(1), recipient)?;
        let g3: Vec<CannotGiveReason> = list
            .items
            .iter()
            .filter(|item| item.subject == CannotGiveSubject::Grant(g(3)))
            .map(|item| item.reason)
            .collect();
        assert_eq!(
            g3,
            vec![LentToYou],
            "{recipient}: G3 carries lent_to_you alone"
        );
        legs += 1;
    }
    assert_eq!(legs, 2);
    Ok(())
}

#[test]
fn cannot_give_order_table() {
    let mut cases = 0;
    for mask in 1u8..128 {
        let subset: Vec<CannotGiveReason> = CannotGiveReason::ALL
            .into_iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) != 0)
            .map(|(_, reason)| reason)
            .collect();
        let reversed: Vec<CannotGiveReason> = subset.iter().rev().copied().collect();
        assert_eq!(
            CannotGiveReason::first(reversed),
            subset.first().copied(),
            "{subset:?}"
        );
        cases += 1;
    }
    assert_eq!(cases, 127);
    assert_eq!(
        CannotGiveReason::ALL.map(CannotGiveReason::name),
        [
            "sign_in_identity",
            "above_what_you_hold",
            "lent_to_you",
            "use_only",
            "people_only",
            "agents_only",
            "recipient_kind_excluded"
        ]
    );
    assert_eq!(CannotGiveReason::first([]), None);
}

#[test]
fn cannot_give_people_only() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::PeopleOnly], false)?;
    let agent = IdentityId::Agent(people.a1);
    let person = IdentityId::Person(people.p2);
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(list.items.len(), 5, "{list:?}");
    assert_eq!(
        seen(&list)?,
        vec![
            Seen::Grant(2, UseOnly),
            Seen::Grant(3, LentToYou),
            Seen::Grant(11, PeopleOnly),
            Seen::Relation("damson", AboveWhatYouHold),
            Seen::SignIn,
        ]
    );
    let list = ask(&mut people, &book, g(1), person)?;
    assert_eq!(seen(&list)?, base_for_person());
    assert!(!names_grant(&list, g(11)));
    Ok(())
}

#[test]
fn cannot_give_agents_only() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::AgentsOnly], false)?;
    let agent = IdentityId::Agent(people.a1);
    let person = IdentityId::Person(people.p2);
    let list = ask(&mut people, &book, g(1), person)?;
    assert_eq!(list.items.len(), 4, "{list:?}");
    assert_eq!(
        seen(&list)?,
        vec![
            Seen::Grant(2, UseOnly),
            Seen::Grant(3, LentToYou),
            Seen::Grant(10, AgentsOnly),
            Seen::Relation("damson", AboveWhatYouHold),
        ]
    );
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(seen(&list)?, base_for_agent());
    assert!(!names_grant(&list, g(10)));
    Ok(())
}

#[test]
fn cannot_give_in_force_only() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::NotInForce], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(list.items.len(), 3, "{list:?}");
    assert_eq!(
        seen(&list)?,
        vec![
            Seen::Grant(2, UseOnly),
            Seen::Grant(3, LentToYou),
            Seen::SignIn
        ]
    );
    for id in [5, 9, 13, 14] {
        assert!(!names_grant(&list, g(id)), "G{id} is not in force");
    }
    assert!(
        !names_relation(&list, "damson"),
        "G9 covers damson, in any standing"
    );
    // Before G9's end passes it is in force, and may be given, so it is still unlisted.
    let request = CannotGiveRequest {
        caller: IdentityId::Person(people.p1),
        route: Route::Browser,
        source: g(1),
        recipient: IdentityId::Agent(people.a1),
    };
    let directory = people.projection()?;
    let early = cannot_give(&book.grants, directory, &book.model, &request, T0 + 1)?;
    assert_eq!(seen(&early)?, seen(&list)?);
    Ok(())
}

#[test]
fn cannot_give_source_mark() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(2), agent)?;
    assert_eq!(seen(&list)?, base_for_agent());
    assert_eq!(marked(&list), 1);
    assert!(list.items[0].source, "G2's item is the source");
    assert_eq!(list.items[0].subject, CannotGiveSubject::Grant(g(2)));
    Ok(())
}

#[test]
fn cannot_give_service_account() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::ServiceAccount], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(list.items.len(), 5, "{list:?}");
    assert_eq!(
        seen(&list)?,
        vec![
            Seen::Grant(2, UseOnly),
            Seen::Grant(3, LentToYou),
            Seen::Account(7, UseOnly),
            Seen::Relation("damson", AboveWhatYouHold),
            Seen::SignIn,
        ]
    );
    let naming_g7 = list
        .items
        .iter()
        .filter(|item| named(item) == Some(g(7)))
        .count();
    assert_eq!(naming_g7, 1, "SA1 appears once");
    assert!(
        !list.items.contains(&CannotGiveItem {
            subject: CannotGiveSubject::Grant(g(7)),
            reason: UseOnly,
            source: false,
        }),
        "no grant item names G7"
    );
    Ok(())
}

#[test]
fn cannot_give_service_account_givable() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[Extra::ServiceAccountGivable], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), agent)?;
    assert_eq!(seen(&list)?, base_for_agent());
    assert!(!names_grant(&list, g(8)));
    Ok(())
}

#[test]
fn cannot_give_no_rank() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, true, &[], false)?;
    let agent = IdentityId::Agent(people.a1);
    let list = ask(&mut people, &book, g(1), agent)?;
    let relations: Vec<Seen> = seen(&list)?
        .into_iter()
        .filter(|item| matches!(item, Seen::Relation(..)))
        .collect();
    assert_eq!(relations, vec![Seen::Relation("alder", AboveWhatYouHold)]);
    Ok(())
}

#[test]
fn cannot_give_admission_agrees() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let (holder, other, agent) = (people.p1, people.p2, people.a1);
    let cases = [(g(1), "alder"), (g(2), "birch"), (g(3), "cedar")];
    let (mut legs, mut permitted, mut refused) = (0, 0, 0);
    for (grant, relation) in cases {
        for (recipient, responsible) in [
            (IdentityId::Agent(agent), holder),
            (IdentityId::Person(other), other),
        ] {
            let list = ask(&mut people, &book, g(1), recipient)?;
            let request = DelegateRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(holder),
                route: Route::Api,
                source: grant,
                recipient,
                responsible,
                resource: project("p")?,
                relation: Relation::new(relation)?,
                pass_on: PassOn::UseOnly,
                window: Window::new(T0, None)?,
            };
            let directory = people.projection()?;
            let admitted =
                judge_delegation(&book.grants, directory, &book.model, &request, AT, g(99)).is_ok();
            assert_eq!(
                !names_grant(&list, grant),
                admitted,
                "G{} to {recipient}",
                grant_number(grant)?
            );
            if admitted {
                permitted += 1;
            } else {
                refused += 1;
            }
            legs += 1;
        }
    }
    assert_eq!((legs, permitted, refused), (6, 2, 4));
    Ok(())
}

#[test]
fn cannot_give_deterministic() -> TestResult {
    let mut people = People::new()?;
    let forward = fixture(&people, false, &[], false)?;
    let reversed = fixture(&people, false, &[], true)?;
    let a1 = IdentityId::Agent(people.a1);
    let first = format!("{:?}", ask(&mut people, &forward, g(1), a1)?);
    let second = format!("{:?}", ask(&mut people, &reversed, g(1), a1)?);
    assert_eq!(first.as_bytes(), second.as_bytes());
    Ok(())
}

#[test]
fn cannot_give_refuses_a_source_or_recipient_it_cannot_answer_for() -> TestResult {
    let mut people = People::new()?;
    let book = fixture(&people, false, &[], false)?;
    let a1 = IdentityId::Agent(people.a1);
    let mut refusals = 0;
    for source in [g(0), g(42)] {
        assert!(ask(&mut people, &book, source, a1).is_err(), "{source}");
        refusals += 1;
    }
    let unknown = IdentityId::Agent(AgentId::generate()?);
    assert!(ask(&mut people, &book, g(1), unknown).is_err());
    refusals += 1;
    assert_eq!(refusals, 3);
    Ok(())
}
