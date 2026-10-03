//! The external login binding: an issuer and the subject it names.
//!
//! Issuer plus subject identifies one external login, and one binding names at
//! most one person (P1). The pair is compared exactly, as OIDC
//! compares an issuer: no case folding, no trailing-slash trimming and no URL
//! rewriting, because two spellings a normaliser treats as one are two issuers
//! to the provider that minted the token. A value with surrounding whitespace
//! or a control character is refused rather than trimmed, so the bytes
//! recorded are the bytes the token carried.

use crate::error::IdentityError;

/// The longest subject accepted, in bytes: `OpenID` Connect Core 1.0 section 2
/// says `sub` must not exceed 255 ASCII characters.
pub const SUBJECT_MAX_BYTES: usize = 255;

/// An issuer and subject pair, exactly as the issuer's token names them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LoginBinding {
    issuer: String,
    subject: String,
}

/// Refuse a value that is empty, padded with whitespace or carries a control character.
fn check_exact(
    value: &str,
    empty: &'static str,
    padded: &'static str,
) -> Result<(), IdentityError> {
    if value.is_empty() {
        return Err(IdentityError::BindingMalformed { reason: empty });
    }
    if value.trim() != value || value.chars().any(char::is_control) {
        return Err(IdentityError::BindingMalformed { reason: padded });
    }
    Ok(())
}

impl LoginBinding {
    /// The binding of `subject` at `issuer`, refused by name if either is not exact.
    pub fn new(issuer: &str, subject: &str) -> Result<Self, IdentityError> {
        check_exact(
            issuer,
            "the issuer is empty",
            "the issuer carries surrounding whitespace or a control character",
        )?;
        if !(issuer.starts_with("https://") || issuer.starts_with("http://")) {
            return Err(IdentityError::BindingMalformed {
                reason: "the issuer is not an http or https URL",
            });
        }
        if subject.len() > SUBJECT_MAX_BYTES {
            return Err(IdentityError::BindingMalformed {
                reason: "the subject is longer than 255 bytes",
            });
        }
        check_exact(
            subject,
            "the subject is empty",
            "the subject carries surrounding whitespace or a control character",
        )?;
        Ok(Self {
            issuer: issuer.to_owned(),
            subject: subject.to_owned(),
        })
    }

    /// The issuer, exactly as recorded.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// The subject, exactly as recorded.
    pub fn subject(&self) -> &str {
        &self.subject
    }
}
