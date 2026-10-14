//! Stable UUIDs derived from the home session and entry identity.

use sha1::{Digest, Sha1};

use crate::record::blocks::hex_of;

/// The lys render namespace, fixed: UUID version 5 of the RFC 9562 URL namespace
/// over `lys/home/claude-code/render-uuid/v1`. Every uuid a render derives
/// sits under a session namespace drawn from it; a change to the scheme is a
/// new namespace alongside, never a change to this one.
pub const RENDER_NAMESPACE: [u8; 16] = [
    0x32, 0xc0, 0x59, 0x04, 0xd1, 0xf1, 0x55, 0x0c, 0x9e, 0xee, 0x2f, 0x6c, 0x8f, 0x98, 0xb6, 0x65,
];

/// The role of a derived uuid that is a rendered record's `uuid`. A role
/// never carries `#`, so the last `#` of a name splits it.
pub const ROLE_RECORD: &str = "record";

/// The role of a derived uuid that is the `uuid` of the `compact_boundary`
/// record a compaction renders as, beside its summary record's.
pub const ROLE_BOUNDARY: &str = "compact_boundary";

/// A UUID version 5 (RFC 9562): the SHA-1 of the namespace bytes then the
/// name, its first 16 bytes with the version nibble set to 5 and the variant
/// bits to 10.
#[must_use]
pub fn uuid_v5(namespace: &[u8; 16], name: &[u8]) -> [u8; 16] {
    let mut hasher = Sha1::new();
    hasher.update(namespace);
    hasher.update(name);
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

/// A uuid's lowercase hyphenated form, 8-4-4-4-12.
#[must_use]
pub fn uuid_string(bytes: &[u8; 16]) -> String {
    let hex = hex_of(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// The namespace of one session's derived uuids: UUID version 5 of the render
/// namespace over the session's own id. The session is the salt, so the same
/// entry id in two sessions never derives one uuid; the target session id is
/// not part of the record and never enters it.
#[must_use]
pub fn session_namespace(session_id: &str) -> [u8; 16] {
    uuid_v5(&RENDER_NAMESPACE, session_id.as_bytes())
}

/// Whether an id is 36 characters of hex with `-` at 8, 13, 18 and 23.
fn is_uuid_shaped(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

/// The uuid a record carries for the entry with `id` in the session with
/// `session_id`. A uuid-shaped id is kept as it is, so an imported Claude
/// Code session keeps its source's uuids; any other id (the importer's
/// `<uuid>-r<i>` for a split tool result, a hand-authored id, a canon id)
/// derives as UUID version 5 under the session's namespace over `<id>#record`, so a
/// derived uuid carries version nibble 5 where Claude Code's own carry 4.
/// Nothing random and no clock enters a render: the same session head
/// renders the same bytes every time.
#[must_use]
pub fn record_uuid(session_id: &str, id: &str) -> String {
    if is_uuid_shaped(id) {
        id.to_owned()
    } else {
        derived_uuid(session_id, id, ROLE_RECORD)
    }
}

/// The uuid derived for the entry `id` of session `session_id` in `role`:
/// UUID version 5 under the session's namespace over `<id>#<role>`.
#[must_use]
pub fn derived_uuid(session_id: &str, id: &str, role: &str) -> String {
    let name = format!("{id}#{role}");
    uuid_string(&uuid_v5(&session_namespace(session_id), name.as_bytes()))
}
