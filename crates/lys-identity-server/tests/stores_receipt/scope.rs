//! Every store the service makes is counted by its receipt (DIRECTORY-107
//! R2, C777): after a boot that wrote grants, a machine and a session and a
//! clean stop, every file and folder the service made where it keeps its
//! stores is a store the receipt names, or one of the files named below as
//! no store and why.

use std::collections::BTreeSet;

use lys_identity::stores_receipt as receipt;

use super::{ADMINISTRATOR, TestResult, agent, grant, login, machine, seeded};

/// What the service keeps beside its stores that is no store, each with why.
const NOT_STORES: &[(&str, &str)] = &[
    ("install", "the receipt and acceptances themselves"),
    ("client.secret", "a credential the installer writes"),
    ("issuer.key", "the test's own sign-in issuer's key, not the service's"),
    ("grant-model.json", "the configuration the installer writes"),
    (
        "sessions.json",
        "sign-in sessions: losing one signs a person in again",
    ),
    (
        "grant-log.identity.json",
        "the grant log's identity, compared with the grant log",
    ),
];

#[tokio::test]
async fn every_store_the_service_makes_is_named_in_its_receipt() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    grant(&service, &ada, &seeded.people[1].id.to_string(), "1").await?;
    machine(&service, &ada, "Ward desk", &agent(&seeded)?).await?;
    service.restart().await?;
    let root = service.dir.path().to_path_buf();
    let written =
        receipt::read_receipt(&root.join("install").join("stores.json"))?.ok_or("no receipt")?;
    for (name, entry) in &written.stores {
        assert!(
            entry.counts_nothing() || entry.path.exists(),
            "{name} is counted at {} and nothing is there",
            entry.path.display()
        );
    }
    let counted: BTreeSet<_> = written
        .stores
        .values()
        .map(|entry| entry.path.clone())
        .collect();
    let unnamed: Vec<String> = std::fs::read_dir(&root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| !counted.contains(path))
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .filter(|name| !NOT_STORES.iter().any(|(kept, _)| kept == name))
        .collect();
    assert!(
        unnamed.is_empty(),
        "made by the service and named by no receipt entry: {unnamed:?}"
    );
    Ok(())
}
