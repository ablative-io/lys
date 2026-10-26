//! The four letters Lys writes, one template each (DIRECTORY-110 R3,
//! DIRECTORY-111 R4): added, to a person admitted; access given, to a
//! person a root grant names; founded, to a founder, carrying the link that
//! proves their address; already yours, to an address someone asked to
//! found with that already signs in. Every letter is plain text, says who
//! it is to, and names the address people open Lys at. The words are the
//! ones the screens use.

use lettre::Address;
use lettre::message::Mailbox;
use lys_identity::grants::shipped::ACTION_SENTENCES;

/// A letter to one person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Letter {
    /// They were added: where to sign in and, when Lys offers Google, that
    /// they choose it with the email Lys knows them by.
    Added {
        /// The person's name.
        name: String,
        /// Their email, the one they sign in with.
        email: String,
        /// The address people open Lys at.
        address: String,
        /// Google's name at the issuer when Lys offers it, else none.
        google: Option<String>,
    },
    /// A root grant names them: what they may now do, in the access
    /// screen's words.
    AccessGiven {
        /// The person's name.
        name: String,
        /// Their email.
        email: String,
        /// What they may now do, as [`access_words`] says it.
        words: String,
        /// The address people open Lys at.
        address: String,
    },
    /// They are founding an organisation: the link that proves their address.
    Founded {
        /// The founder's name.
        name: String,
        /// Their email.
        email: String,
        /// The proving link.
        link: String,
        /// The organisation's name.
        organisation: String,
    },
    /// Someone asked to found an organisation with an address that already
    /// signs in to Lys: the person is told, nothing was founded, and the
    /// asker learns nothing (DIRECTORY-111 R1).
    AlreadyYours {
        /// Their email.
        email: String,
        /// The organisation's name as it was asked for.
        organisation: String,
        /// The address people open Lys at.
        address: String,
    },
}

impl Letter {
    /// The letter's name: `added`, `access_given`, `founded` or
    /// `already_yours`.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Added { .. } => "added",
            Self::AccessGiven { .. } => "access_given",
            Self::Founded { .. } => "founded",
            Self::AlreadyYours { .. } => "already_yours",
        }
    }

    /// The address it goes to.
    #[must_use]
    pub fn to(&self) -> &str {
        match self {
            Self::Added { email, .. }
            | Self::AccessGiven { email, .. }
            | Self::Founded { email, .. }
            | Self::AlreadyYours { email, .. } => email,
        }
    }

    /// The name of the person it goes to; the already-yours letter knows
    /// no name, since it says nothing the asker gave.
    #[must_use]
    pub fn recipient(&self) -> &str {
        match self {
            Self::Added { name, .. }
            | Self::AccessGiven { name, .. }
            | Self::Founded { name, .. } => name,
            Self::AlreadyYours { .. } => "",
        }
    }

    /// Its subject line.
    #[must_use]
    pub fn subject(&self) -> String {
        match self {
            Self::Added { .. } => "You have been added to Lys".to_owned(),
            Self::AccessGiven { .. } => "You have been given access in Lys".to_owned(),
            Self::Founded { organisation, .. } => format!("Found {organisation} in Lys"),
            Self::AlreadyYours { .. } => "This address already signs in to Lys".to_owned(),
        }
    }

    /// Its text.
    #[must_use]
    pub fn body(&self) -> String {
        match self {
            Self::Added {
                name,
                email,
                address,
                google: Some(google),
            } => format!(
                "Hello {name}, you have been added to Lys. This sign-in opens your record and signs you in as {name}. Open {address} and choose Sign in with {google}, using {email}."
            ),
            Self::Added {
                name,
                email,
                address,
                google: None,
            } => format!(
                "Hello {name}, you have been added to Lys. This sign-in opens your record and signs you in as {name}. Sign in at {address} with {email}."
            ),
            Self::AccessGiven {
                name,
                words,
                address,
                ..
            } => format!(
                "Hello {name}, you have been given access in Lys: {words}. Sign in at {address} to use it."
            ),
            Self::Founded {
                name,
                link,
                organisation,
                ..
            } => format!(
                "Hello {name}, open {link} to prove this address and found {organisation} in Lys. The link is yours alone; if you did not ask for it, ignore this letter."
            ),
            Self::AlreadyYours {
                organisation,
                address,
                ..
            } => format!(
                "Hello, someone asked to found {organisation} in Lys with this address. It already signs in to Lys at {address}, so nothing was founded; sign in there. If this was not you, ignore this letter."
            ),
        }
    }

    /// The recipient as a mailbox, refused in words when the address is not one.
    pub(crate) fn mailbox(&self) -> Result<Mailbox, String> {
        let address: Address = self
            .to()
            .parse()
            .map_err(|error| format!("{} is not an address: {error}", self.to()))?;
        let name = Some(self.recipient())
            .filter(|name| !name.is_empty())
            .map(str::to_owned);
        Ok(Mailbox::new(name, address))
    }
}

/// What a grant lets its holder do, in the access screen's words: the
/// relation on the resource, then each action's sentence, an action with no
/// sentence by its name.
#[must_use]
pub fn access_words<'a>(
    relation: &str,
    kind: &str,
    id: &str,
    actions: impl Iterator<Item = &'a str>,
) -> String {
    let sentences: Vec<&str> = actions
        .map(|action| {
            ACTION_SENTENCES
                .iter()
                .find(|(named, _)| *named == action)
                .map_or(action, |(_, sentence)| sentence)
        })
        .collect();
    if sentences.is_empty() {
        format!("{relation} on {kind} {id}")
    } else {
        format!("{relation} on {kind} {id}: {}", sentences.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn added(google: Option<&str>) -> Letter {
        Letter::Added {
            name: "Care Helper".to_owned(),
            email: "helper@example.test".to_owned(),
            address: "https://lys.example.test".to_owned(),
            google: google.map(str::to_owned),
        }
    }

    #[test]
    fn the_added_letter_is_the_invitation_the_screen_used_to_show() {
        assert_eq!(added(None).subject(), "You have been added to Lys");
        assert_eq!(
            added(None).body(),
            "Hello Care Helper, you have been added to Lys. This sign-in opens your record and signs you in as Care Helper. Sign in at https://lys.example.test with helper@example.test."
        );
        assert_eq!(
            added(Some("Google")).body(),
            "Hello Care Helper, you have been added to Lys. This sign-in opens your record and signs you in as Care Helper. Open https://lys.example.test and choose Sign in with Google, using helper@example.test."
        );
        assert_eq!(added(None).name(), "added");
        assert_eq!(added(None).to(), "helper@example.test");
    }

    #[test]
    fn the_access_letter_says_what_they_may_do_in_the_screens_words() {
        let words = access_words(
            "editor",
            "workspace",
            "ward-7",
            ["view", "edit"].into_iter(),
        );
        assert_eq!(
            words,
            "editor on workspace ward-7: View this resource; Edit this resource"
        );
        assert_eq!(
            access_words("tern", "doc", "alpha", ["doc.sign"].into_iter()),
            "tern on doc alpha: doc.sign"
        );
        assert_eq!(
            access_words("alpha", "doc", "alpha", std::iter::empty()),
            "alpha on doc alpha"
        );
        let letter = Letter::AccessGiven {
            name: "Bea".to_owned(),
            email: "bea@example.test".to_owned(),
            words,
            address: "http://localhost:8490".to_owned(),
        };
        assert_eq!(letter.subject(), "You have been given access in Lys");
        assert_eq!(
            letter.body(),
            "Hello Bea, you have been given access in Lys: editor on workspace ward-7: View this resource; Edit this resource. Sign in at http://localhost:8490 to use it."
        );
        assert_eq!(letter.name(), "access_given");
    }

    #[test]
    fn the_founded_letter_carries_the_proving_link_and_the_organisation() {
        let letter = Letter::Founded {
            name: "Ada".to_owned(),
            email: "ada@example.test".to_owned(),
            link: "https://lys.example.test/found/abc".to_owned(),
            organisation: "Acme".to_owned(),
        };
        assert_eq!(letter.subject(), "Found Acme in Lys");
        assert_eq!(
            letter.body(),
            "Hello Ada, open https://lys.example.test/found/abc to prove this address and found Acme in Lys. The link is yours alone; if you did not ask for it, ignore this letter."
        );
        assert_eq!(letter.name(), "founded");
        let mailbox = letter.mailbox().expect("an address");
        assert_eq!(mailbox.email.to_string(), "ada@example.test");
        assert_eq!(mailbox.name.as_deref(), Some("Ada"));
    }

    #[test]
    fn an_address_that_is_not_one_is_refused_in_words() {
        let letter = Letter::Founded {
            name: "Ada".to_owned(),
            email: "not an address".to_owned(),
            link: String::new(),
            organisation: String::new(),
        };
        let refused = letter.mailbox().expect_err("not an address");
        assert!(
            refused.starts_with("not an address is not an address"),
            "{refused}"
        );
    }

    #[test]
    fn the_already_yours_letter_names_the_organisation_asked_for_and_no_asker() {
        let letter = Letter::AlreadyYours {
            email: "priya@example.test".to_owned(),
            organisation: "Acme".to_owned(),
            address: "https://lys.example.test".to_owned(),
        };
        assert_eq!(letter.name(), "already_yours");
        assert_eq!(letter.to(), "priya@example.test");
        assert_eq!(letter.recipient(), "");
        assert_eq!(letter.subject(), "This address already signs in to Lys");
        assert_eq!(
            letter.body(),
            "Hello, someone asked to found Acme in Lys with this address. It already signs in to Lys at https://lys.example.test, so nothing was founded; sign in there. If this was not you, ignore this letter."
        );
        let mailbox = letter.mailbox().expect("an address");
        assert_eq!(mailbox.name, None);
        assert_eq!(mailbox.email.to_string(), "priya@example.test");
    }
}
