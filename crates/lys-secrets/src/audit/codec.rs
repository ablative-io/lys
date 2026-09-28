//! How an audit line is written into its leaf and read back: the line in a
//! canonical encoding, then the line and its signature in another.

use lys_core::Ed25519Identity;

use super::{AuditKind, AuditLine, LINE_DOMAIN, SIGNED_DOMAIN};
use crate::encoding::{Canonical, Reader};
use crate::error::SecretsError;

fn optional(encoding: &mut Canonical, value: Option<&str>) -> Result<(), SecretsError> {
    match value {
        None => encoding.field(&[0]),
        Some(text) => {
            let mut bytes = Vec::with_capacity(text.len() + 1);
            bytes.push(1);
            bytes.extend_from_slice(text.as_bytes());
            encoding.field(&bytes)
        }
    }
    .map(|_| ())
}

fn read_optional(reader: &mut Reader<'_>) -> Result<Option<String>, SecretsError> {
    let field = reader.field()?;
    match field.split_first() {
        Some((0, [])) => Ok(None),
        Some((1, text)) => {
            String::from_utf8(text.to_vec())
                .map(Some)
                .ok()
                .ok_or_else(|| SecretsError::Encoding {
                    context: "audit line",
                    reason: "a text field is not UTF-8".to_owned(),
                })
        }
        _ => Err(SecretsError::Encoding {
            context: "audit line",
            reason: "an optional field is malformed".to_owned(),
        }),
    }
}

pub(super) fn encode_line(line: &AuditLine) -> Result<Vec<u8>, SecretsError> {
    let mut encoding = Canonical::new(LINE_DOMAIN)?;
    encoding.field(line.kind.label().as_bytes())?;
    encoding.field(&line.at_ms.to_be_bytes())?;
    optional(&mut encoding, line.handle.as_deref())?;
    optional(&mut encoding, line.identity.as_deref())?;
    optional(&mut encoding, line.secret.as_deref())?;
    optional(&mut encoding, line.operation.as_deref())?;
    optional(&mut encoding, line.request.as_deref())?;
    optional(
        &mut encoding,
        line.uses.map(|uses| uses.to_string()).as_deref(),
    )?;
    optional(
        &mut encoding,
        line.spend.map(|spend| spend.to_string()).as_deref(),
    )?;
    encoding.field(line.outcome.as_bytes())?;
    Ok(encoding.into_bytes())
}

pub(super) fn decode_signed(
    index: u64,
    bytes: &[u8],
    key: &[u8; 32],
) -> Result<AuditLine, SecretsError> {
    let unreadable = |reason: String| SecretsError::AuditLineUnreadable { index, reason };
    let mut signed = Reader::new(bytes, "signed audit line");
    if signed.field()? != SIGNED_DOMAIN.as_bytes() {
        return Err(unreadable("not a signed audit line".to_owned()));
    }
    let body = signed.field()?;
    let signature = signed.field()?;
    Ed25519Identity::verify(key, body, signature)
        .ok()
        .ok_or(SecretsError::AuditSignatureInvalid { index })?;
    let mut reader = Reader::new(body, "audit line");
    if reader.field()? != LINE_DOMAIN.as_bytes() {
        return Err(unreadable("not an audit line".to_owned()));
    }
    let kind =
        AuditKind::parse(reader.field()?).ok_or_else(|| unreadable("unknown kind".to_owned()))?;
    let at: [u8; 8] = reader
        .field()?
        .try_into()
        .ok()
        .ok_or_else(|| unreadable("the time is not 8 bytes".to_owned()))?;
    let handle = read_optional(&mut reader)?;
    let identity = read_optional(&mut reader)?;
    let secret = read_optional(&mut reader)?;
    let operation = read_optional(&mut reader)?;
    let request = read_optional(&mut reader)?;
    let uses = read_optional(&mut reader)?
        .map(|text| {
            text.parse::<u64>()
                .ok()
                .ok_or_else(|| unreadable("the use count is not a number".to_owned()))
        })
        .transpose()?;
    let spend = read_optional(&mut reader)?
        .map(|text| {
            text.parse::<u64>()
                .ok()
                .ok_or_else(|| unreadable("the spend is not a number".to_owned()))
        })
        .transpose()?;
    let outcome = String::from_utf8(reader.field()?.to_vec())
        .ok()
        .ok_or_else(|| unreadable("the outcome is not UTF-8".to_owned()))?;
    Ok(AuditLine {
        kind,
        at_ms: i64::from_be_bytes(at),
        handle,
        identity,
        secret,
        operation,
        request,
        uses,
        spend,
        outcome,
    })
}
