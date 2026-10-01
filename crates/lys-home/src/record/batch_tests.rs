#![cfg(test)]
//! Batch cost, branch preservation and interruption at durable boundaries.

use serde_json::json;

use crate::record::entries::{Entry, EntryBody};
use crate::record::index::{Index, read_head};
use crate::record::{Home, Session};

type Gate = Result<(), Box<dyn std::error::Error>>;

fn message(text: &str) -> EntryBody {
    EntryBody::Message {
        message: json!({"role": "user", "content": text}),
    }
}

#[test]
fn a_batch_preserves_every_body_and_parent_with_four_syncs() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("batch", "/work", None)?;
    let root = session.append(message("root"))?;
    let bodies = vec![message("one"), message("two"), message("three")];
    let before = session.io_counts().syncs;
    let ids = session.append_all(&bodies)?;
    assert_eq!(session.io_counts().syncs - before, 4);
    assert_eq!(ids.len(), bodies.len());
    let mut parent = root.as_str();
    for (id, body) in ids.iter().zip(&bodies) {
        let entry = session.entry(id)?;
        assert_eq!(&entry.body, body);
        assert_eq!(entry.parent_id(), Some(parent));
        parent = id;
    }
    assert_eq!(session.head()?, ids.last().map(String::as_str));
    session.move_head(Some(&root))?;
    let branch = session.append_all(&[message("branch")])?;
    assert_eq!(session.entry(&branch[0])?.parent_id(), Some(root.as_str()));
    assert_eq!(session.path()?.0.len(), 2);
    let file = session.file().to_path_buf();
    drop(session);
    let reopened = Session::open(file)?;
    assert_eq!(reopened.len()?, 5);
    assert_eq!(reopened.head()?, Some(branch[0].as_str()));
    assert!(!reopened.index_was_rebuilt());
    drop(reopened);
    dir.close()?;
    Ok(())
}

#[test]
fn an_empty_batch_writes_nothing_and_keeps_the_head() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("empty-batch", "/work", None)?;
    let before = session.io_counts();
    let bytes = std::fs::read(session.file())?;
    assert!(session.append_all(&[])?.is_empty());
    assert_eq!(session.io_counts(), before);
    assert_eq!(std::fs::read(session.file())?, bytes);
    assert_eq!(session.head()?, None);
    drop(session);
    dir.close()?;
    Ok(())
}

#[test]
fn fifty_single_appends_each_publish_their_own_index_and_head() -> Gate {
    let dir = tempfile::tempdir()?;
    let home = Home::open(dir.path().join("home"))?;
    let mut session = home.create_session("single-appends", "/work", None)?;
    let mut ids = Vec::new();
    for n in 0..50 {
        let before = session.io_counts().syncs;
        let id = session.append(message(&n.to_string()))?;
        assert_eq!(session.io_counts().syncs - before, 4);
        let (_, index, rebuilt) = Index::load(session.file())?;
        assert!(!rebuilt);
        assert!(index.row(&id).is_some());
        assert_eq!(read_head(session.file(), &index)?, Some(id.clone()));
        ids.push(id);
    }
    let file = session.file().to_path_buf();
    drop(session);
    let reopened = Session::open(file)?;
    assert_eq!(reopened.head()?, ids.last().map(String::as_str));
    assert_eq!(
        reopened.path()?.0.iter().map(Entry::id).collect::<Vec<_>>(),
        ids.iter().map(String::as_str).collect::<Vec<_>>()
    );
    drop(reopened);
    dir.close()?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn killing_a_batch_at_each_durable_boundary_never_puts_the_head_ahead_of_a_row() -> Gate {
    use std::io::{BufRead, BufReader};
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};

    for boundary in ["lines", "rows", "head"] {
        let dir = tempfile::tempdir()?;
        let home = Home::open(dir.path().join("home"))?;
        let mut session = home.create_session("interrupted", "/work", None)?;
        let root = session.append(message("root"))?;
        session.append(message("other branch"))?;
        session.move_head(Some(&root))?;
        let file = session.file().to_path_buf();
        drop(session);
        let mut held = HeldChild(Some(
            Command::new(std::env::current_exe()?)
                .args([
                    "--exact",
                    "record::batch_tests::append_batch_child",
                    "--nocapture",
                ])
                .env("LYS_BATCH_CHILD_FILE", &file)
                .env("LYS_BATCH_CHILD_BOUNDARY", boundary)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()?,
        ));
        let stdout = held
            .0
            .as_mut()
            .and_then(|child| child.stdout.take())
            .ok_or("batch child has no output pipe")?;
        let mut output = BufReader::new(stdout);
        let signal = format!("batch:{boundary}");
        loop {
            let mut line = String::new();
            if output.read_line(&mut line)? == 0 {
                return Err(format!("batch child exited before {signal}").into());
            }
            if line.trim() == signal {
                break;
            }
        }
        let child = held.0.as_mut().ok_or("batch child is missing")?;
        child.kill()?;
        let status = child.wait()?;
        held.0 = None;
        assert_eq!(status.signal(), Some(9));
        let reopened = Session::open(&file)?;
        assert_eq!(reopened.len()?, 5);
        let last = reopened
            .index
            .last()
            .ok_or("batch has no last row")?
            .id
            .as_str();
        let expected = if boundary == "head" {
            last
        } else {
            root.as_str()
        };
        assert_eq!(reopened.head()?, Some(expected));
        assert!(reopened.index.row(expected).is_some());
        assert_eq!(reopened.index_was_rebuilt(), boundary == "lines");
        let entries = reopened
            .index
            .rows()
            .iter()
            .map(|row| reopened.entry(&row.id))
            .collect::<Result<Vec<_>, _>>()?;
        let bodies = [message("one"), message("two"), message("three")];
        let mut parent = root.as_str();
        for (entry, body) in entries[2..].iter().zip(&bodies) {
            assert_eq!(&entry.body, body);
            assert_eq!(entry.parent_id(), Some(parent));
            parent = entry.id();
        }
        drop(reopened);
        dir.close()?;
    }
    Ok(())
}

#[cfg(unix)]
struct HeldChild(Option<std::process::Child>);

#[cfg(unix)]
impl Drop for HeldChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            if let Err(error) = child.kill() {
                eprintln!("killing the batch child failed: {error}");
            }
            if let Err(error) = child.wait() {
                eprintln!("reaping the batch child failed: {error}");
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn append_batch_child() -> Gate {
    use crate::record::batch::BatchStep;
    use std::io::{Read, Write};

    let file = match std::env::var_os("LYS_BATCH_CHILD_FILE") {
        Some(file) => file,
        None => return Ok(()),
    };
    let boundary = std::env::var("LYS_BATCH_CHILD_BOUNDARY")?;
    let mut session = Session::open(std::path::PathBuf::from(file))?;
    session.append_all_observed(
        &[message("one"), message("two"), message("three")],
        |step| {
            let name = match step {
                BatchStep::Lines => "lines",
                BatchStep::Rows => "rows",
                BatchStep::Head => "head",
            };
            if name == boundary {
                let mut out = std::io::stdout().lock();
                writeln!(out, "\nbatch:{name}").unwrap();
                out.flush().unwrap();
                std::io::stdin().read_exact(&mut [0]).unwrap();
                panic!("batch child was resumed instead of killed");
            }
        },
    )?;
    Err("batch child did not reach the requested boundary".into())
}
