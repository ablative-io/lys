#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn capability_claims_oid_extends_the_lys_arc_by_one() {
    let oid = capability_claims_oid();
    assert_eq!(oid, vec![1, 3, 6, 1, 4, 1, 66364, 1]);
    assert_eq!(&oid[..LYS_OID_ARC.len()], LYS_OID_ARC);
}

#[test]
fn terminal_safety_rejects_escape_and_carriage_control() {
    assert!(is_terminal_safe("{\"role\": \"reviewer\"}"));
    assert!(is_terminal_safe("multi\nline\tclaims"));
    assert!(!is_terminal_safe("claims \u{1b}[2K\u{1b}[1A spoofed"));
    assert!(!is_terminal_safe("overwrite\rme"));
    assert!(!is_terminal_safe("bell\u{07}"));
}
