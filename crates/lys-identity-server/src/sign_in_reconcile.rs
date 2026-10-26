//! Sign-in accounts follow the directory (DIRECTORY-124). Lys says who a
//! person is; the issuer only holds the credential they sign in with. Each
//! person's sign-in is judged against the issuer and shown on People in
//! words, and what is safe to repair is repaired by the install itself: at
//! every start, before anything is served, and after every administrator
//! change to a person. A login whose account the issuer no longer holds is
//! unbound, and a person left with no live login is given one: the issuer's
//! unbound account under the address Lys holds for them, or a new account
//! made under it. Nothing here ever hands one person's account to another:
//! an account carrying another person's address, an address two people
//! hold, more than one address held for one person, or an address whose
//! account belongs to someone else waits for an administrator on People.
//! Wiping the issuer and starting Lys again gives every person an account
//! under their own address, which their provider sign-in then joins.

use lys_identity::projection::Projection;
use lys_identity::{
    Actor, AuthMethod, Change, IdentityEvent, IdentityId, LifecycleState, LoginBinding,
    OperationId, PersonId, Provenance,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ServerError;
use crate::routes::{AppState, with_directory};
use crate::sign_in_providers::SignInProviders;

/// A person's sign-in judged against the issuer, as People shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SignInHealth {
    /// Their account is at the issuer, turned on, under the address Lys holds.
    Ready,
    /// Lys holds no address for them and no account of theirs is live.
    NoAddress,
    /// No account of theirs is live; Lys finds or makes one under their
    /// address the next time it repairs sign-ins.
    Repairable,
    /// Their account is turned off.
    Disabled,
    /// Lys holds more than one address for them, so their account signs in
    /// as nobody until an administrator sets the one that is theirs.
    SeveralAddresses {
        /// The addresses held.
        addresses: Vec<String>,
    },
    /// Their account carries an address another person holds: shown for an
    /// administrator to settle, by moving it to its person (`belongs_to`,
    /// when exactly one person holds that address) or retiring a duplicate.
    NotTheirs {
        /// The address the account carries.
        account_email: String,
        /// The person who holds that address, when exactly one does.
        belongs_to: Option<String>,
    },
    /// The issuer's account under their address is bound to another person.
    AddressTaken {
        /// The person that account is bound to.
        holder: String,
    },
    /// Another person holds the same address; an administrator decides.
    SharedAddress {
        /// The other people holding it.
        with: Vec<String>,
    },
}

/// One person as the directory and the foundings store hold them.
pub(crate) struct Seen {
    pub(crate) person: PersonId,
    /// Active, rather than suspended: only an active person is repaired.
    active: bool,
    held: Option<String>,
    several: Vec<String>,
    subjects: Vec<String>,
}

/// The issuer's answer for one person's logins and address.
pub(crate) struct Read {
    live: Vec<(String, String, bool)>,
    dead: Vec<String>,
    under_address: Option<Value>,
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Every active or suspended person, with the address held for them and
/// their logins at this install's issuer. One directory and foundings hold.
pub(crate) fn seen(state: &AppState) -> Result<Vec<Seen>, ServerError> {
    let issuer = state.oidc.issuer().to_owned();
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let foundings = crate::organisations_found::foundings(state)?;
        Ok(people(projection)
            .map(|(person, subjects, active)| {
                let addresses = foundings.addresses_for(&person.to_string());
                let (held, several) = match addresses.as_slice() {
                    [one] => (Some((*one).to_owned()), Vec::new()),
                    [] => (None, Vec::new()),
                    many => (None, many.iter().map(|email| (*email).to_owned()).collect()),
                };
                Seen {
                    person,
                    active,
                    held,
                    several,
                    subjects: subjects
                        .iter()
                        .filter(|binding| binding.issuer() == issuer)
                        .map(|binding| binding.subject().to_owned())
                        .collect(),
                }
            })
            .collect())
    })
}

/// Every active or suspended person, with their logins and whether they are active.
fn people(projection: &Projection) -> impl Iterator<Item = (PersonId, &[LoginBinding], bool)> {
    projection
        .people(std::ops::Bound::Unbounded)
        .filter_map(|(_, person)| {
            let record = projection.record(IdentityId::Person(person))?;
            let state = record.state();
            matches!(state, LifecycleState::Active | LifecycleState::Suspended).then_some((
                person,
                record.bindings(),
                state == LifecycleState::Active,
            ))
        })
}

/// Read each login's account and, when none is live, the account under the
/// held address. No directory or foundings hold crosses these calls.
pub(crate) async fn read(api: &SignInProviders, seen: &Seen) -> Result<Read, ServerError> {
    let mut answer = Read {
        live: Vec::new(),
        dead: Vec::new(),
        under_address: None,
    };
    for subject in &seen.subjects {
        match crate::accounts::read(api, subject).await {
            Ok(user) => answer.live.push((
                subject.clone(),
                user.get("email")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                user.get("enabled").and_then(Value::as_bool) == Some(true),
            )),
            Err(ServerError::SignInProvidersRefused { status: 404, .. }) => {
                answer.dead.push(subject.clone());
            }
            Err(error) => return Err(error),
        }
    }
    if answer.live.is_empty()
        && let Some(held) = &seen.held
    {
        answer.under_address = crate::accounts::find_by_email(api, held).await?;
    }
    Ok(answer)
}

/// Judge one person against everyone else's addresses and logins.
pub(crate) fn judge(
    seen: &Seen,
    read: &Read,
    everyone: &[Seen],
    name: &dyn Fn(PersonId) -> String,
) -> SignInHealth {
    if !seen.several.is_empty() {
        return SignInHealth::SeveralAddresses {
            addresses: seen.several.clone(),
        };
    }
    if let Some((_, email, enabled)) = read.live.first() {
        if !enabled {
            return SignInHealth::Disabled;
        }
        // Their own address is theirs whoever else holds it: a mix-up on
        // another record is shown on that record.
        if seen.held.as_deref().is_some_and(|held| same(held, email)) {
            return SignInHealth::Ready;
        }
        let others: Vec<&Seen> = everyone
            .iter()
            .filter(|other| other.person != seen.person)
            .filter(|other| {
                other
                    .held
                    .iter()
                    .chain(&other.several)
                    .any(|held| same(held, email))
            })
            .collect();
        return match others.as_slice() {
            [] => SignInHealth::Ready,
            [one] => SignInHealth::NotTheirs {
                account_email: email.clone(),
                belongs_to: Some(one.person.to_string()),
            },
            _ => SignInHealth::NotTheirs {
                account_email: email.clone(),
                belongs_to: None,
            },
        };
    }
    let Some(held) = &seen.held else {
        return SignInHealth::NoAddress;
    };
    let with: Vec<String> = everyone
        .iter()
        .filter(|other| other.person != seen.person)
        .filter(|other| {
            other
                .held
                .as_deref()
                .is_some_and(|theirs| same(theirs, held))
        })
        .map(|other| name(other.person))
        .collect();
    if !with.is_empty() {
        return SignInHealth::SharedAddress { with };
    }
    match read
        .under_address
        .as_ref()
        .and_then(|user| user.get("id")?.as_str())
    {
        Some(subject) => everyone
            .iter()
            .find(|other| other.subjects.iter().any(|theirs| theirs == subject))
            .map_or(SignInHealth::Repairable, |other| {
                SignInHealth::AddressTaken {
                    holder: name(other.person),
                }
            }),
        None => SignInHealth::Repairable,
    }
}

/// What one repair did, said on its one line.
#[derive(Debug, Default)]
pub(crate) struct Repaired {
    made: Vec<String>,
    adopted: Vec<String>,
    pruned: usize,
    waiting: Vec<String>,
    refused: Vec<String>,
}

impl Repaired {
    fn line(&self) -> String {
        format!(
            "sign-in accounts reconciled: {} made ({}), {} found under their address ({}), {} dead logins unbound, {} waiting for an administrator ({}), {} refused ({})",
            self.made.len(),
            self.made.join(", "),
            self.adopted.len(),
            self.adopted.join(", "),
            self.pruned,
            self.waiting.len(),
            self.waiting.join(", "),
            self.refused.len(),
            self.refused.join("; "),
        )
    }
}

/// Repair every person's sign-in that is safe to repair, as `actor`. Each
/// person is repaired on their own: one refused never stops the others,
/// and is named on the repair's line.
///
/// # Errors
/// Names a directory or foundings failure met before any person is read.
pub(crate) async fn repair(state: &AppState, actor: &Actor) -> Result<Repaired, ServerError> {
    let Some(api) = state.sign_in_providers.as_ref() else {
        return Ok(Repaired::default());
    };
    let everyone = seen(state)?;
    let names = names(state)?;
    let name = |person: PersonId| {
        names
            .iter()
            .find(|(id, _)| *id == person)
            .map_or_else(|| person.to_string(), |(_, name)| name.clone())
    };
    // The administrator's own login is never replaced here: the install
    // acts under it, and a fault at the issuer must not lock them out.
    let administrator = state.admission.administrator_login()?;
    let mut done = Repaired::default();
    for person in &everyone {
        let theirs = administrator.as_ref().is_some_and(|login| {
            login.issuer() == state.oidc.issuer()
                && person
                    .subjects
                    .iter()
                    .any(|subject| subject == login.subject())
        });
        let who = name(person.person);
        match repair_one(state, api, actor, person, &everyone, &name, theirs).await {
            Ok(Outcome::Nothing) => {}
            Ok(Outcome::Waiting) => done.waiting.push(who),
            Ok(Outcome::Made(pruned)) => {
                done.made.push(who);
                done.pruned += pruned;
            }
            Ok(Outcome::Adopted(pruned)) => {
                done.adopted.push(who);
                done.pruned += pruned;
            }
            Err(error) => done.refused.push(format!("{who}: {error}")),
        }
    }
    Ok(done)
}

/// What one person's repair did.
enum Outcome {
    Nothing,
    Waiting,
    Made(usize),
    Adopted(usize),
}

async fn repair_one(
    state: &AppState,
    api: &SignInProviders,
    actor: &Actor,
    person: &Seen,
    everyone: &[Seen],
    name: &(dyn Fn(PersonId) -> String + Sync),
    theirs: bool,
) -> Result<Outcome, ServerError> {
    let read = read(api, person).await?;
    let health = judge(person, &read, everyone, name);
    let (new, made) = match (&health, &read.under_address) {
        (SignInHealth::Ready | SignInHealth::Disabled, _) => return Ok(Outcome::Nothing),
        // A suspended person is never given an account: theirs waits
        // until they are reinstated, which repairs again.
        (SignInHealth::Repairable, _) if theirs || !person.active => return Ok(Outcome::Waiting),
        (SignInHealth::Repairable, Some(user)) => (crate::accounts::user_id(user)?, false),
        (SignInHealth::Repairable, None) => {
            let display = name(person.person);
            let held = person.held.as_deref().unwrap_or_default();
            let account = crate::accounts::AccountName {
                display: &display,
                given: None,
                family: None,
            };
            (crate::accounts::make(api, held, account).await?.0, true)
        }
        _ => return Ok(Outcome::Waiting),
    };
    // Dead logins go only with the live one that replaces them.
    if !commit(state, actor, person, &read.dead, &new)? {
        return Ok(Outcome::Waiting);
    }
    let pruned = read.dead.len();
    Ok(if made {
        Outcome::Made(pruned)
    } else {
        Outcome::Adopted(pruned)
    })
}

fn names(state: &AppState) -> Result<Vec<(PersonId, String)>, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        Ok(people(projection)
            .filter_map(|(person, _, _)| {
                let record = projection.record(IdentityId::Person(person))?;
                Some((person, record.profile().display_name().to_owned()))
            })
            .collect())
    })
}

/// Each of `ids` judged for People; a person who is neither active nor
/// suspended is not judged. Reads the issuer outside every hold.
///
/// # Errors
/// Names an issuer, directory or foundings failure.
pub(crate) async fn health_of(
    state: &AppState,
    api: &SignInProviders,
    ids: &[PersonId],
) -> Result<Vec<Option<SignInHealth>>, ServerError> {
    let everyone = seen(state)?;
    let names = names(state)?;
    let name = |person: PersonId| {
        names
            .iter()
            .find(|(id, _)| *id == person)
            .map_or_else(|| person.to_string(), |(_, name)| name.clone())
    };
    let mut answer = Vec::with_capacity(ids.len());
    for id in ids {
        answer.push(match everyone.iter().find(|seen| seen.person == *id) {
            Some(person) => Some(judge(person, &read(api, person).await?, &everyone, &name)),
            None => None,
        });
    }
    Ok(answer)
}

/// The login of `person` whose account the issuer holds, if any is.
///
/// # Errors
/// Names an unknown person, an issuer or a directory failure.
pub(crate) async fn live_account(
    state: &AppState,
    api: &SignInProviders,
    person: PersonId,
) -> Result<Option<String>, ServerError> {
    let issuer = state.oidc.issuer().to_owned();
    let subjects = with_directory(state, |directory| {
        Ok(
            crate::read_api::person_record(directory.projection()?, person)?
                .bindings()
                .iter()
                .filter(|binding| binding.issuer() == issuer)
                .map(|binding| binding.subject().to_owned())
                .collect(),
        )
    })?;
    let seen = Seen {
        person,
        active: true,
        held: None,
        several: Vec::new(),
        subjects,
    };
    Ok(read(api, &seen)
        .await?
        .live
        .first()
        .map(|(subject, _, _)| subject.clone()))
}

/// Hold `email` for `person`, who has no live account, so the repair finds
/// or makes their account under it (DIRECTORY-124 R3). An address another
/// person holds, or whose issuer account is bound to another person, is
/// refused by `EmailHeldByAnother`, naming them.
///
/// # Errors
/// Names that refusal, or an issuer, directory or foundings failure.
pub(crate) async fn hold_address(
    state: &AppState,
    api: &SignInProviders,
    person: PersonId,
    email: &str,
) -> Result<(), ServerError> {
    let everyone = seen(state)?;
    let names = names(state)?;
    let account = crate::accounts::find_by_email(api, email).await?;
    let subject = account
        .as_ref()
        .and_then(|user| user.get("id")?.as_str().map(str::to_owned));
    let other = everyone
        .iter()
        .filter(|other| other.person != person)
        .find(|other| {
            other
                .held
                .iter()
                .chain(&other.several)
                .any(|held| same(held, email))
                || subject
                    .as_ref()
                    .is_some_and(|subject| other.subjects.contains(subject))
        });
    if let Some(other) = other {
        return Err(ServerError::EmailHeldByAnother {
            person: other.person.to_string(),
            display_name: names
                .iter()
                .find(|(id, _)| *id == other.person)
                .map_or_else(|| other.person.to_string(), |(_, name)| name.clone()),
            email: email.to_owned(),
        });
    }
    with_directory(state, |_directory| {
        crate::organisations_found::foundings(state)?.hold_email(&person.to_string(), email, false)
    })
}

/// Unbind `dead` and bind `new` for one person, in one signed batch, after
/// checking their logins are still the ones read and that no record, in any
/// state, holds `new`; ends the dead sessions. Answers whether it wrote.
fn commit(
    state: &AppState,
    actor: &Actor,
    seen: &Seen,
    dead: &[String],
    new: &str,
) -> Result<bool, ServerError> {
    let issuer = state.oidc.issuer().to_owned();
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let record = crate::read_api::person_record(projection, seen.person)?;
        let now: Vec<&str> = record
            .bindings()
            .iter()
            .filter(|binding| binding.issuer() == issuer)
            .map(LoginBinding::subject)
            .collect();
        if now != seen.subjects.iter().map(String::as_str).collect::<Vec<_>>() {
            // Someone changed this person's logins since the read; the next
            // repair judges them afresh.
            return Ok(false);
        }
        // An account another record holds, a retired one included, is
        // never taken: an administrator moves it on People.
        if projection
            .person_for(&LoginBinding::new(&issuer, new)?)
            .is_some()
        {
            return Ok(false);
        }
        let operation = OperationId::generate()?;
        let at = crate::session::now();
        let mut events = Vec::new();
        for (step, change) in dead
            .iter()
            .map(|subject| ("unbind", subject.as_str()))
            .chain(std::iter::once(("bind", new)))
            .enumerate()
            .map(|(at, (verb, subject))| {
                let binding = LoginBinding::new(&issuer, subject)?;
                let change = if verb == "bind" {
                    Change::BindLogin { binding }
                } else {
                    Change::UnbindLogin { binding }
                };
                Ok::<_, ServerError>((format!("{verb}-{at}"), change))
            })
            .collect::<Result<Vec<_>, _>>()?
        {
            events.push(IdentityEvent::new(
                crate::people_admit::derived(&operation, &step).parse()?,
                actor.clone(),
                IdentityId::Person(seen.person),
                at,
                change,
            )?);
        }
        state.sessions.revoke_matching(|held| {
            held.binding().issuer() == issuer
                && dead
                    .iter()
                    .any(|subject| held.binding().subject() == subject)
        })?;
        directory.commit_batch(&events)?;
        Ok(true)
    })
}

/// The install's own actor: the administrator's login, by the operator,
/// which the log reads as no sign-in having taken place.
fn install_actor(state: &AppState) -> Result<Option<Actor>, ServerError> {
    Ok(state.admission.administrator_login()?.map(|login| {
        Actor::new(
            login,
            Provenance::new(AuthMethod::Operator, crate::session::now()),
        )
    }))
}

/// Repair at start, before anything is served. An issuer that cannot be
/// reached or a refused repair is said on its own line and the service
/// starts: People shows each person's sign-in, and the next administrator
/// change repairs again.
pub(crate) async fn at_start(state: &AppState) {
    let said = match install_actor(state) {
        Ok(None) => return,
        Ok(Some(actor)) => match repair(state, &actor).await {
            Ok(done) => done.line(),
            Err(error) => format!("sign-in accounts were not reconciled at start: {error}"),
        },
        Err(error) => format!("sign-in accounts were not reconciled at start: {error}"),
    };
    (state.say)(&said);
}

/// Repair after an administrator's change to a person, as that
/// administrator. The change stands whatever the repair answers; a refused
/// repair is said, and People shows what waits.
pub(crate) async fn after_change(state: &AppState, actor: &Actor) {
    let said = match repair(state, actor).await {
        Ok(done) => done.line(),
        Err(error) => format!("sign-in accounts were not reconciled after a change: {error}"),
    };
    (state.say)(&said);
}

#[cfg(test)]
#[path = "sign_in_reconcile_tests.rs"]
mod tests;
