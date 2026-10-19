//! Each store compared with its receipt entry, by the rule its kind keeps.

use super::{ComparedBy, Entry, Found};

/// What a comparison found wrong with one store: the refusal's name, what the
/// receipt holds and what was found, each in words a person reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loss {
    /// The store's name.
    pub store: String,
    /// `store_lost`, `grant_log_identity_changed` or
    /// `spicedb_projection_foreign`.
    pub refusal: &'static str,
    /// What the receipt holds.
    pub held: String,
    /// What was found.
    pub found: String,
}

/// A store's standing against its receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// At or past the receipt: served.
    Kept,
    /// A projection behind the log it projects: re-projected before serving.
    Behind {
        /// The revision projected.
        projected: u64,
        /// The grant log's head.
        head: u64,
    },
    /// Short of the receipt, of another origin or identity, or absent.
    Lost(Loss),
}

fn short(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}

/// `entry`'s head in words: `head 12 of origin 3fa2c1d09e8b`.
fn held_words(entry: &Entry) -> String {
    match entry.compared_by {
        ComparedBy::LogHead => format!("head {} of origin {}", entry.head, short(&entry.digest)),
        ComparedBy::JournalHead => {
            format!("{} records in format {}", entry.head, short(&entry.digest))
        }
        ComparedBy::ProjectedRevision => format!("projected revision {}", entry.head),
        ComparedBy::Digest => format!("key {}", short(&entry.digest)),
        ComparedBy::Presence => format!("present at {}", entry.path.display()),
    }
}

fn found_words(entry: &Entry, found: &Found) -> String {
    held_words(&Entry {
        head: found.head,
        digest: found.digest.clone(),
        ..entry.clone()
    })
}

fn lost(store: &str, refusal: &'static str, held: String, found: String) -> Verdict {
    Verdict::Lost(Loss {
        store: store.to_owned(),
        refusal,
        held,
        found,
    })
}

/// `store`'s standing: `found` read where `entry` says it is kept, `None`
/// when nothing is there. A projection is compared by
/// [`compare_projection`], never here.
#[must_use]
pub fn compare(store: &str, entry: &Entry, found: Option<&Found>) -> Verdict {
    // A store the operator accepted at nothing is counted as nothing until
    // it is first written: absent or new, no record of it can be lost.
    if entry.counts_nothing() {
        return Verdict::Kept;
    }
    let Some(found) = found else {
        return lost(
            store,
            "store_lost",
            held_words(entry),
            format!("nothing at {}", entry.path.display()),
        );
    };
    if let (Some(held), Some(read)) = (&entry.identity, &found.identity)
        && (held.identity != read.identity || read.epoch < held.epoch)
    {
        return lost(
            store,
            "grant_log_identity_changed",
            format!("identity {} at epoch {}", held.identity, held.epoch),
            format!("identity {} at epoch {}", read.identity, read.epoch),
        );
    }
    match entry.compared_by {
        ComparedBy::LogHead if found.digest != entry.digest || found.head < entry.head => lost(
            store,
            "store_lost",
            held_words(entry),
            found_words(entry, found),
        ),
        // A one-file store's format moves forward on its first write (an
        // earlier whole-file layout gains its journal header), so only its
        // records are compared; the format digest is kept to read.
        ComparedBy::JournalHead if found.head < entry.head => lost(
            store,
            "store_lost",
            held_words(entry),
            found_words(entry, found),
        ),
        ComparedBy::Digest if found.digest != entry.digest => lost(
            store,
            "store_lost",
            held_words(entry),
            found_words(entry, found),
        ),
        _ => Verdict::Kept,
    }
}

/// The projection's standing: `projected` is the grant revision the engine
/// stands at, `head` and `identity` the grant log's. Behind is re-projected;
/// ahead of the head, or a projection recorded of another log identity, is
/// foreign.
#[must_use]
pub fn compare_projection(
    store: &str,
    entry: Option<&Entry>,
    projected: u64,
    head: u64,
    identity: Option<&str>,
) -> Verdict {
    let held_identity = entry
        .and_then(|entry| entry.identity.as_ref())
        .map(|held| held.identity.as_str());
    if let (Some(held), Some(log)) = (held_identity, identity)
        && held != log
    {
        return lost(
            store,
            "spicedb_projection_foreign",
            format!("a projection of grant log {held} at revision {projected}"),
            format!("grant log {log} at revision {head}"),
        );
    }
    if projected > head {
        return lost(
            store,
            "spicedb_projection_foreign",
            format!("projected revision {projected}"),
            format!("grant log head {head}"),
        );
    }
    if projected < head {
        return Verdict::Behind { projected, head };
    }
    Verdict::Kept
}
