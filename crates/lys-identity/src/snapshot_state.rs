//! The state a signed-event log's snapshot carries: every leaf it held, in
//! order.
//!
//! The state is a count and then each leaf, every number an 8-byte big-endian
//! integer and every leaf framed by its length. Nothing may follow the last
//! leaf. The snapshot seals this state with the tree size and root it was
//! taken at, so a reader checks the leaves against that root before it
//! believes any of them.

/// The bytes of a number or a length.
const WIDTH: usize = 8;

/// Encodes `leaves` as a snapshot state.
pub(crate) fn encode_leaves(leaves: &[Vec<u8>]) -> Vec<u8> {
    let total: usize = leaves.iter().map(|leaf| leaf.len() + WIDTH).sum();
    let mut out = Vec::with_capacity(total + WIDTH);
    out.extend_from_slice(&(leaves.len() as u64).to_be_bytes());
    for leaf in leaves {
        out.extend_from_slice(&(leaf.len() as u64).to_be_bytes());
        out.extend_from_slice(leaf);
    }
    out
}

/// Decodes a snapshot state into its leaves, refusing any other shape.
pub(crate) fn decode_leaves(state: &[u8]) -> Result<Vec<Vec<u8>>, &'static str> {
    let mut rest = state;
    let count = number(&mut rest)?;
    let mut leaves = Vec::new();
    for _ in 0..count {
        let len = usize::try_from(number(&mut rest)?)
            .map_err(|_width| "a leaf is longer than this machine can address")?;
        if rest.len() < len {
            return Err("a leaf runs past the end of the state");
        }
        let (leaf, after) = rest.split_at(len);
        leaves.push(leaf.to_vec());
        rest = after;
    }
    if rest.is_empty() {
        Ok(leaves)
    } else {
        Err("bytes follow the last leaf of the state")
    }
}

fn number(rest: &mut &[u8]) -> Result<u64, &'static str> {
    if rest.len() < WIDTH {
        return Err("a number runs past the end of the state");
    }
    let (bytes, after) = rest.split_at(WIDTH);
    let bytes = <[u8; WIDTH]>::try_from(bytes).map_err(|_length| "a number is not 8 bytes")?;
    *rest = after;
    Ok(u64::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::{decode_leaves, encode_leaves};

    #[test]
    fn leaves_survive_the_state_encoding() {
        let leaves = vec![b"one".to_vec(), Vec::new(), vec![7; 300]];
        assert_eq!(decode_leaves(&encode_leaves(&leaves)), Ok(leaves));
    }

    #[test]
    fn a_state_with_trailing_or_missing_bytes_is_refused() {
        let mut state = encode_leaves(&[b"leaf".to_vec()]);
        state.push(0);
        assert!(decode_leaves(&state).is_err());
        state.truncate(state.len() - 2);
        assert!(decode_leaves(&state).is_err());
        assert!(decode_leaves(&[]).is_err());
    }
}
