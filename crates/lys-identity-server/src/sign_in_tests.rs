#![cfg(test)]
//! The issuer's page template, the proof of work and the cookie jar, each
//! checked against a value written apart from the code that reads it.

use std::error::Error;

use sha2::{Digest, Sha256};

use super::{Jar, leading_zero_bits, solve, template};

#[test]
fn the_request_token_is_read_from_its_template_and_nothing_else() {
    let page = concat!(
        "<html><body><div class=\"hidden\" aria-hidden=\"true\">",
        "<template id=\"tpl_client_name\">Lys</template>",
        "<template id=\"tpl_csrf_token\">Qx7aZ09</template>",
        "</div></body></html>"
    );
    assert_eq!(template(page, "tpl_csrf_token"), Some("Qx7aZ09"));
    assert_eq!(template(page, "tpl_client_name"), Some("Lys"));
    assert_eq!(template(page, "tpl_login_action"), None);
    assert_eq!(
        template("<template id=\"tpl_csrf_token\">", "tpl_csrf_token"),
        None
    );
}

#[test]
fn zero_bits_are_counted_from_the_first_byte() {
    assert_eq!(leading_zero_bits(&[0xff]), 0);
    assert_eq!(leading_zero_bits(&[0x00, 0x80]), 8);
    assert_eq!(leading_zero_bits(&[0x00, 0x00, 0x0f]), 20);
    assert_eq!(leading_zero_bits(&[0x00, 0x00]), 16);
}

#[test]
fn an_answer_hashes_to_the_difficulty_asked_and_extends_the_challenge() -> Result<(), Box<dyn Error>>
{
    let challenge = "1:12:4102444800:c2FsdHNhbHRzYWx0:Y2hhbGxlbmdlY2hhbGxlbmdlY2hhbGxlbmdlY2hhbGw:";
    let answer = solve(challenge)?;
    let counter = answer
        .strip_prefix(challenge)
        .ok_or("the answer extends the challenge")?;
    assert!(!counter.is_empty());
    assert!(
        counter.bytes().all(|byte| byte.is_ascii_digit()),
        "{counter}"
    );
    let hash = Sha256::digest(answer.as_bytes());
    assert_eq!(hash[0], 0, "the first eight bits are zero");
    assert_eq!(hash[1] & 0xf0, 0, "the next four bits are zero");
    Ok(())
}

#[test]
fn a_challenge_of_another_version_or_difficulty_is_refused() {
    let refused = [
        "2:12:4102444800:salt:challenge:",
        "1:09:4102444800:salt:challenge:",
        "1:xx:4102444800:salt:challenge:",
        "1:12:4102444800:salt:challenge",
        "",
    ];
    for challenge in refused {
        let refusal = solve(challenge).err().map(|error| error.name());
        assert_eq!(refusal.as_deref(), Some("SignInFailed"), "{challenge}");
    }
    assert_eq!(refused.len(), 5);
}

#[test]
fn the_jar_keeps_the_latest_value_of_each_cookie_and_drops_a_cleared_one() {
    let mut headers = reqwest::header::HeaderMap::new();
    for value in [
        "RauthySession=first; Path=/; HttpOnly",
        "rauthy-bid=browser; Path=/",
        "RauthySession=second; Path=/",
    ] {
        headers.append(
            reqwest::header::SET_COOKIE,
            reqwest::header::HeaderValue::from_static(value),
        );
    }
    let mut jar = Jar::default();
    jar.keep(&headers);
    assert_eq!(jar.header(), "rauthy-bid=browser; RauthySession=second");
    let mut cleared = reqwest::header::HeaderMap::new();
    cleared.append(
        reqwest::header::SET_COOKIE,
        reqwest::header::HeaderValue::from_static("rauthy-bid=; Max-Age=0"),
    );
    jar.keep(&cleared);
    assert_eq!(jar.header(), "RauthySession=second");
}

#[test]
fn the_connection_peer_is_the_address_and_no_peer_is_refused() -> Result<(), Box<dyn Error>> {
    let mut extensions = axum::http::Extensions::new();
    assert!(super::person_address(&extensions).is_err());
    extensions.insert(axum::extract::ConnectInfo(
        "192.0.2.1:1234".parse::<std::net::SocketAddr>()?,
    ));
    assert_eq!(
        super::person_address(&extensions)?,
        "192.0.2.1".parse::<std::net::IpAddr>()?
    );
    Ok(())
}

#[test]
fn a_difficulty_nineteen_counter_search_allocates_nothing() -> Result<(), Box<dyn Error>> {
    let challenge = "1:19:4102444800:salt:challenge:";
    let mut answer = None;
    let allocations = allocation_counter::measure(|| {
        answer = Some(super::solve_counter(challenge));
    });
    let counter = answer.ok_or("the search ran")??;
    let hash = Sha256::digest(format!("{challenge}{counter}").as_bytes());
    assert!(leading_zero_bits(&hash) >= 19);
    assert_eq!(allocations.count_total, 0, "{allocations:?}");
    Ok(())
}

#[test]
fn counter_search_matches_known_answers_without_allocating() -> Result<(), Box<dyn Error>> {
    for (challenge, expected) in [
        ("1:10:4102444800:salt:challenge:", 415),
        (
            "1:10:4102444800:abcdefghijklmnop:abcdefghijklmnopqrstuvwxyz0123456789abcdefghijk:",
            48,
        ),
    ] {
        let mut answer = None;
        let allocations = allocation_counter::measure(|| {
            answer = Some(super::solve_counter(challenge));
        });
        let counter = answer.ok_or("the counter search ran")??;
        assert_eq!(counter, expected);
        assert_eq!(allocations.count_total, 0, "{allocations:?}");
        let expected_answer = format!("{challenge}{expected}");
        assert_eq!(solve(challenge)?, expected_answer);
        assert!(leading_zero_bits(&Sha256::digest(expected_answer.as_bytes())) >= 10);
    }
    Ok(())
}

#[test]
fn an_allocating_counter_search_proves_the_measurement_detects_it() -> Result<(), Box<dyn Error>> {
    let challenge = "1:10:4102444800:salt:challenge:";
    let mut answer = None;
    let allocations = allocation_counter::measure(|| {
        for counter in 0_u64..=415 {
            let candidate = format!("{challenge}{counter}");
            if leading_zero_bits(&Sha256::digest(candidate.as_bytes())) >= 10 {
                answer = Some(counter);
                break;
            }
        }
    });
    assert_eq!(answer.ok_or("the allocating search answered")?, 415);
    assert!(allocations.count_total > 0, "{allocations:?}");
    Ok(())
}

#[tokio::test]
async fn a_forbidden_answer_at_the_challenge_expiry_names_the_expiry() -> Result<(), Box<dyn Error>>
{
    for checked_at in [100, 101] {
        let answer = forbidden_answer()?;
        let refusal = super::accepted(answer, Some(100), checked_at)
            .await
            .err()
            .ok_or("the issuer refused")?;
        assert_eq!(refusal.name(), "IssuerChallengeExpired");
        assert!(!refusal.to_string().contains("private-sentinel"));
    }
    Ok(())
}

fn forbidden_answer() -> Result<reqwest::Response, axum::http::Error> {
    Ok(reqwest::Response::from(
        axum::http::Response::builder()
            .status(403)
            .body(reqwest::Body::from(
                r#"{"error":"Forbidden","message":"private-sentinel"}"#,
            ))?,
    ))
}

#[tokio::test]
async fn a_forbidden_answer_one_second_before_expiry_names_its_cause() -> Result<(), Box<dyn Error>>
{
    for expires in [Some(100), None] {
        let refusal = super::accepted(forbidden_answer()?, expires, 99)
            .await
            .err()
            .ok_or("the issuer refused")?;
        assert!(
            matches!(refusal, crate::error::ServerError::IssuerRefused { status: 403, ref error } if error == "Forbidden")
        );
        assert!(!refusal.to_string().contains("private-sentinel"));
    }
    Ok(())
}

#[tokio::test]
async fn only_an_unauthorized_error_word_names_a_credential_refusal() -> Result<(), Box<dyn Error>>
{
    use crate::error::ServerError;
    for (status, body, name, word) in [
        (401, r#"{"error":"Unauthorized"}"#, "SignInRefused", None),
        (
            401,
            r#"{"error":"Forbidden"}"#,
            "IssuerRefused",
            Some("Forbidden"),
        ),
        (401, "not-json", "IssuerRefused", Some("unreadable")),
        (
            400,
            r#"{"error":false}"#,
            "IssuerRefused",
            Some("unreadable"),
        ),
        (
            400,
            r#"{"message":"private-sentinel"}"#,
            "IssuerRefused",
            Some("unreadable"),
        ),
        (429, "not-json", "SignInThrottled", None),
        (200, "not-json", "SecondFactorUnsupported", None),
    ] {
        let answer = reqwest::Response::from(
            axum::http::Response::builder()
                .status(status)
                .body(reqwest::Body::from(body))?,
        );
        let refusal = super::accepted(answer, Some(100), 99)
            .await
            .err()
            .ok_or("the issuer refused")?;
        assert_eq!(refusal.name(), name);
        assert!(!refusal.to_string().contains("private-sentinel"));
        if let Some(word) = word {
            assert!(
                matches!(refusal, ServerError::IssuerRefused { status: actual, ref error }
                if actual == status && error == word)
            );
        }
    }
    Ok(())
}
