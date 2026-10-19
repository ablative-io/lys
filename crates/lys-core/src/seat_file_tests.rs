#![cfg(test)]
//! AGENTS-017 R5: a seat file written by the writer is read back whole by the
//! reader, in the member names Cambium's door reads; a file of another version
//! is refused naming the version; the key is never in `Debug`.

use zeroize::Zeroizing;

use super::{
    DoorSeat, SEAT_FILE_VERSION, SeatFile, SeatFileError, VersionFound, identity_from_pem,
};
use crate::keys::Ed25519Identity;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const AGENT: &str = "agent-00000000000000000000000000000017";

fn written(door: Option<DoorSeat>) -> SeatFile {
    let mut seat = SeatFile::new(
        &Zeroizing::new([17; 32]),
        ("https://lys.example.test/api", AGENT),
        door,
    );
    seat.certificate_pem = Some("-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n".to_owned());
    seat.serial = Some("op-00000000000000000000000000000001".to_owned());
    seat
}

#[test]
fn a_written_seat_file_is_read_back_whole_in_the_doors_member_names() -> TestResult {
    let door = DoorSeat {
        door_url: "https://cambium.example.test".to_owned(),
        handle: "scott".to_owned(),
        device: "scotts-laptop".to_owned(),
    };
    let seat = written(Some(door.clone()));
    let text = seat.to_json()?;
    let members: serde_json::Value = serde_json::from_str(&text)?;
    for member in [
        "version",
        "doorUrl",
        "handle",
        "device",
        "privateKeyPem",
        "lysUrl",
        "lysAgentId",
        "certificatePem",
        "serial",
    ] {
        assert!(members.get(member).is_some(), "{member} is missing");
    }
    assert_eq!(members["version"], SEAT_FILE_VERSION);
    assert_eq!(members["lysAgentId"], AGENT);
    let read = SeatFile::from_json(&text)?;
    assert_eq!(read, seat, "read back whole");
    assert_eq!(read.door(), Some(door));
    let expected = Ed25519Identity::from_seed(&Zeroizing::new([17; 32])).public_key_bytes();
    assert_eq!(read.identity()?.public_key_bytes(), expected);
    Ok(())
}

#[test]
fn a_seat_no_door_founded_writes_none_of_the_doors_members() -> TestResult {
    let seat = written(None);
    let members: serde_json::Value = serde_json::from_str(&seat.to_json()?)?;
    for member in ["doorUrl", "handle", "device"] {
        assert!(members.get(member).is_none(), "{member} was written");
    }
    let read = SeatFile::from_json(&seat.to_json()?)?;
    assert_eq!(read.door(), None);
    assert_eq!(read, seat);
    Ok(())
}

#[test]
fn a_file_of_another_version_is_refused_naming_the_version() -> TestResult {
    let mut members: serde_json::Value = serde_json::from_str(&written(None).to_json()?)?;
    members["version"] = serde_json::json!(2);
    let refused = SeatFile::from_json(&members.to_string())
        .err()
        .ok_or("version 2 was read")?;
    assert_eq!(
        refused,
        SeatFileError::VersionUnknown {
            found: VersionFound(Some(2))
        }
    );
    assert!(refused.to_string().contains("is version 2"), "{refused}");
    let missing = SeatFile::from_json(r#"{"privateKeyPem":"x"}"#)
        .err()
        .ok_or("a file with no version was read")?;
    assert_eq!(
        missing,
        SeatFileError::VersionUnknown {
            found: VersionFound(None)
        }
    );
    let words = missing.to_string();
    assert!(words.starts_with("seat_file_version_unknown"), "{words}");
    assert!(words.contains("carries no version"), "{words}");
    assert!(words.contains(&format!("reads version {SEAT_FILE_VERSION}")), "{words}");
    let torn = SeatFile::from_json("{\"version\": 1,").err().ok_or("torn JSON was read")?;
    assert!(matches!(torn, SeatFileError::Unreadable { .. }), "{torn}");
    Ok(())
}

#[test]
fn the_key_is_never_in_debug_or_in_a_refusal() -> TestResult {
    let seat = written(None);
    let pem = seat.private_key_pem.clone();
    let body = pem
        .lines()
        .find(|line| !line.starts_with("-----"))
        .ok_or("the PEM has no body")?
        .to_owned();
    assert!(!format!("{seat:?}").contains(&body), "Debug shows the key");
    let torn = seat.to_json()?.replace("\"version\": 1", "\"version\": \"one\"");
    let refused = SeatFile::from_json(&torn).err().ok_or("a torn file was read")?;
    assert!(!refused.to_string().contains(&body), "the refusal shows the key");
    Ok(())
}

#[test]
fn a_pem_that_is_not_an_ed25519_key_is_refused_by_name() {
    for text in ["not pem at all", "-----BEGIN PRIVATE KEY-----\nAAEC\n-----END PRIVATE KEY-----\n"] {
        assert!(
            matches!(identity_from_pem(text), Err(SeatFileError::KeyUnreadable { .. })),
            "{text} was read as a key"
        );
    }
}
