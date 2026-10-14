//! Replay admission stays bounded within an epoch without forgetting a used nonce.

use lys_identity_server::runner_client::DialHub;

#[test]
fn a_dial_epoch_refuses_capacity_exhaustion_and_still_rejects_a_replayed_nonce()
-> Result<(), Box<dyn std::error::Error>> {
    let hub = DialHub::default();
    assert!(hub.fresh("computer", "first")?);
    for index in 1..65_536 {
        assert!(hub.fresh("computer", &format!("nonce-{index}"))?);
    }
    assert!(!hub.fresh("computer", "first")?);
    let error = hub
        .fresh("computer", "overflow")
        .err()
        .ok_or("replay memory grew past its bound")?;
    assert_eq!(error.name(), "runner_dial_capacity");
    Ok(())
}
