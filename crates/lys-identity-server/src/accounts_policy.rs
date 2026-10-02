//! Lys's password policy and the shape of an email address, checked here
//! before anything is sent to the issuer, so a person is told in Lys's words.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::refused;
use crate::error::ServerError;

/// Lys's password policy, as the service's configuration states it and the
/// install wrote it to the issuer. A length is counted as the issuer counts
/// it, in bytes of UTF-8, and each kind of character as the issuer sorts it,
/// so the check here and the issuer's never disagree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordPolicy {
    /// The fewest characters a password has.
    pub length_min: u16,
    /// The most characters a password has.
    pub length_max: u16,
    /// The fewest lower-case letters, when any are asked for.
    #[serde(default)]
    pub lower_case: Option<u16>,
    /// The fewest upper-case letters, when any are asked for.
    #[serde(default)]
    pub upper_case: Option<u16>,
    /// The fewest digits, when any are asked for.
    #[serde(default)]
    pub digits: Option<u16>,
    /// The fewest characters that are neither letters nor digits, when any
    /// are asked for.
    #[serde(default)]
    pub special: Option<u16>,
    /// How many of a person's last passwords a new one may not be, when any
    /// are refused.
    #[serde(default)]
    pub not_recently_used: Option<u16>,
}

/// Each kind of character a policy may ask for: its field, its words, and
/// how many of it `password` holds.
fn kinds(policy: &PasswordPolicy, password: &str) -> [(Option<u16>, &'static str, usize); 4] {
    let (mut lower, mut upper, mut digits, mut special) = (0, 0, 0, 0);
    for c in password.chars() {
        if c.is_lowercase() {
            lower += 1;
        } else if c.is_uppercase() {
            upper += 1;
        } else if c.is_ascii_digit() {
            digits += 1;
        } else if !c.is_alphanumeric() {
            special += 1;
        }
    }
    [
        (policy.lower_case, "lower-case letter", lower),
        (policy.upper_case, "upper-case letter", upper),
        (policy.digits, "digit", digits),
        (
            policy.special,
            "character that is not a letter or a digit",
            special,
        ),
    ]
}

impl PasswordPolicy {
    /// The bounds the configuration's policy must keep, the ones the issuer
    /// holds, each refusal naming its field.
    pub fn validate(&self) -> Result<(), String> {
        if !(8..=128).contains(&self.length_min) {
            return Err("length_min is not 8 to 128".to_owned());
        }
        if !(self.length_min..=128).contains(&self.length_max) {
            return Err("length_max is not length_min to 128".to_owned());
        }
        let counts = [
            ("lower_case", self.lower_case, 32),
            ("upper_case", self.upper_case, 32),
            ("digits", self.digits, 32),
            ("special", self.special, 32),
            ("not_recently_used", self.not_recently_used, 10),
        ];
        match counts
            .iter()
            .find(|(_, value, most)| value.is_some_and(|value| !(1..=*most).contains(&value)))
        {
            Some((name, _, most)) => Err(format!("{name} is not 1 to {most}")),
            None => Ok(()),
        }
    }

    /// The policy in the words the screens show before a password is sent.
    pub fn words(&self) -> String {
        let mut words = format!(
            "At least {} and at most {} characters",
            self.length_min, self.length_max
        );
        let asked: Vec<String> = kinds(self, "")
            .into_iter()
            .filter_map(|(fewest, kind, _)| {
                fewest.map(|fewest| match fewest {
                    1 => format!("a {kind}"),
                    more => format!("{more} of the kind: {kind}"),
                })
            })
            .collect();
        if !asked.is_empty() {
            words.push_str(", with ");
            words.push_str(&asked.join(", "));
        }
        words.push('.');
        if let Some(last) = self.not_recently_used {
            words.push_str(" A new password is not one of your last ");
            words.push_str(&last.to_string());
            words.push('.');
        }
        words
    }

    /// The policy as the setup and account screens read it.
    pub fn view(&self) -> Value {
        json!({
            "length_min": self.length_min,
            "length_max": self.length_max,
            "lower_case": self.lower_case,
            "upper_case": self.upper_case,
            "digits": self.digits,
            "special": self.special,
            "not_recently_used": self.not_recently_used,
            "words": self.words(),
        })
    }

    /// Refuse `password` when this policy does not take it, in Lys's words.
    pub fn check(&self, password: &str) -> Result<(), ServerError> {
        let lengths = usize::from(self.length_min)..=usize::from(self.length_max);
        if !lengths.contains(&password.len()) {
            return Err(refused(format!(
                "a password has {} to {} characters",
                self.length_min, self.length_max
            )));
        }
        for (fewest, kind, held) in kinds(self, password) {
            if let Some(fewest) = fewest
                && held < usize::from(fewest)
            {
                return Err(refused(format!(
                    "a password has at least {fewest} of the kind: {kind}"
                )));
            }
        }
        Ok(())
    }
}

/// Refuse a password `policy` does not take, in Lys's words. With no policy
/// configured only an empty password is refused here, and the issuer checks
/// the rest.
pub fn check_password(policy: Option<&PasswordPolicy>, password: &str) -> Result<(), ServerError> {
    match policy {
        Some(policy) => policy.check(password),
        None if password.is_empty() => Err(refused("a password is not empty")),
        None => Ok(()),
    }
}

/// Refuse what is not an email address: one `@` with text on each side, a
/// dot in the domain, and no space, control character or character an
/// address carries in its path or query. The 254-octet ceiling is RFC 5321's
/// (4.5.3.1.3: a path of 256 octets, less its angle brackets), which the
/// issuer's own email validation also holds to.
pub fn check_email(email: &str) -> Result<&str, ServerError> {
    let email = email.trim();
    let shaped = email.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains('@')
    });
    let clean = !email
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '/' | '?' | '#' | '%'));
    if shaped && clean && email.len() <= 254 {
        Ok(email)
    } else {
        Err(refused("that is not an email address"))
    }
}
