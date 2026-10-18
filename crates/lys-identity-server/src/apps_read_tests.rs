#![cfg(test)]
//! A person's view of an approved app carries its address (DIRECTORY-101 R6):
//! the origin of its first sign-in redirect, so Open <app> is a link for
//! everyone the app is shown to; an app a person does not see is not shown.

use identity_contract::apps::{
    Auth, BEA, FILES, NOTES, get, login, ok, register, registered, seeded,
};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::json;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn a_person_sees_an_approved_apps_address_and_nothing_of_an_app_they_do_not_see() -> Result {
    let (service, _) = seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    registered(&service, &admin, NOTES).await?;
    register(&service, &admin, FILES).await?;
    let bea = service.sign_in(login(BEA)).await?;
    let listed = ok(get(&service, "/apps", Auth::Cookie(&bea)).await?)?;
    assert_eq!(
        listed["apps"],
        json!([{
            "id": NOTES,
            "name": format!("The {NOTES} fixture"),
            "state": "approved",
            "address": "https://app.example.test",
        }]),
        "{listed}"
    );
    Ok(())
}
