#![cfg(test)]
//! An end answer needs the child's exit evidence, including signal exits.

use lys_runner::{Ended, EndedHow};

#[test]
fn exit_requires_a_reaped_status_or_signal() -> Result<(), Box<dyn std::error::Error>> {
    let mut ended = Ended {
        how: EndedHow::Exited,
        at: 1,
        status: None,
        signal: None,
        reason: None,
        stopped: None,
    };
    let refused = ended
        .require_reaped("unreaped")
        .err()
        .ok_or("unreaped end accepted")?;
    assert_eq!(refused.name(), "process_exit_unconfirmed");
    assert!(refused.to_string().contains("unreaped"));
    ended.status = Some(0);
    ended.require_reaped("normal")?;
    ended.status = None;
    ended.signal = Some("SIGHUP".to_owned());
    ended.require_reaped("signalled")?;
    ended.at = 0;
    assert!(ended.require_reaped("no-instant").is_err());
    Ok(())
}
