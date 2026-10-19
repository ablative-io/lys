//! The public password and provider routes name each refusal without a
//! retry or anything private (moved from `sign_in_page.rs` whole so each
//! file stays under the 500-line gate; DIRECTORY-107 touched the parent).

use std::error::Error;

use serde_json::Value;

use super::{TestResult, public_sign_in};

#[tokio::test]
async fn the_public_password_route_names_refusals_without_retry_or_private_data() -> TestResult {
    let cases = [
        (403, "Forbidden", 4_102_444_800, "IssuerRefused", 503, 1),
        (400, "BadRequest", 4_102_444_800, "IssuerRefused", 503, 1),
        (401, "Unauthorized", 4_102_444_800, "SignInRefused", 401, 1),
        (401, "Forbidden", 4_102_444_800, "IssuerRefused", 503, 1),
        (
            429,
            "TooManyRequests",
            4_102_444_800,
            "SignInThrottled",
            429,
            1,
        ),
        (
            200,
            "SecondFactor",
            4_102_444_800,
            "SecondFactorUnsupported",
            403,
            1,
        ),
        (403, "Forbidden", 1, "IssuerChallengeExpired", 503, 0),
    ];
    let mut service = public_sign_in::PublicSignIn::start().await?;
    let outcome = async {
        let mut answers = Vec::with_capacity(cases.len());
        for (status, error, expires, _, _, _) in cases {
            answers.push(service.request(status, error, expires, false).await?);
        }
        let malformed = service
            .request(401, "Unauthorized", 4_102_444_800, true)
            .await?;
        Ok::<_, Box<dyn Error>>((answers, malformed))
    }
    .await;
    let cleanup = service.close().await;
    let (answers, malformed) = match (outcome, cleanup) {
        (Ok(answers), Ok(())) => answers,
        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
        (Err(error), Err(cleanup)) => {
            return Err(format!(
                "public sign-in fixture failed: {error}; cleanup failed: {cleanup}"
            )
            .into());
        }
    };
    assert_eq!(answers.len(), cases.len());
    for (seen, (status, error, _, named, wire_status, authorize)) in answers.iter().zip(cases) {
        assert_eq!(seen.status, wire_status, "{}", seen.body);
        let body: Value = serde_json::from_str(&seen.body)?;
        assert_eq!(body["refusal"], named);
        assert!(!seen.cookie);
        assert_eq!(seen.pow, 1, "one challenge, no retry");
        assert_eq!(seen.authorize, authorize);
        assert_public_refusal_privacy(seen);
        if named == "IssuerRefused" {
            assert!(seen.body.contains(&status.to_string()) && seen.body.contains(error));
            assert!(seen.log.contains("WARN") && seen.log.contains(error));
            assert!(seen.log.contains(&status.to_string()));
        } else if named == "IssuerChallengeExpired" {
            assert!(seen.body.contains("sign-in again"));
            assert!(seen.log.contains("WARN") && seen.log.contains(named));
        }
    }
    assert_eq!(malformed.status, 400, "{}", malformed.body);
    let body: Value = serde_json::from_str(&malformed.body)?;
    assert_eq!(body["refusal"], "RequestMalformed");
    assert!(!malformed.cookie);
    assert_eq!(malformed.pow, 0);
    assert_eq!(malformed.authorize, 0);
    assert_public_refusal_privacy(&malformed);
    Ok(())
}

fn assert_public_refusal_privacy(seen: &public_sign_in::Observed) {
    for sentinel in [
        "issuer-private-message-sentinel",
        "issuer-private-email-sentinel",
        "issuer-private-token-sentinel",
        "attempt-private-email-sentinel",
        "attempt-private-password-sentinel",
        "fixture-request-token",
    ] {
        assert!(
            !seen.body.contains(sentinel),
            "private data in public refusal body"
        );
        assert!(
            !seen.log.contains(sentinel),
            "private data in public refusal log"
        );
    }
}

#[tokio::test]
async fn the_public_provider_start_refuses_expiry_before_posting_to_the_provider() -> TestResult {
    let mut service = public_sign_in::PublicSignIn::start().await?;
    let outcome = async {
        let expired = service.provider_start(1).await?;
        let live = service.provider_start(4_102_444_800).await?;
        Ok::<_, Box<dyn Error>>((expired, live))
    }
    .await;
    let cleanup = service.close().await;
    let (expired, live) = match (outcome, cleanup) {
        (Ok(answers), Ok(())) => answers,
        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
        (Err(error), Err(cleanup)) => {
            return Err(format!(
                "public provider fixture failed: {error}; cleanup failed: {cleanup}"
            )
            .into());
        }
    };
    assert_eq!(expired.status, 303);
    assert_eq!(
        expired.location.as_deref(),
        Some("/#/sign-in?refused=IssuerChallengeExpired")
    );
    assert!(!expired.cookie);
    assert_eq!(expired.pow, 1, "one challenge, no retry");
    assert_eq!(expired.authorize, 0);
    assert_eq!(expired.provider_posts, 0);
    assert!(expired.log.contains("WARN") && expired.log.contains("IssuerChallengeExpired"));
    assert_public_refusal_privacy(&expired);

    assert_eq!(live.status, 303);
    assert!(live.cookie);
    assert_eq!(live.pow, 1, "one challenge, no retry");
    assert_eq!(live.authorize, 0);
    assert_eq!(live.provider_posts, 1);
    let location = reqwest::Url::parse(
        live.location
            .as_deref()
            .ok_or("live provider redirect missing")?,
    )?;
    assert_eq!(location.scheme(), "https");
    assert_eq!(location.host_str(), Some("provider.example.test"));
    assert_eq!(location.path(), "/login");
    let query: Vec<_> = location.query_pairs().collect();
    assert_eq!(query.len(), 2);
    assert!(query.iter().any(|(name, value)| name == "redirect_uri"
        && value == "http://127.0.0.1/auth/v1/providers/callback"));
    assert!(
        query
            .iter()
            .any(|(name, value)| name == "state" && value == "fixture-provider-state")
    );
    assert!(!live.log.contains("IssuerChallengeExpired"));
    assert_public_refusal_privacy(&live);
    Ok(())
}
