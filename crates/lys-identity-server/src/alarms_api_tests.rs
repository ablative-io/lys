//! Named alarm refusals retain their HTTP status and explanatory words.

use super::AlarmError;
use axum::body::to_bytes;
use axum::response::IntoResponse;

#[tokio::test]
async fn alarm_errors_produce_the_documented_refusals() -> Result<(), Box<dyn std::error::Error>> {
    for (error, status, name) in [
        (AlarmError::Unknown, 404, "AlarmUnknown"),
        (
            AlarmError::Malformed("invalid acknowledgement".to_owned()),
            400,
            "AlarmMalformed",
        ),
        (
            AlarmError::Unavailable("store unavailable".to_owned()),
            503,
            "AlarmsUnavailable",
        ),
    ] {
        let reason = error.to_string();
        let response = error.into_response();
        assert_eq!(response.status().as_u16(), status);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
        assert_eq!(body["refusal"], name);
        assert_eq!(body["reason"], reason);
    }
    Ok(())
}
