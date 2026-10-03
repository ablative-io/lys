//! The acts a verified request asks for: run, read, write, wait, end and
//! status, each answered on the sessions it names.

use crate::admitted::Admitted;
use crate::error::RunnerError;
use crate::protocol::{Act, Answer, Greeting, Output, verify_request};
use crate::scrollback::whole_text;
use crate::session::Sessions;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// The answer to request `line`, made on the connection given `greeting`.
pub fn dispatch(
    sessions: &Arc<Sessions>,
    server: &[u8; 32],
    greeting: &Greeting,
    line: &str,
    left: &AtomicBool,
) -> Answer {
    dispatch_for(sessions, server, greeting, line, left, None)
}

/// Dispatch a signed server request with an optional verified input caller.
pub fn dispatch_for(
    sessions: &Arc<Sessions>,
    server: &[u8; 32],
    greeting: &Greeting,
    line: &str,
    left: &AtomicBool,
    context: Option<&crate::legacy_input::InputContext<'_>>,
) -> Answer {
    let act = match verify_request(line.trim_end(), server, greeting) {
        Ok(act) => act,
        Err(error) => return Answer::refusal(&error),
    };
    perform(sessions, server, act, left, context).unwrap_or_else(|error| Answer::refusal(&error))
}

pub(super) fn perform(
    sessions: &Arc<Sessions>,
    server: &[u8; 32],
    act: Act,
    left: &AtomicBool,
    context: Option<&crate::legacy_input::InputContext<'_>>,
) -> Result<Answer, RunnerError> {
    match act {
        Act::ReadBytes {
            session,
            cursor,
            follow,
        } => crate::terminal_bytes::read(sessions, &session, cursor, follow, left),
        Act::InputBytes { session, data } => {
            crate::legacy_input::write(sessions, server, &session, &data, context)
                .map(|()| Answer::Delivered { session })
        }
        Act::Start {
            mut launch,
            lys_mcp,
        } => {
            if let Some(entry) = lys_mcp {
                crate::launch_config::add_lys_mcp(&mut launch, &entry)?;
            }
            let session = launch.session.clone();
            let policy = launch
                .policy
                .clone()
                .map(|admitted| Admitted::verified(*admitted))
                .transpose()?;
            let (pid, started_at) = sessions.begin(*launch, policy, None)?;
            Ok(Answer::Started {
                session,
                pid,
                started_at,
            })
        }
        Act::Input {
            session,
            text,
            enter,
        } => {
            let mut bytes = text.into_bytes();
            if enter {
                bytes.extend_from_slice(crate::protocol::Key::Enter.bytes());
            }
            crate::legacy_input::write(sessions, server, &session, &bytes, context)
                .map(|()| Answer::Delivered { session })
        }
        Act::Keys { session, keys } => {
            let bytes: Vec<u8> = keys
                .iter()
                .flat_map(|key| key.bytes().iter().copied())
                .collect();
            crate::legacy_input::write(sessions, server, &session, &bytes, context)
                .map(|()| Answer::Delivered { session })
        }
        Act::Read {
            session,
            cursor,
            lines,
            bytes,
            follow,
        } => read(sessions, &session, (cursor, lines, bytes), follow, left),
        Act::Wait {
            session,
            cursor,
            pattern,
            regex,
        } => wait(sessions, &session, cursor, &pattern, regex, left),
        Act::Resize {
            session,
            columns,
            rows,
        } => sessions
            .resize(&session, columns, rows)
            .map(|()| Answer::Delivered { session }),
        Act::End { session } => {
            let ended = sessions.end(&session, left)?;
            Ok(Answer::Ended { session, ended })
        }
        Act::Status { session } => Ok(Answer::Status {
            status: sessions.status(session.as_deref())?,
        }),
        Act::Operate { operation } => {
            let outcome = if matches!(
                operation.request,
                crate::operations::OperationRequest::Compact { .. }
            ) {
                crate::legacy_input::compact(sessions, server, operation, context)?
            } else {
                sessions.operate(operation)?
            };
            Ok(Answer::Operation { outcome })
        }
        Act::Feed { cursor, follow } => {
            if follow {
                sessions.until_any(left, |table| {
                    table
                        .feed
                        .after(cursor.as_deref())
                        .map_or(Some(()), |more| more.then_some(()))
                })?;
            }
            let reader = sessions.lock()?.feed.reader();
            let page = reader.page(cursor.as_deref())?;
            Ok(Answer::Feed { page })
        }
        Act::Folders { under } => crate::folders::inside(under.as_deref()),
        Act::AsCaller { caller, done } => as_caller(sessions, server, &caller, *done, left),
        Act::GrantChannel => Err(RunnerError::refused(
            "grant_channel_unheld",
            "a grant channel is held only as the whole of its connection",
        )),
        Act::Withdraw { operation, why } => sessions
            .withdraw(&operation, &why)
            .map(|outcome| Answer::Operation { outcome }),
        Act::Outcome { operation } => sessions
            .outcome(&operation)
            .map(|outcome| Answer::Operation { outcome }),
    }
}

/// Do `act` for `caller`: a start owned by them, or typed input and an
/// operation attributed to them. The server signed this request only after
/// verifying the caller and judging their grant, so its signature is the
/// judgement the runner holds them to; anything else is refused by name.
fn as_caller(
    sessions: &Arc<Sessions>,
    server: &[u8; 32],
    caller: &str,
    act: Act,
    left: &AtomicBool,
) -> Result<Answer, RunnerError> {
    match act {
        Act::Start {
            mut launch,
            lys_mcp,
        } => {
            if let Some(entry) = lys_mcp {
                crate::launch_config::add_lys_mcp(&mut launch, &entry)?;
            }
            let session = launch.session.clone();
            let policy = launch
                .policy
                .clone()
                .map(|admitted| Admitted::verified(*admitted))
                .transpose()?;
            let (pid, started_at) = sessions.begin_for(*launch, policy, None, caller)?;
            Ok(Answer::Started {
                session,
                pid,
                started_at,
            })
        }
        act @ (Act::Input { .. }
        | Act::InputBytes { .. }
        | Act::Keys { .. }
        | Act::Operate { .. }) => {
            let judge = |_signed: &crate::injection::InputGrant<'_>| Ok::<(), RunnerError>(());
            let context = crate::legacy_input::InputContext {
                sender: caller,
                signed: false,
                cookie: None,
                judge: &judge,
            };
            perform(sessions, server, act, left, Some(&context))
        }
        _ => Err(RunnerError::refused(
            "caller_act_unsupported",
            "only a start, typed input or an operation is done for a caller",
        )),
    }
}

/// Where a read begins: a cursor, the last lines, or the last bytes.
pub(super) type Start = (Option<u64>, Option<u32>, Option<u64>);

pub(super) fn read(
    sessions: &Sessions,
    id: &str,
    (cursor, lines, bytes): Start,
    follow: bool,
    left: &AtomicBool,
) -> Result<Answer, RunnerError> {
    sessions.until(id, left, |session, id| {
        let scrollback = session.scrollback();
        let from = match (cursor, lines, bytes) {
            (Some(cursor), _, _) => cursor,
            (None, Some(lines), _) => scrollback.last_lines(lines),
            (None, None, Some(bytes)) => scrollback.last_bytes(bytes),
            (None, None, None) => scrollback.oldest(),
        };
        if from > scrollback.end() {
            return Some(Err(RunnerError::refused(
                "cursor_ahead",
                format!(
                    "cursor {from} is past the end of the output, {}",
                    scrollback.end()
                ),
            )));
        }
        let kept = match scrollback.from(from) {
            Ok(kept) => kept,
            Err(expired) => return Some(Err(expired)),
        };
        let (text, given) = whole_text(&kept);
        let ended = session.ended();
        if follow && given == 0 && ended.is_none() {
            return None;
        }
        Some(Ok(Answer::Output {
            output: Output {
                session: id.to_owned(),
                from,
                cursor: from + given as u64,
                oldest: scrollback.oldest(),
                text,
                ended,
            },
        }))
    })
}

/// The offset in `bytes` that offset `at` of their lossy text stands for:
/// each replacement character stands for the whole invalid sequence it
/// replaced, so a cursor after a match counts the bytes the session gave.
pub(super) fn byte_at(bytes: &[u8], at: usize) -> usize {
    let (mut text, mut byte) = (0, 0);
    for chunk in bytes.utf8_chunks() {
        let valid = chunk.valid().len();
        if at <= text + valid {
            return byte + (at - text);
        }
        text += valid;
        byte += valid;
        if !chunk.invalid().is_empty() {
            text += char::REPLACEMENT_CHARACTER.len_utf8();
            byte += chunk.invalid().len();
            if at <= text {
                return byte;
            }
        }
    }
    byte
}

/// Where `needle` first begins in `haystack`.
pub(super) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(super) fn wait(
    sessions: &Sessions,
    id: &str,
    cursor: Option<u64>,
    pattern: &str,
    regex: bool,
    left: &AtomicBool,
) -> Result<Answer, RunnerError> {
    if pattern.is_empty() {
        return Err(RunnerError::refused(
            "pattern_invalid",
            "the pattern is empty",
        ));
    }
    let expression = if regex {
        Some(
            regex_lite::Regex::new(pattern)
                .map_err(|error| RunnerError::refused("pattern_invalid", error.to_string()))?,
        )
    } else {
        None
    };
    let mut start = cursor;
    sessions.until(id, left, |session, id| {
        let scrollback = session.scrollback();
        let from = *start.get_or_insert(scrollback.end());
        let kept = match scrollback.from(from.min(scrollback.end())) {
            Ok(kept) => kept,
            Err(expired) => return Some(Err(expired)),
        };
        let found = match &expression {
            Some(expression) => {
                let text = String::from_utf8_lossy(&kept);
                expression
                    .find(&text)
                    .map(|matched| (matched.as_str().to_owned(), byte_at(&kept, matched.end())))
            }
            None => {
                find(&kept, pattern.as_bytes()).map(|at| (pattern.to_owned(), at + pattern.len()))
            }
        };
        if let Some((matched, end)) = found {
            return Some(Ok(Answer::Matched {
                session: id.to_owned(),
                matched,
                cursor: from.min(scrollback.end()) + end as u64,
            }));
        }
        session.ended().map(|ended| {
            Err(RunnerError::refused(
                "session_ended",
                format!(
                    "session {id} ended ({:?}) before the pattern appeared",
                    ended.how
                ),
            ))
        })
    })
}
