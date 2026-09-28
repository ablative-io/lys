//! Sparse tree checkpoints: the log's frontier at every multiple of
//! [`CHECKPOINT_EVERY`] leaves.
//!
//! A receipt names the root of the tree its leaf completed. Holding a root
//! per leaf grows with the log, and rebuilding one from the whole tree reads
//! every leaf. A checkpoint every [`CHECKPOINT_EVERY`] leaves holds the
//! frontier at that size, and the root at any leaf is that frontier extended
//! by the leaves between it and the leaf: never more than
//! `CHECKPOINT_EVERY` reads.
//!
//! The checkpoints travel in the snapshot beside the owner's state, sealed by
//! the same signature. A snapshot whose checkpoints are not one per multiple
//! up to its size, each a well-formed frontier of that size, is refused.

use ciborium::Value;
use lys_log_store::Frontier;

use crate::state_value::{
    Unreadable, array, bytes, decode as decode_value, encode as encode_value, list, read_uint,
    tuple, uint,
};

/// How many leaves lie between two checkpoints.
pub const CHECKPOINT_EVERY: u64 = 1024;

/// The version of the wrapping this crate writes and reads.
const WRAP_VERSION: u64 = 1;

/// The frontier at every multiple of [`CHECKPOINT_EVERY`] leaves, from size 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Checkpoints {
    frontiers: Vec<Frontier>,
}

impl Default for Checkpoints {
    fn default() -> Self {
        Self {
            frontiers: vec![Frontier::new()],
        }
    }
}

impl Checkpoints {
    /// Keeps `frontier` if its size is the next multiple not yet held.
    pub(crate) fn record(&mut self, frontier: &Frontier) {
        let next = self.frontiers.len() as u64 * CHECKPOINT_EVERY;
        if frontier.size() == next {
            self.frontiers.push(frontier.clone());
        }
    }

    /// The checkpoint at or below the leaf at `index`.
    pub(crate) fn before(&self, index: u64) -> Option<&Frontier> {
        usize::try_from(index / CHECKPOINT_EVERY)
            .ok()
            .and_then(|slot| self.frontiers.get(slot))
    }

    fn encode(&self) -> Value {
        array(
            self.frontiers
                .iter()
                .map(|frontier| bytes(&frontier.nodes().concat()))
                .collect(),
        )
    }

    fn decode(value: Value, size: u64) -> Result<Self, Unreadable> {
        let held = list(value, "the checkpoints")?;
        let expected = size / CHECKPOINT_EVERY + 1;
        if held.len() as u64 != expected {
            return Err(format!(
                "the snapshot at {size} holds {} checkpoints, not {expected}",
                held.len()
            ));
        }
        let frontiers = (0_u64..)
            .zip(held)
            .map(|(slot, nodes)| {
                let Value::Bytes(nodes) = nodes else {
                    return Err("a checkpoint is not a byte string".to_owned());
                };
                let nodes = nodes
                    .chunks(32)
                    .map(|chunk| <[u8; 32]>::try_from(chunk).ok())
                    .collect::<Option<Vec<_>>>()
                    .ok_or("a checkpoint is not a whole number of 32-byte roots")?;
                Frontier::from_parts(slot * CHECKPOINT_EVERY, nodes)
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { frontiers })
    }
}

/// The snapshot state: the checkpoints and the owner's state beside them.
pub(crate) fn wrap(checkpoints: &Checkpoints, owner: &[u8]) -> Result<Vec<u8>, Unreadable> {
    encode_value(&array(vec![
        uint(WRAP_VERSION),
        checkpoints.encode(),
        bytes(owner),
    ]))
}

/// The checkpoints and the owner's state a snapshot at `size` carries.
pub(crate) fn unwrap(state: &[u8], size: u64) -> Result<(Checkpoints, Vec<u8>), Unreadable> {
    let [version, checkpoints, owner] = tuple::<3>(decode_value(state)?, "a snapshot state")?;
    let version = read_uint(&version, "a snapshot state version")?;
    if version != WRAP_VERSION {
        return Err(format!(
            "snapshot state version {version} is not {WRAP_VERSION}"
        ));
    }
    let Value::Bytes(owner) = owner else {
        return Err("the owner's state is not a byte string".to_owned());
    };
    Ok((Checkpoints::decode(checkpoints, size)?, owner))
}

#[cfg(test)]
mod tests {
    use lys_log_store::Frontier;

    use super::{CHECKPOINT_EVERY, Checkpoints, unwrap, wrap};

    #[test]
    fn checkpoints_travel_with_the_state_and_name_the_nearest_frontier() -> Result<(), String> {
        let mut frontier = Frontier::new();
        let mut checkpoints = Checkpoints::default();
        for leaf in 0..(2 * CHECKPOINT_EVERY + 5) {
            frontier.push(&leaf.to_be_bytes());
            checkpoints.record(&frontier);
        }
        let size = frontier.size();
        let (read, owner) = unwrap(&wrap(&checkpoints, b"owner")?, size)?;
        assert_eq!(read, checkpoints);
        assert_eq!(owner, b"owner");
        assert_eq!(
            checkpoints.before(CHECKPOINT_EVERY + 3).map(Frontier::size),
            Some(CHECKPOINT_EVERY)
        );
        assert!(unwrap(&wrap(&checkpoints, b"owner")?, size + CHECKPOINT_EVERY).is_err());
        Ok(())
    }
}
