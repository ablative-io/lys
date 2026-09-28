#![cfg(test)]
//! Gates on the hash-only tree. The second party is the leaf-holding
//! [`AppendOnlyTree<RawLeaf>`] over ct-merkle, an independent implementation of
//! the same RFC 6962 algorithm: over seeded random logs every root, inclusion
//! proof and consistency proof must be byte-identical to its, and must verify
//! with this crate's verifiers.

use super::*;
use crate::merkle::leaf::raw_leaf_hash;
use crate::merkle::proof::{verify_consistency, verify_inclusion_raw};

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// A seeded source of random logs (splitmix64), so a failing case replays
/// exactly.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    /// Up to 40 random bytes, empty included.
    fn leaf(&mut self) -> Vec<u8> {
        let words = self.below(6);
        let keep = self.below(41);
        (0..words)
            .flat_map(|_| self.next().to_le_bytes())
            .zip(0..keep)
            .map(|(byte, _)| byte)
            .collect()
    }

    fn log(&mut self, size: u64) -> Vec<Vec<u8>> {
        (0..size).map(|_| self.leaf()).collect()
    }
}

/// Both trees over the same leaves, and the base tree's root at every size.
struct Pair {
    leaves: Vec<Vec<u8>>,
    base: AppendOnlyTree<RawLeaf>,
    hashes: HashTree,
    base_roots: Vec<RootHash>,
}

impl Pair {
    fn over(leaves: Vec<Vec<u8>>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut base = AppendOnlyTree::<RawLeaf>::new();
        let mut hashes = HashTree::new();
        let mut base_roots = vec![base.root()];
        for leaf in &leaves {
            let size = base.append_raw(leaf);
            assert_eq!(hashes.push_leaf_hash(raw_leaf_hash(leaf)), size);
            assert_eq!(hashes.root()?, base.root(), "root at size {size}");
            base_roots.push(base.root());
        }
        Ok(Self {
            leaves,
            base,
            hashes,
            base_roots,
        })
    }

    fn len(&self) -> u64 {
        self.hashes.len()
    }

    /// Holds one inclusion proof to the base tree's bytes and to the verifier.
    fn check_inclusion(&self, index: u64) -> Outcome {
        let ours = self.hashes.prove_inclusion(index)?;
        let theirs = self.base.prove_inclusion(index)?;
        let n = self.len();
        assert_eq!(ours.as_bytes(), theirs.as_bytes(), "inclusion {index} of {n}");
        let leaf = self.leaves.get(usize::try_from(index)?).ok_or("no leaf")?;
        verify_inclusion_raw(&self.base.root(), leaf, index, &ours)?;
        Ok(())
    }

    /// Holds one consistency proof to the base tree's bytes and to the verifier.
    fn check_consistency(&self, old: u64) -> Outcome {
        let n = self.len();
        let ours = self.hashes.prove_consistency(old, n)?;
        let theirs = self.base.prove_consistency(old, n)?;
        assert_eq!(ours.as_bytes(), theirs.as_bytes(), "consistency {old} to {n}");
        let old_root = self.base_roots.get(usize::try_from(old)?).ok_or("no root")?;
        verify_consistency(old_root, &self.base.root(), &ours)?;
        Ok(())
    }
}

#[test]
fn every_proof_over_every_small_log_is_the_base_trees_and_verifies() -> Outcome {
    let mut random = Random(0x6c79_732d_6861_7368);
    let (mut inclusions, mut consistencies) = (0_u64, 0_u64);
    for size in 1..=70 {
        let pair = Pair::over(random.log(size))?;
        for index in 0..size {
            pair.check_inclusion(index)?;
            inclusions += 1;
        }
        for old in 1..=size {
            pair.check_consistency(old)?;
            consistencies += 1;
        }
    }
    assert_eq!(inclusions, 70 * 71 / 2, "every leaf of every size was proved");
    assert_eq!(consistencies, 70 * 71 / 2, "every prefix of every size was proved");
    Ok(())
}

#[test]
fn random_proofs_over_random_larger_logs_are_the_base_trees_and_verify() -> Outcome {
    let mut random = Random(0x7261_6e64_6f6d_2d6c);
    let (mut inclusions, mut consistencies) = (0_u64, 0_u64);
    for _ in 0..24 {
        let size = 71 + random.below(2_000);
        let pair = Pair::over(random.log(size))?;
        for index in [0, size - 1, size / 2] {
            pair.check_inclusion(index)?;
            inclusions += 1;
        }
        for _ in 0..40 {
            pair.check_inclusion(random.below(size))?;
            inclusions += 1;
        }
        for old in [1, size - 1, size, size.next_power_of_two() / 2] {
            pair.check_consistency(old)?;
            consistencies += 1;
        }
        for _ in 0..40 {
            pair.check_consistency(1 + random.below(size))?;
            consistencies += 1;
        }
    }
    assert_eq!(inclusions, 24 * 43);
    assert_eq!(consistencies, 24 * 44);
    Ok(())
}

#[test]
fn a_proof_after_more_appends_is_still_the_base_trees() -> Outcome {
    let mut random = Random(0x6170_7065_6e64_7321);
    let mut hashes = HashTree::new();
    let mut base = AppendOnlyTree::<RawLeaf>::new();
    let mut first = None;
    for (round, size) in [37, 29].into_iter().enumerate() {
        for leaf in random.log(size) {
            hashes.push_leaf_hash(raw_leaf_hash(&leaf));
            base.append_raw(&leaf);
        }
        let proof = hashes.prove_inclusion(5)?;
        assert_eq!(proof.as_bytes(), base.prove_inclusion(5)?.as_bytes());
        assert_eq!(hashes.root()?, base.root());
        if round == 0 {
            first = Some(proof);
        } else {
            let first = first.as_ref().ok_or("no first proof")?;
            assert_ne!(first.as_bytes(), proof.as_bytes(), "the path grew");
        }
    }
    assert_eq!(hashes.len(), 66);
    Ok(())
}

#[test]
fn the_empty_tree_has_the_rfc_empty_root_and_proves_nothing() -> Outcome {
    let hashes = HashTree::new();
    let base = AppendOnlyTree::<RawLeaf>::new();
    assert!(hashes.is_empty());
    assert_eq!(hashes.root()?, base.root());
    let ours = hashes.prove_inclusion(0).err().map(|e| e.to_string());
    assert!(ours.is_some(), "the empty tree refused nothing");
    let theirs = base.prove_inclusion(0).err().map(|e| e.to_string());
    assert_eq!(ours, theirs);
    Ok(())
}

#[test]
fn out_of_range_requests_are_refused_with_the_base_trees_reasons() -> Outcome {
    let pair = Pair::over(Random(7).log(9))?;
    let (hashes, base) = (&pair.hashes, &pair.base);
    let refused = [
        (hashes.prove_inclusion(9).err(), base.prove_inclusion(9).err()),
        (hashes.prove_consistency(0, 9).err(), base.prove_consistency(0, 9).err()),
        (hashes.prove_consistency(10, 9).err(), base.prove_consistency(10, 9).err()),
        (hashes.prove_consistency(3, 10).err(), base.prove_consistency(3, 10).err()),
    ];
    let mut checked = 0;
    for (ours, theirs) in &refused {
        assert!(matches!(ours, Some(TrustError::MerkleTree { .. })), "{ours:?}");
        assert_eq!(
            ours.as_ref().map(ToString::to_string),
            theirs.as_ref().map(ToString::to_string)
        );
        checked += 1;
    }
    assert_eq!(checked, 4);
    Ok(())
}

#[test]
fn a_tree_built_to_its_size_holds_fewer_than_two_hashes_per_leaf() -> Outcome {
    let mut measured = 0;
    for size in [1_u64, 2, 3, 7, 8, 100, 1_023, 1_024, 1_025, 4_097] {
        let mut tree = HashTree::with_capacity(size);
        for index in 0..size {
            tree.push_leaf_hash(raw_leaf_hash(&index.to_le_bytes()));
        }
        let hashes = 2 * size - u64::from(size.count_ones());
        assert_eq!(u64::try_from(tree.retained_bytes())?, 32 * hashes, "size {size}");
        measured += 1;
    }
    assert_eq!(measured, 10);
    Ok(())
}
