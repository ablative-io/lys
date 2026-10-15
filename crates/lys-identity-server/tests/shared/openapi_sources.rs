//! Read test modules without counting refusal names from production code.

use std::error::Error;
use std::path::Path;

fn quoted_end(bytes: &[u8], mut at: usize, quote: u8) -> usize {
    at += 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            byte if byte == quote => return at + 1,
            _ => at += 1,
        }
    }
    bytes.len()
}

fn raw_end(bytes: &[u8], at: usize) -> Option<usize> {
    let mut quote = at + 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return None;
    }
    let hashes = quote - at - 1;
    let mut end = quote + 1;
    while end < bytes.len() {
        if bytes[end] == b'"'
            && bytes
                .get(end + 1..end + 1 + hashes)
                .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
        {
            return Some(end + 1 + hashes);
        }
        end += 1;
    }
    Some(bytes.len())
}

fn mask(source: &str) -> Vec<u8> {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut at = 0;
    while at < bytes.len() {
        let end = match bytes[at] {
            b'/' if bytes.get(at + 1) == Some(&b'/') => bytes[at..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |length| at + length),
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                let mut end = at + 2;
                let mut depth = 1;
                while end + 1 < bytes.len() && depth != 0 {
                    match &bytes[end..end + 2] {
                        b"/*" => {
                            depth += 1;
                            end += 2;
                        }
                        b"*/" => {
                            depth -= 1;
                            end += 2;
                        }
                        _ => end += 1,
                    }
                }
                if depth == 0 { end } else { bytes.len() }
            }
            b'r' => raw_end(bytes, at).unwrap_or(at),
            b'"' => quoted_end(bytes, at, b'"'),
            b'\'' => {
                let length = source[at + 1..].chars().next().map_or(0, char::len_utf8);
                if bytes.get(at + 1) == Some(&b'\\') {
                    quoted_end(bytes, at, b'\'')
                } else if bytes.get(at + 1 + length) == Some(&b'\'') {
                    at + 2 + length
                } else {
                    at
                }
            }
            _ => at,
        };
        if end > at {
            masked[at..end].fill(b' ');
            at = end;
        } else {
            at += 1;
        }
    }
    masked
}

pub(super) fn inline_tests(source: &str) -> Result<String, Box<dyn Error>> {
    let masked = mask(source);
    let code = std::str::from_utf8(&masked)?;
    let mut found = String::new();
    let mut offset = 0;
    while let Some(relative) = code[offset..].find("#[cfg(test)]") {
        let start = offset + relative + "#[cfg(test)]".len();
        offset = start;
        let rest = code[start..].trim_start();
        if !rest.starts_with("mod ") {
            continue;
        }
        let Some(open) = rest.find(['{', ';']) else {
            continue;
        };
        if rest.as_bytes()[open] == b';' {
            continue;
        }
        let begin = code.len() - rest.len() + open;
        let mut depth = 0;
        for (index, byte) in masked.iter().enumerate().skip(begin) {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        found.push_str(&source[begin..=index]);
                        offset = index + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(found)
}

pub(super) fn read(root: &Path) -> Result<String, Box<dyn Error>> {
    let src = root.join("src");
    let mut dirs = vec![
        root.join("tests"),
        root.join("../../tests/identity_contract/tests"),
        src.clone(),
    ];
    let mut text = String::new();
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("non-text source name")?;
            if path.extension().is_none_or(|extension| extension != "rs") || name == "openapi.rs" {
                continue;
            }
            let source = std::fs::read_to_string(&path)?;
            if !path.starts_with(&src)
                || name.ends_with("_tests.rs")
                || name == "tests.rs"
                || path
                    .strip_prefix(&src)?
                    .components()
                    .any(|part| part.as_os_str() == "tests")
            {
                text.push_str(&source);
            } else {
                text.push_str(&inline_tests(&source)?);
            }
        }
    }
    Ok(text)
}
