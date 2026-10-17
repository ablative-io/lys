//! The one line said for every refused person sign-in (DIRECTORY-099 R2),
//! on Lys's own sign-in route and on the provider callback, so an
//! administrator asked why someone could not sign in reads it from the log.
//!
//! The line is a WARN event through the identity server's WARN sink
//! (`warn_lines`), which the install keeps as identity.log: the refusal's
//! name, the route and the handle tried. The handle is the text tried,
//! trimmed and lowercased, only when `accounts::check_email` admits it, so
//! no space, newline or control character enters a line and a password typed
//! into the email box is never written; otherwise it is `none` for an empty
//! field and `not-an-email` for anything else. The password, the issuer's
//! body and any token are never written.

use crate::error::ServerError;

/// The route Lys's own sign-in screen posts to.
pub(crate) const SIGN_IN_ROUTE: &str = "POST /api/sign-in";

/// The route a provider sends the person back to.
pub(crate) const CALLBACK_ROUTE: &str = "GET /auth/v1/providers/callback";

/// The route a provider sign-in starts at.
pub(crate) const PROVIDER_START_ROUTE: &str = "GET /api/sign-in/providers/{id}";

/// What the line names as the handle tried.
fn handle(tried: &str) -> String {
    let tried = tried.trim().to_lowercase();
    if tried.is_empty() {
        "none".to_owned()
    } else if crate::accounts::check_email(&tried).is_ok() {
        tried
    } else {
        "not-an-email".to_owned()
    }
}

/// Say that a person's sign-in on `route`, trying `tried`, was refused as
/// `refusal`.
pub(crate) fn refused(route: &'static str, tried: &str, refusal: &ServerError) {
    let handle = handle(tried);
    let refusal = refusal.name();
    tracing::warn!(
        refusal = refusal.as_str(),
        route,
        handle = handle.as_str(),
        "a person's sign-in was refused"
    );
}

/// `answer`, unchanged, having said the line when it is a refusal.
pub(crate) fn said<T>(
    route: &'static str,
    tried: &str,
    answer: Result<T, ServerError>,
) -> Result<T, ServerError> {
    if let Err(refusal) = &answer {
        refused(route, tried, refusal);
    }
    answer
}

/// `answer` of POST /api/sign-in to a person who tried `tried`, unchanged,
/// having said the line when it is a refusal.
pub(crate) fn signing_in<T>(tried: &str, answer: Result<T, ServerError>) -> Result<T, ServerError> {
    said(SIGN_IN_ROUTE, tried, answer)
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_a_well_formed_email_is_named_and_then_lowercased() {
        for (tried, named) in [
            (" Grace@Example.TEST ", "grace@example.test"),
            ("", "none"),
            ("   ", "none"),
            ("Not-The-Password-1", "not-an-email"),
            ("grace@example.test\nWARN forged", "not-an-email"),
            ("grace @example.test", "not-an-email"),
        ] {
            assert_eq!(super::handle(tried), named, "{tried:?}");
        }
    }

    #[test]
    fn the_reason_is_the_refusals_words_on_one_line() {
        let refusal = crate::error::ServerError::SignInFailed {
            reason: "the issuer\nanswered\r\u{7}twice".to_owned(),
        };
        assert_eq!(
            super::words(&refusal),
            "SignInFailed: the issuer answered  twice"
        );
    }
}
