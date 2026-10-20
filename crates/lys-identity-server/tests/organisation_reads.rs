//! Every people, identities, teams and grants read answers within the
//! caller's organisation, with no special case for the install's own
//! (DIRECTORY-109 R4): two organisations, A founded by the administrator
//! and B by Priya, each with two people and a root grant; A's caller is
//! answered A's records and B's caller B's, and the other's are absent,
//! exactly as a record the directory does not hold.

use std::error::Error;

use identity_contract::harness::{ADMINISTRATOR, Service};
use identity_contract::two_organisations::{ids, signed_in, two_organisations};
use lys_identity::{OperationId, PersonId};
use serde_json::json;

type Outcome = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn every_read_answers_the_callers_organisation_and_the_other_as_absent() -> Outcome {
    let (service, two) = Service::start_with(two_organisations).await?;
    let admin = signed_in(&service, ADMINISTRATOR).await?;
    let ada = signed_in(&service, "ada").await?;
    let priya = signed_in(&service, "priya").await?;
    let (a, b) = (two.a.to_string(), two.b.to_string());

    // GET /me names the caller's organisation, id and name.
    let (status, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{me}");
    assert_eq!(me["organisation"]["id"], a, "{me}");
    assert_eq!(me["organisation"]["display_name"], "Organisation A");
    let (_, me) = service.get("/me", Some(&priya)).await?;
    assert_eq!(me["organisation"]["id"], b, "{me}");
    assert_eq!(me["organisation"]["display_name"], "Organisation B");

    // GET /directory/people: the administrator's wider view is A's.
    let (status, people) = service.get("/directory/people", Some(&admin)).await?;
    assert_eq!(status, 200, "{people}");
    let listed = ids(&people["people"], "id");
    assert!(listed.contains(&two.admin.to_string()) && listed.contains(&two.ada.to_string()));
    assert!(
        !listed.contains(&two.priya.to_string()) && !listed.contains(&two.pat.to_string()),
        "B's people are absent from A's view: {people}"
    );
    let (status, paged) = service
        .get("/directory/people?limit=10", Some(&admin))
        .await?;
    assert_eq!(status, 200, "{paged}");
    assert_eq!(ids(&paged["people"], "id").len(), 2, "{paged}");
    assert_eq!(
        paged["total"], 2,
        "the total counts A's people alone: {paged}"
    );

    // GET /identities and /identities/{id}: B's are absent, as an unknown id is.
    let (status, identities) = service.get("/identities", Some(&admin)).await?;
    assert_eq!(status, 200, "{identities}");
    let listed = ids(&identities["identities"], "id");
    assert!(
        listed.contains(&two.ada.to_string()) && listed.contains(&a),
        "{identities}"
    );
    assert!(
        !listed.contains(&two.priya.to_string()) && !listed.contains(&b),
        "{identities}"
    );
    let (absent_status, absent) = service
        .get(&format!("/identities/{}", two.priya), Some(&admin))
        .await?;
    let (unknown_status, unknown) = service
        .get(
            &format!("/identities/{}", PersonId::generate()?),
            Some(&admin),
        )
        .await?;
    assert_eq!(absent_status, unknown_status, "{absent} vs {unknown}");
    assert_eq!(
        absent["refusal"], unknown["refusal"],
        "{absent} vs {unknown}"
    );
    assert_eq!(absent_status, 404, "{absent}");

    // GET /teams: a team of A's owner is A's; B's caller reads none.
    let (status, created) = service
        .post(
            "/teams",
            Some(&admin),
            &json!({"operation": OperationId::generate()?.to_string(), "name": "A's team"}),
        )
        .await?;
    assert_eq!(status, 200, "{created}");
    let (status, teams) = service.get("/teams", Some(&admin)).await?;
    assert_eq!(status, 200, "{teams}");
    assert_eq!(teams["teams"].as_array().map(Vec::len), Some(1), "{teams}");
    let (status, teams) = service.get("/teams", Some(&priya)).await?;
    assert_eq!(status, 200, "{teams}");
    assert_eq!(teams["teams"], json!([]), "B's caller reads no team of A");

    // GET /grants: every grant listed is the caller's organisation's.
    for (cookie, organisation, site) in [
        (&ada, &a, "a-site"),
        (&priya, &b, "b-site"),
        (&admin, &a, "a-site"),
    ] {
        let (status, grants) = service.get("/grants", Some(cookie)).await?;
        assert_eq!(status, 200, "{grants}");
        let listed = grants["grants"].as_array().ok_or("no grants")?;
        assert!(!listed.is_empty(), "{grants}");
        assert!(
            listed
                .iter()
                .all(|grant| grant["organisation"] == json!(organisation)),
            "{grants}"
        );
        assert!(
            listed
                .iter()
                .any(|grant| grant["resource"]["id"] == json!(site)),
            "{grants}"
        );
    }

    // POST /grants/who and /grants/reach: the administrator, of A, reaches
    // nobody on B's site; B's holder is absent, not refused.
    let who = |site: &str| {
        json!({"route": "browser", "resource": {"kind": "project", "id": site},
            "action": "read", "page_size": 10, "after": null})
    };
    let check = json!({"route": "browser", "resource": {"kind": "project", "id": "a-site"}, "action": "read"});
    let (status, checked) = service.post("/grants/check", Some(&ada), &check).await?;
    assert_eq!(status, 200, "Ada's own root on a-site: {checked}");
    let (status, holders) = service
        .post("/grants/who", Some(&admin), &who("a-site"))
        .await?;
    assert_eq!(status, 200, "{holders}");
    assert_eq!(
        ids(&holders["holders"], "holder"),
        vec![two.ada.to_string()],
        "{holders}"
    );
    let (status, holders) = service
        .post("/grants/who", Some(&admin), &who("b-site"))
        .await?;
    assert_eq!(status, 200, "{holders}");
    assert_eq!(holders["holders"], json!([]), "{holders}");
    let reach = json!({"route": "browser", "resources": [
        {"kind": "project", "id": "a-site", "actions": ["read"]},
        {"kind": "project", "id": "b-site", "actions": ["read"]}]});
    let (status, reached) = service.post("/grants/reach", Some(&admin), &reach).await?;
    assert_eq!(status, 200, "{reached}");
    assert_eq!(
        ids(&reached["resources"][0]["holders"], "holder"),
        vec![two.ada.to_string()],
        "{reached}"
    );
    assert_eq!(reached["resources"][1]["holders"], json!([]), "{reached}");

    // POST /grants/which: a person's question is partitioned by their organisation.
    let which = |subject: PersonId| json!({"subject": subject.to_string(), "kind": "project", "action": "read", "page_size": 10});
    let (status, page) = service
        .post("/grants/which", Some(&admin), &which(two.ada))
        .await?;
    assert_eq!(status, 200, "{page}");
    assert_eq!(page["ids"], json!(["a-site"]), "{page}");
    let (status, page) = service
        .post("/grants/which", Some(&admin), &which(two.priya))
        .await?;
    assert_eq!(status, 200, "{page}");
    assert_eq!(
        page["ids"],
        json!([]),
        "B's subject is answered no ids: {page}"
    );
    Ok(())
}
