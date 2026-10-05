#![cfg(test)]
//! An app's sign-in settings in the apps' record: kept beside the approval
//! under its operation, changed under an operation of their own, the latest
//! in force; refused on an app that is not approved, and an address list
//! judged as a registration's is, with an empty list refused by name.

use std::error::Error;

use serde_json::json;

use super::redirects;
use crate::apps_error::AppError;
use crate::apps_state::{Approved, By, Client, Held, Line, Registered, SignInSet, Standing};

type Outcome = Result<(), Box<dyn Error>>;

const APP: &str = "fixture_notes";
const FIRST: &str = "https://app.example.test/signed-in";
const SECOND: &str = "https://app.example.test/elsewhere";

fn registered() -> Result<Held, Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Line::Registered(Registered {
        operation: "register".to_owned(),
        app: APP.to_owned(),
        name: "The notes fixture".to_owned(),
        redirects: vec![FIRST.to_owned()],
        schema: json!({"kinds": {}}),
        service_account: None,
        by: By::Start,
        at: 1,
    }))?;
    Ok(held)
}

fn set(operation: &str, redirects: &[&str], profile: bool, at: u64) -> Line {
    Line::SignInSet(SignInSet {
        operation: operation.to_owned(),
        app: APP.to_owned(),
        redirects: redirects
            .iter()
            .map(|address| (*address).to_owned())
            .collect(),
        profile,
        by: By::Start,
        at,
    })
}

fn approved() -> Result<Held, Box<dyn Error>> {
    let mut held = registered()?;
    held.hold(Line::Approved(Approved {
        operation: "approve".to_owned(),
        app: APP.to_owned(),
        client: Client {
            client_id: APP.to_owned(),
            secret_sha256: "00".repeat(32),
        },
        binding: None,
        by: By::Start,
        at: 2,
    }))?;
    held.hold(set("approve", &[FIRST], false, 2))?;
    Ok(held)
}

#[test]
fn the_settings_beside_an_approval_share_its_operation_and_are_kept_once() -> Outcome {
    let mut held = approved()?;
    let app = held.app(APP).ok_or("no app")?;
    assert_eq!(app.standing(), Standing::Approved);
    let settings = app.sign_in.as_ref().ok_or("no settings")?;
    assert_eq!(settings.redirects, vec![FIRST.to_owned()]);
    assert!(!settings.profile);
    assert!(
        matches!(held.operation("approve"), Some(Line::Approved(_))),
        "the approval's operation still answers the approval"
    );
    let again = held.hold(set("approve", &[SECOND], true, 3));
    assert!(
        again.is_err(),
        "a second line beside the approval is refused"
    );
    let app = held.app(APP).ok_or("no app")?;
    assert_eq!(
        app.sign_in
            .as_ref()
            .map(|settings| settings.redirects.clone()),
        Some(vec![FIRST.to_owned()])
    );
    Ok(())
}

#[test]
fn a_later_change_is_its_own_line_and_the_latest_is_in_force() -> Outcome {
    let mut held = approved()?;
    held.hold(set("change-1", &[SECOND], true, 3))?;
    let app = held.app(APP).ok_or("no app")?;
    let settings = app.sign_in.as_ref().ok_or("no settings")?;
    assert_eq!(settings.redirects, vec![SECOND.to_owned()]);
    assert!(settings.profile);
    assert_eq!(settings.operation, "change-1");
    assert!(matches!(
        held.operation("change-1"),
        Some(Line::SignInSet(_))
    ));
    held.hold(set("change-2", &[SECOND], false, 4))?;
    let app = held.app(APP).ok_or("no app")?;
    assert!(!app.sign_in.as_ref().ok_or("no settings")?.profile);
    assert_eq!(app.history.len(), 4, "approval, its settings, two changes");
    Ok(())
}

#[test]
fn the_settings_are_refused_on_an_app_that_is_not_approved() -> Outcome {
    let mut pending = registered()?;
    assert!(
        pending.hold(set("change", &[FIRST], false, 2)).is_err(),
        "a pending app has no sign-in settings"
    );
    assert!(pending.app(APP).ok_or("no app")?.sign_in.is_none());
    let mut other_operation = registered()?;
    assert!(
        other_operation
            .hold(set("approve", &[FIRST], false, 2))
            .is_err(),
        "settings under an approval's operation with no approval kept are refused"
    );
    Ok(())
}

#[test]
fn an_address_list_is_judged_as_a_registrations_and_an_empty_one_by_name() -> Outcome {
    let kept = redirects(
        APP,
        &[FIRST.to_owned(), "http://localhost:3000/back".to_owned()],
    )?;
    assert_eq!(kept.len(), 2);
    let empty = redirects(APP, &[]);
    assert!(
        matches!(&empty, Err(AppError::NoRedirect { app }) if app == APP),
        "{empty:?}"
    );
    let words = empty.err().ok_or("no refusal")?.to_string();
    assert!(words.starts_with("redirect_invalid: "), "{words}");
    assert!(words.contains("can sign nobody in"), "{words}");
    for address in [
        "http://product.example.test/back",
        "https://product.example.test/back#fragment",
        "not an address",
    ] {
        let refused = redirects(APP, &[FIRST.to_owned(), address.to_owned()]);
        assert!(
            matches!(&refused, Err(AppError::RedirectInvalid { address: named, .. }) if named == address),
            "{refused:?}"
        );
        let registration = crate::apps_api::redirect(address);
        assert_eq!(
            refused.err().map(|error| error.to_string()),
            registration.err().map(|error| error.to_string()),
            "the settings refuse exactly as a registration refuses"
        );
    }
    Ok(())
}
