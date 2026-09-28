//! Leases in the world of CONFORMANCE row 7.6. `agent_a` acts for `person_a`,
//! stands in the organisation and may use `a_org` and `b_org`; `person_a` may use
//! `a_org`. A use goes through the broker's proxy path to an upstream double
//! that counts the use requests forwarded to it, and the system behind is a
//! double that counts the revoke requests it receives and acknowledges
//! nothing until a test delivers its acknowledgement.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use lys_core::Ed25519Identity;
use lys_secrets::{
    Broker, EndAct, Holder, IssuedHandle, LeaseRefusal, LeaseView, LocalGrants, Presentation,
    Relation, Scope, SecretRelation, new_operation_id,
};
use serde_json::Value;
use tempfile::TempDir;

use crate::fixture::{
    A_ORG, AGENT_A, B_ORG, Failure, HIDDEN_FROM_A, ORGANISATION, PERSON_A, PERSON_B, SEEN_BY_A,
    START_MS, list, names, owned, person_a, world,
};

/// A person in neither `team_a` nor `team_b`, acted for under no lease.
pub const PERSON_C: &str = "person-c";
/// How long a lease's window is.
pub const WINDOW_MS: i64 = 60_000;
/// How a revoke's drop line opens.
pub const REVOKED: &str = "ended by ";
/// How a relinquish's drop line opens.
pub const RELINQUISHED: &str = "relinquished by ";
/// How the audit line of the move to upstream unconfirmed opens.
pub const UNCONFIRMED: &str = "revocation_unconfirmed";
/// The audit line of the move to upstream confirmed.
pub const CONFIRMED: &str = "revoked_upstream";

fn relation(identity: &str, target: &str, by: &str) -> SecretRelation {
    SecretRelation {
        identity: identity.to_owned(),
        secret: target.to_owned(),
        granted_by: Some(by.to_owned()),
    }
}

/// The world with its leases and its two doubles.
pub struct Leases {
    dir: TempDir,
    clock: Arc<AtomicI64>,
    pub broker: Broker<LocalGrants>,
    /// Each lease's holder and the end of its window.
    issued: BTreeMap<String, (String, i64)>,
    /// The use requests the upstream double received.
    pub forwarded: u64,
    /// The leases the system behind was asked to revoke, in order.
    pub revoke_requests: Vec<String>,
}

impl Leases {
    pub fn new() -> Result<Self, Failure> {
        let dir = tempfile::tempdir()?;
        let clock = Arc::new(AtomicI64::new(START_MS));
        let read = Arc::clone(&clock);
        let broker = world(dir.path(), Box::new(move || read.load(Ordering::SeqCst)))?;
        let grants = broker.permissions();
        let acted_for = Scope::Personal(PERSON_A.to_owned()).target();
        let organisation = Scope::Organisation(ORGANISATION.to_owned()).target();
        grants.grant_as(Relation::Member, relation(AGENT_A, &acted_for, PERSON_A));
        grants.grant_as(Relation::Member, relation(AGENT_A, &organisation, PERSON_A));
        grants.grant(relation(AGENT_A, A_ORG, PERSON_A));
        grants.grant(relation(AGENT_A, B_ORG, PERSON_B));
        grants.grant(relation(PERSON_A, A_ORG, PERSON_A));
        Ok(Self {
            dir,
            clock,
            broker,
            issued: BTreeMap::new(),
            forwarded: 0,
            revoke_requests: Vec::new(),
        })
    }

    /// The world's clock.
    pub fn now(&self) -> i64 {
        self.clock.load(Ordering::SeqCst)
    }

    /// Moves the world's clock on by `by` milliseconds.
    pub fn advance(&self, by: i64) {
        self.clock.fetch_add(by, Ordering::SeqCst);
    }

    fn key(&self, holder: &str) -> Result<Ed25519Identity, Failure> {
        Ok(Ed25519Identity::load_or_generate(
            &self.dir.path().join(format!("{holder}.key")),
        )?)
    }

    /// Issues a lease on `secret` to `holder`, its window ending
    /// `WINDOW_MS` from now.
    pub fn issue(&mut self, holder: &str, secret: &str) -> Result<IssuedHandle, Failure> {
        let not_after = self.now() + WINDOW_MS;
        let bound = Holder {
            identity: holder.to_owned(),
            key: self.key(holder)?.public_key_bytes(),
        };
        let lease = self.broker.issue(&bound, secret, 100, not_after)?;
        self.issued
            .insert(lease.id.as_str().to_owned(), (holder.to_owned(), not_after));
        Ok(lease)
    }

    /// One use of `lease` by its holder through the proxy path: whether the
    /// upstream double was reached.
    pub fn use_lease(&mut self, lease: &IssuedHandle) -> Result<bool, Failure> {
        let (holder, _window) = self
            .issued
            .get(lease.id.as_str())
            .cloned()
            .ok_or("no such lease was issued")?;
        let presented = Presentation::sign(
            &lease.id,
            &new_operation_id()?,
            self.now(),
            [1; 32],
            &self.key(&holder)?,
        )?;
        let forwarded = &mut self.forwarded;
        let used = self
            .broker
            .use_handle(&lease.token, &presented, |_credential| *forwarded += 1);
        Ok(used.is_ok())
    }

    /// Moves the world's clock past the end of `lease`'s window.
    pub fn pass_window(&self, lease: &IssuedHandle) -> Result<i64, Failure> {
        let (_holder, not_after) = self
            .issued
            .get(lease.id.as_str())
            .cloned()
            .ok_or("no such lease was issued")?;
        self.clock.store(not_after + 1, Ordering::SeqCst);
        Ok(not_after)
    }

    /// `GET /leases/{lease_id}` as `caller`: the status and the body.
    pub fn read(&self, caller: &str, lease: &IssuedHandle) -> (u16, Value) {
        answered(self.broker.lease_view(caller, &lease.id))
    }

    /// The upstream `GET /leases/{lease_id}` reads for `person_a`.
    pub fn upstream(&self, lease: &IssuedHandle) -> Result<String, Failure> {
        let (status, body) = self.read(PERSON_A, lease);
        if status != 200 {
            return Err(format!("the lease read answered {status}: {body}").into());
        }
        Ok(body["upstream"]
            .as_str()
            .ok_or("the lease read carries no upstream")?
            .to_owned())
    }

    /// `POST /leases/{lease_id}/revoke` as `caller`.
    pub fn revoke(&mut self, caller: &str, lease: &IssuedHandle) -> (u16, Value) {
        let asked = &mut self.revoke_requests;
        answered(self.broker.revoke_lease(
            caller,
            &lease.id,
            &mut |ended: &str, _secret: &str| asked.push(ended.to_owned()),
        ))
    }

    /// `POST /leases/{lease_id}/relinquish` as `caller`.
    pub fn relinquish(&mut self, caller: &str, lease: &IssuedHandle) -> (u16, Value) {
        let asked = &mut self.revoke_requests;
        answered(self.broker.relinquish_lease(
            caller,
            &lease.id,
            &mut |ended: &str, _secret: &str| asked.push(ended.to_owned()),
        ))
    }

    /// The system behind delivers its acknowledgement for `lease`.
    pub fn ack(&mut self, lease: &IssuedHandle) -> Result<bool, Failure> {
        Ok(self.broker.deliver_upstream_ack(&lease.id)?)
    }

    /// How many audit lines for `lease` have an outcome opening `opening`.
    pub fn lines(&self, lease: &IssuedHandle, opening: &str) -> Result<usize, Failure> {
        Ok(self
            .broker
            .audit()
            .replay()?
            .iter()
            .filter(|recorded| recorded.line.handle.as_deref() == Some(lease.id.as_str()))
            .filter(|recorded| recorded.line.outcome.starts_with(opening))
            .count())
    }

    /// With `lease` ended by `first` and its acknowledgement withheld, drives
    /// every other operation on it in turn, nine steps, and reads its
    /// upstream after each. Answers the nine readings.
    pub fn nine_steps(
        &mut self,
        lease: &IssuedHandle,
        first: EndAct,
    ) -> Result<Vec<String>, Failure> {
        let mut readings = Vec::new();

        assert!(!self.use_lease(lease)?, "(1) a use of an ended lease");
        readings.push(self.upstream(lease)?);

        assert_eq!(self.read(PERSON_A, lease).0, 200, "(2) the lease read");
        readings.push(self.upstream(lease)?);

        let (again, other) = match first {
            EndAct::Revoke => (self.revoke(PERSON_A, lease), self.relinquish(AGENT_A, lease)),
            EndAct::Relinquish => (self.relinquish(AGENT_A, lease), self.revoke(PERSON_A, lease)),
        };
        assert_eq!(again.0, 409, "(3) the same act again: {}", again.1);
        readings.push(self.upstream(lease)?);
        assert_eq!(other.0, 409, "(4) the other act: {}", other.1);
        readings.push(self.upstream(lease)?);

        for scope in [None, Some("organisation"), Some("team"), Some("mine")] {
            let (status, body) = list(&self.broker, &person_a(), scope);
            assert_eq!(status, 200, "(5 to 8) the list with scope {scope:?}: {body}");
            let listed = names(&body);
            assert!(
                listed.iter().all(|name| SEEN_BY_A.contains(&name.as_str())),
                "{body}"
            );
            assert!(
                HIDDEN_FROM_A
                    .iter()
                    .all(|hidden| !listed.contains(&(*hidden).to_owned())),
                "{body}"
            );
            readings.push(self.upstream(lease)?);
        }
        assert_eq!(
            names(&list(&self.broker, &person_a(), None).1),
            owned(&SEEN_BY_A)
        );

        self.pass_window(lease)?;
        readings.push(self.upstream(lease)?);
        Ok(readings)
    }
}

/// A lease route's answer: the status and the body.
fn answered(answer: Result<LeaseView, LeaseRefusal>) -> (u16, Value) {
    match answer {
        Ok(view) => (200, view.body()),
        Err(refusal) => (refusal.status(), refusal.body()),
    }
}
