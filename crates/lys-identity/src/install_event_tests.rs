use super::{InstallChange, InstallEvent, decode, encode};
use crate::error::IdentityError;

const EARLIER: &str = "http://localhost:18080/auth/v1/";
const MOVED: &str = "http://localhost:8490/auth/v1/";

fn moved() -> Result<InstallEvent, IdentityError> {
    InstallEvent::issuer_moved(EARLIER, MOVED, "f3372d8b", 1_790_850_000)
}

#[test]
fn an_install_event_decodes_to_itself_from_its_canonical_bytes() -> Result<(), IdentityError> {
    let event = moved()?;
    assert_eq!(decode(&encode(&event))?, event);
    Ok(())
}

#[test]
fn the_same_move_has_one_operation_id_and_another_move_another() -> Result<(), IdentityError> {
    let again = InstallEvent::issuer_moved(EARLIER, MOVED, "a-later-build", 2)?;
    assert_eq!(moved()?.operation(), again.operation());
    let back = InstallEvent::issuer_moved(MOVED, EARLIER, "f3372d8b", 1)?;
    assert_ne!(moved()?.operation(), back.operation());
    Ok(())
}

#[test]
fn a_changed_actor_subject_kind_or_padding_never_decodes() -> Result<(), IdentityError> {
    let body = encode(&moved()?);
    // Key 1 of the actor and of the subject holds code 4, and key 6 holds the
    // change kind 10; each changed code is refused.
    let mut changed_codes = 0;
    for (key, code) in [(0x01u8, 0x04u8), (0x06, 0x0a)] {
        for at in (0..body.len() - 1).filter(|&at| body[at..at + 2] == [key, code]) {
            let mut changed = body.clone();
            changed[at + 1] = code + 1;
            assert!(decode(&changed).is_err(), "key {key} code {code} at {at}");
            changed_codes += 1;
        }
    }
    assert_eq!(
        changed_codes, 3,
        "the actor, the subject and the change kind"
    );
    let mut padded = body;
    padded.push(0);
    assert!(decode(&padded).is_err());
    Ok(())
}

#[test]
fn an_operation_id_that_is_not_the_moves_own_never_decodes() -> Result<(), IdentityError> {
    let event = moved()?;
    let mut body = encode(&event);
    let operation = event.operation();
    let at = body
        .windows(16)
        .position(|window| window == operation.as_bytes())
        .ok_or(IdentityError::EventMalformed {
            reason: "the operation id is written",
        })?;
    body[at] ^= 1;
    assert!(decode(&body).is_err());
    let InstallChange::IssuerMoved { from, to } = event.change();
    assert_eq!((from.as_str(), to.as_str()), (EARLIER, MOVED));
    Ok(())
}
