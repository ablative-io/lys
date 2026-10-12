//! Normalize reviewed fixture tokens without changing their meaning.

use std::error::Error;

fn quoted_end(bytes: &[u8], start: usize) -> Result<usize, Box<dyn Error>> {
    let quote = bytes[start];
    let mut end = start + 1;
    while end < bytes.len() {
        if bytes[end] == quote {
            return Ok(end + 1);
        }
        if bytes[end] == b'\\' {
            end += 1;
        }
        end += 1;
    }
    Err("unterminated fixture literal".into())
}

fn raw_end(bytes: &[u8], start: usize) -> Result<Option<usize>, Box<dyn Error>> {
    if bytes[start] != b'r' {
        return Ok(None);
    }
    let mut quote = start + 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return Ok(None);
    }
    let hashes = quote - start - 1;
    let mut end = quote + 1;
    while end < bytes.len() {
        if bytes[end] == b'"'
            && bytes
                .get(end + 1..end + 1 + hashes)
                .is_some_and(|tail| tail.iter().all(|byte| *byte == b'#'))
        {
            return Ok(Some(end + 1 + hashes));
        }
        end += 1;
    }
    Err("unterminated raw fixture literal".into())
}

fn comment_end(bytes: &[u8], start: usize) -> Result<usize, Box<dyn Error>> {
    let mut depth = 1;
    let mut end = start + 2;
    while end + 1 < bytes.len() {
        match &bytes[end..end + 2] {
            b"/*" => {
                depth += 1;
                end += 2;
            }
            b"*/" => {
                depth -= 1;
                end += 2;
                if depth == 0 {
                    return Ok(end);
                }
            }
            _ => end += 1,
        }
    }
    Err("unterminated fixture comment".into())
}

fn import_leaves(tokens: &[Vec<u8>], depth: usize) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    if depth > 32 {
        return Err("fixture import exceeds review bound".into());
    }
    let Some(open) = tokens.iter().position(|token| token == b"{") else {
        let mut leaf = Vec::new();
        for token in tokens {
            leaf.extend(u64::try_from(token.len())?.to_le_bytes());
            leaf.extend(token);
        }
        return Ok(vec![leaf]);
    };
    if tokens.last().is_none_or(|token| token != b"}") {
        return Err("unreviewed fixture import syntax".into());
    }
    let mut leaves = Vec::new();
    let mut start = open + 1;
    let mut nesting = 0;
    for (at, token) in tokens.iter().enumerate().skip(open + 1) {
        if nesting == 0 && (token == b"," || at == tokens.len() - 1) {
            if start < at {
                let joined: Vec<Vec<u8>> = tokens[..open]
                    .iter()
                    .chain(&tokens[start..at])
                    .cloned()
                    .collect();
                leaves.extend(import_leaves(&joined, depth + 1)?);
            }
            start = at + 1;
        }
        if token == b"{" {
            nesting += 1;
        }
        if token == b"}" && nesting > 0 {
            nesting -= 1;
        }
    }
    leaves.sort();
    Ok(leaves)
}

fn normalized_tokens(tokens: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let mut normalized = Vec::new();
    let mut delimiters = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        if tokens[at] == b"use" {
            let mut imports = Vec::new();
            while tokens.get(at).is_some_and(|token| token == b"use") {
                let end = tokens[at..]
                    .iter()
                    .position(|token| token == b";")
                    .ok_or("unterminated fixture import")?
                    + at;
                imports.extend(import_leaves(&tokens[at + 1..end], 0)?);
                at = end + 1;
            }
            imports.sort();
            normalized.push(b"use".to_vec());
            normalized.extend(imports);
            normalized.push(b";".to_vec());
            continue;
        }
        let token = &tokens[at];
        if token == b"(" || token == b"[" || token == b"{" {
            let call = token == b"("
                && at > 0
                && tokens[at - 1]
                    .first()
                    .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_');
            delimiters.push((token.clone(), call, 0_usize));
        }
        let trailing = token == b","
            && tokens.get(at + 1).is_some_and(|next| {
                next == b"}"
                    || next == b"]"
                    || next == b")"
                        && delimiters
                            .last()
                            .is_some_and(|(_, call, commas)| *call || *commas > 0)
            });
        if token == b","
            && let Some((_, _, commas)) = delimiters.last_mut()
        {
            *commas += 1;
        }
        if !trailing {
            normalized.push(token.clone());
        }
        if token == b")" || token == b"]" || token == b"}" {
            let (open, ..) = delimiters.pop().ok_or("unbalanced fixture delimiter")?;
            if !matches!(
                (open.as_slice(), token.as_slice()),
                (b"(", b")") | (b"[", b"]") | (b"{", b"}")
            ) {
                return Err("mismatched fixture delimiter".into());
            }
        }
        at += 1;
    }
    if !delimiters.is_empty() {
        return Err("unclosed fixture delimiter".into());
    }
    Ok(normalized)
}

pub(super) fn source_digest(source: &str) -> Result<[u8; 32], Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    if source.len() > 2_000_000 {
        return Err("fixture source exceeds review bound".into());
    }
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at].is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if bytes.get(at..at + 2) == Some(b"//") {
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
            continue;
        }
        if bytes.get(at..at + 2) == Some(b"/*") {
            at = comment_end(bytes, at)?;
            continue;
        }
        let end = if let Some(end) = raw_end(bytes, at)? {
            end
        } else if bytes[at] == b'"'
            || (bytes[at] == b'\''
                && (bytes.get(at + 1) == Some(&b'\\') || bytes.get(at + 2) == Some(&b'\'')))
        {
            quoted_end(bytes, at)?
        } else if bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_' {
            let mut end = at + 1;
            while bytes
                .get(end)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            {
                end += 1;
            }
            end
        } else if bytes[at].is_ascii() {
            at + 1
        } else {
            return Err("unreviewed non-ASCII fixture syntax".into());
        };
        tokens.push(bytes[at..end].to_vec());
        at = end;
    }
    let mut digest = Sha256::new();
    for token in normalized_tokens(&tokens)? {
        digest.update(u64::try_from(token.len())?.to_le_bytes());
        digest.update(token);
    }
    Ok(digest.finalize().into())
}
