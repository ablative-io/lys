use super::{Nonces, remember_nonce};
use std::error::Error;

#[test]
fn a_poisoned_nonce_store_cannot_admit_a_signature() -> Result<(), Box<dyn Error>> {
    let nonces = Nonces::default();
    let poisoned = std::panic::catch_unwind(|| {
        let held = nonces
            .lock()
            .expect("fixture lock poisoned before injection");
        assert!(held.is_empty());
        panic!("signature nonce state failure");
    });
    assert!(poisoned.is_err());
    let refused = remember_nonce(&nonces, "00112233445566778899aabbccddeeff", 1, 1)
        .err()
        .ok_or("poisoned nonces admitted signature")?;
    assert_eq!(refused.name(), "AgentSignatureRefused");
    assert!(refused.to_string().contains("nonce store"));
    Ok(())
}
