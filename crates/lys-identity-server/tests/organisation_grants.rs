//! A root grant rests on organisation.grant (ACCESS-009 R2): a person
//! holding it on their organisation roots a resource of it for a person of
//! it, and nobody else does; a resource rooted under one organisation is
//! refused to another while any grant on it lives; the install's own
//! collections are rooted by its root person alone; a grant is revoked by a
//! person holding organisation.grant on its organisation; and a grant is
//! passed on within its organisation alone (R3).

use std::error::Error;

use identity_contract::apps::{ok, refused, root};
use identity_contract::harness::{ADMINISTRATOR, Service};
use identity_contract::two_organisations::{Two, signed_in, two_organisations};
use lys_identity::OperationId;
use serde_json::{Value, json};

type Outcome = Result<(), Box<dyn Error>>;

/// The grant `id` as `cookie` reads it.
async fn read(service: &Service, cookie: &str, id: &Value) -> Result<Value, Box<dyn Error>> {
    let id = id.as_str().ok_or("the answer names no grant")?;
    let (status, grant) = service.get(&format!("/grants/{id}"), Some(cookie)).await?;
    if status != 200 {
        return Err(format!("reading {id}: {status} {grant}").into());
    }
    Ok(grant)
}

/// The id of the grant `cookie` sees on project `site`.
async fn on_site(service: &Service, cookie: &str, site: &str) -> Result<String, Box<dyn Error>> {
    let (status, grants) = service.get("/grants", Some(cookie)).await?;
    if status != 200 {
        return Err(format!("listing grants: {status} {grants}").into());
    }
    grants["grants"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|grant| grant["resource"]["id"] == site)
        .and_then(|grant| grant["id"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("no grant on {site}: {grants}").into())
}

fn revocation(reason: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "route": "api",
        "reason": reason,
    }))
}

fn passing_on(source: &str, recipient: &str, site: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "route": "api",
        "source": source,
        "recipient": recipient,
        "responsible": recipient,
        "resource": {"kind": "project", "id": site},
        "relation": "beta",
        "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null},
    }))
}

async fn started() -> Result<(Service, Two, [String; 3]), Box<dyn Error>> {
    let (service, two) = Service::start_with(two_organisations).await?;
    let admin = signed_in(&service, ADMINISTRATOR).await?;
    let ada = signed_in(&service, "ada").await?;
    let priya = signed_in(&service, "priya").await?;
    Ok((service, two, [admin, ada, priya]))
}

#[tokio::test]
async fn a_root_rests_on_organisation_grant_and_a_resource_is_rooted_under_one_organisation()
-> Outcome {
    let (service, two, [admin, ada, priya]) = started().await?;
    let (pat, ada_id) = (two.pat.to_string(), two.ada.to_string());

    // Priya, B's founder, roots b-new for Pat of B: the grant carries B.
    let rooted = ok(root(&service, &priya, &pat, ("project", "b-new"), "alpha").await?)?;
    let grant = read(&service, &priya, &rooted["grant"]).await?;
    assert_eq!(grant["organisation"], json!(two.b.to_string()), "{grant}");
    assert_eq!(grant["holder"], json!(pat), "{grant}");

    // Ada, of A with no organisation.grant, is refused by name; the
    // administrator, of A, cannot root a resource for Pat of B.
    refused(
        &root(&service, &ada, &pat, ("project", "b-next"), "alpha").await?,
        403,
        "RootAuthorityRefused",
    )?;
    refused(
        &root(&service, &admin, &pat, ("project", "a-new"), "alpha").await?,
        403,
        "OrganisationOutside",
    )?;

    // b-site is rooted under B while Priya's root lives: A's administrator is
    // refused it by name. Once Priya takes her root back, A roots it.
    refused(
        &root(&service, &admin, &ada_id, ("project", "b-site"), "alpha").await?,
        403,
        "RootOwnedElsewhere",
    )?;
    let b_site = on_site(&service, &priya, "b-site").await?;
    let (status, revoked) = service
        .post(
            &format!("/grants/{b_site}/revoke"),
            Some(&priya),
            &revocation("B is done with b-site")?,
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    let rooted = ok(root(&service, &admin, &ada_id, ("project", "b-site"), "alpha").await?)?;
    let grant = read(&service, &admin, &rooted["grant"]).await?;
    assert_eq!(grant["organisation"], json!(two.a.to_string()), "{grant}");

    // The install's own collections are rooted by its root person alone.
    refused(
        &root(&service, &priya, &pat, ("directory", "apps"), "alpha").await?,
        403,
        "RootAuthorityRefused",
    )?;
    ok(root(&service, &admin, &ada_id, ("directory", "apps"), "alpha").await?)?;
    Ok(())
}

#[tokio::test]
async fn a_grant_is_passed_on_within_its_organisation_and_revoked_by_a_holder_of_organisation_grant()
-> Outcome {
    let (service, two, [admin, ada, priya]) = started().await?;
    let a_site = on_site(&service, &ada, "a-site").await?;

    // Ada's root on a-site passes read on to people: Pat, of B, is refused
    // by name; the administrator, of A, is a recipient within A.
    let (status, outside) = service
        .post(
            "/grants",
            Some(&ada),
            &passing_on(&a_site, &two.pat.to_string(), "a-site")?,
        )
        .await?;
    assert_eq!(
        (status, &outside["refusal"]),
        (403, &json!("OrganisationOutside")),
        "{outside}"
    );
    let (status, passed) = service
        .post(
            "/grants",
            Some(&ada),
            &passing_on(&a_site, &two.admin.to_string(), "a-site")?,
        )
        .await?;
    assert_eq!(status, 200, "{passed}");
    let passed = passed["grant"]
        .as_str()
        .ok_or("no grant passed on")?
        .to_owned();

    // Why Ada may read a-site names the organisation her grant belongs to.
    let (status, why) = service
        .post(
            "/grants/why",
            Some(&ada),
            &json!({"route": "api", "resource": {"kind": "project", "id": "a-site"}, "action": "read"}),
        )
        .await?;
    assert_eq!(status, 200, "{why}");
    assert_eq!(why["organisation"], json!(two.a.to_string()), "{why}");

    // Priya, of B, has no standing over a grant of A: it is not hers to see.
    let (status, refused_b) = service
        .post(
            &format!("/grants/{passed}/revoke"),
            Some(&priya),
            &revocation("not B's to take back")?,
        )
        .await?;
    assert_ne!(status, 200, "{refused_b}");
    let (status, still) = service
        .get(&format!("/grants/{passed}"), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{still}");
    assert_eq!(still["standing"]["stands"], json!(true), "{still}");

    // The administrator neither issued the grant nor holds a grant it derives
    // from, and revokes it by organisation.grant on A.
    let (status, revoked) = service
        .post(
            &format!("/grants/{passed}/revoke"),
            Some(&admin),
            &revocation("A takes it back")?,
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    let (status, gone) = service
        .get(&format!("/grants/{passed}"), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{gone}");
    assert_eq!(gone["standing"]["stands"], json!(false), "{gone}");
    Ok(())
}
