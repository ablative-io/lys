use super::*;

#[test]
fn a_regex_match_after_bytes_that_are_not_text_answers_the_byte_cursor() -> TestResult {
    let mut held = Held::start(1 << 16)?;
    let client = held.client();
    started(&client, shell("bytes", "printf '\\377x-marker'; exec cat"))?;
    let act = Act::Wait {
        session: "bytes".to_owned(),
        cursor: Some(0),
        pattern: "x-mark(er)".to_owned(),
        regex: true,
    };
    let Answer::Matched {
        matched, cursor, ..
    } = client.ask(&act)?
    else {
        return Err("the wait answered no match".into());
    };
    assert_eq!(matched, "x-marker");
    assert_eq!(
        cursor, 9,
        "one byte that is not text, then the 8 of x-marker"
    );
    let Answer::Output { output } = client.ask(&Act::Read {
        session: "bytes".to_owned(),
        cursor: Some(cursor),
        lines: None,
        bytes: None,
        follow: false,
    })?
    else {
        return Err("a read from the match's cursor answered no output".into());
    };
    assert_eq!(output.from, 9);
    held.stop()
}

#[test]
fn raw_terminal_preserves_control_invalid_and_split_character_bytes() -> TestResult {
    let mut held = Held::start(4096)?;
    let client = held.client();
    let id = "raw-terminal";
    started(
        &client,
        shell(
            id,
            r"stty raw -echo; printf '\033[31m\377\342'; dd bs=1 count=2 2>/dev/null",
        ),
    )?;
    let expected = b"\x1b[31m\xff\xe2";
    let mut received = Vec::new();
    let mut cursor = 0;
    while received.len() < expected.len() {
        let Answer::Bytes { output } = client.ask(&Act::ReadBytes {
            session: id.to_owned(),
            cursor: Some(cursor),
            follow: true,
        })?
        else {
            return Err("raw read did not answer bytes".into());
        };
        assert_eq!(output.from, cursor);
        assert_eq!(output.cursor - output.from, output.data.len() as u64);
        cursor = output.cursor;
        received.extend(output.data);
    }
    assert_eq!(received, expected);
    let input = vec![0, 0xfe];
    assert!(matches!(
        client.ask(&Act::InputBytes {
            session: id.to_owned(),
            data: input.clone()
        })?,
        Answer::Delivered { .. }
    ));
    let mut echoed = Vec::new();
    loop {
        let Answer::Bytes { output } = client.ask(&Act::ReadBytes {
            session: id.to_owned(),
            cursor: Some(cursor),
            follow: true,
        })?
        else {
            return Err("raw read did not answer bytes".into());
        };
        assert_eq!(output.from, cursor);
        cursor = output.cursor;
        echoed.extend(output.data);
        if output.ended.is_some() {
            break;
        }
    }
    assert_eq!(echoed, input);
    held.stop()
}
