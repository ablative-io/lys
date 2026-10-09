#![cfg(test)]
//! Products use the same refusal bytes and authorization vectors.

use lys_pass::{Decision, KeySet, Refusal, VerifiedPass, conformance};

#[test]
fn refusal_bytes_are_one_shape() -> Result<(), Box<dyn std::error::Error>> {
    let expected = include_str!("../fixtures/refusals/missing-grant.json").trim();
    let refusal = Refusal::new(
        "sample",
        "sample.file",
        "restricted",
        "read",
        "https://issuer.example",
    )?;
    assert_eq!(serde_json::to_string(&refusal)?, expected);
    assert_eq!(serde_json::from_str::<Refusal>(expected)?, refusal);
    Ok(())
}

#[test]
fn shipped_conformance_vectors_cover_authorization_and_refusal() {
    assert_eq!(conformance::cases().len(), 10);
    assert!(
        conformance::cases()
            .iter()
            .any(|case| case.name == "deliberate-unreachable")
    );
    assert!(
        conformance::cases()
            .iter()
            .any(|case| case.name == "restricted-child")
    );
}

#[test]
fn consumer_gate_names_all_missing_predicates() {
    let report = conformance::run(|case| {
        Ok::<_, std::convert::Infallible>(conformance::Outcome::Allowed(case.name.to_owned()))
    });
    assert_eq!(report.run, 10);
    assert_eq!(report.passed, 0);
    assert_eq!(report.failures.len(), 10);
    assert!(report.require_conformant().is_err());
}

/// What the crate's own hot-route adapter answers for one vector: verify the
/// pass offline, evaluate the exact target, and refuse anything but an
/// outright right in the one refusal shape. A held right on a hot route is
/// refused, never drafted.
fn hot_route(
    case: &conformance::Case,
    keys: &KeySet,
) -> Result<conformance::Outcome, lys_pass::Error> {
    use conformance::Outcome;
    let Some(token) = case.token else {
        return Err(lys_pass::Error::Invalid("a live vector is observed live"));
    };
    let pass = match VerifiedPass::verify(
        token,
        keys,
        conformance::ISSUER,
        conformance::AUDIENCE,
        conformance::NOW,
    ) {
        Ok(pass) => pass,
        Err(error) => return Ok(Outcome::NamedRefusal(error.name().to_owned())),
    };
    match pass.evaluate(case.kind, case.id, case.action, conformance::NOW) {
        Ok(Decision::Allowed { grant }) => Ok(Outcome::Allowed(grant.to_owned())),
        Ok(Decision::Held { .. } | Decision::Refused) => {
            let refusal = Refusal::new(
                conformance::AUDIENCE,
                case.kind,
                case.id,
                case.action,
                conformance::ISSUER,
            )?;
            Ok(Outcome::PermissionRefusal(
                serde_json::to_string(&refusal).map_err(lys_pass::Error::Json)?,
            ))
        }
        Err(error) => Ok(Outcome::NamedRefusal(error.name().to_owned())),
    }
}

/// ACCESS-003 R3: every shipped vector runs green through the crate's own
/// verify, evaluate and refusal, and the deliberate route asks a Lys that
/// is not there and is refused "Lys could not be asked", never answered
/// from the pass.
#[tokio::test]
async fn every_shipped_vector_is_green_through_the_crates_own_observer()
-> Result<(), Box<dyn std::error::Error>> {
    let keys = KeySet::from_json(conformance::KEYS)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let gone = listener.local_addr()?;
    drop(listener);
    let client = lys_pass::Client::new(
        reqwest::Client::new(),
        url::Url::parse(&format!("http://{gone}/"))?,
    )?;
    let mut live = std::collections::BTreeMap::new();
    for case in conformance::cases()
        .iter()
        .filter(|case| case.token.is_none())
    {
        let target = lys_pass::Target::new(case.kind, case.id, case.action)?;
        let asked = client.check_batch("pass", "holder", &[target], None).await;
        let outcome = match asked {
            Ok(answer) => conformance::Outcome::Allowed(format!("{answer:?}")),
            Err(error) => conformance::Outcome::NamedRefusal(error.name().to_owned()),
        };
        live.insert(case.name, outcome);
    }
    let report = conformance::run(|case| match case.token {
        Some(_) => hot_route(case, &keys).map_err(|error| error.to_string()),
        None => live
            .get(case.name)
            .cloned()
            .ok_or_else(|| format!("{} was not observed live", case.name)),
    });
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert_eq!((report.run, report.passed), (10, 10));
    report.require_conformant()?;
    Ok(())
}
