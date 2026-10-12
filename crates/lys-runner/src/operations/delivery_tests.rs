use crate::Sessions;
use crate::operations::{Operation, OperationRequest, OperationState};
use std::error::Error;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

struct Recording(Arc<Mutex<Vec<Vec<u8>>>>);
impl Write for Recording {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|error| io::Error::other(error.to_string()))?
            .push(bytes.to_vec());
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn seatfix_text_then_enter_are_separate_writes_with_one_durable_delivery()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let launch = serde_json::from_value(serde_json::json!({
        "session":"seatfix", "program":"/bin/cat", "arguments":[], "directory":"/",
        "environment":{}, "columns":80, "rows":24
    }))?;
    sessions.start(launch)?;
    let writes = Arc::new(Mutex::new(Vec::new()));
    let original = {
        let mut table = sessions.lock()?;
        let live = table
            .sessions
            .get_mut("seatfix")
            .ok_or("no session")?
            .live("seatfix")?;
        std::mem::replace(
            &mut live.writer,
            crate::input::Input::new(Box::new(Recording(Arc::clone(&writes)))),
        )
    };
    let operation = Operation {
        operation: "seatfix-notice".to_owned(),
        session: "seatfix".to_owned(),
        request: OperationRequest::Notice {
            text: "seat message".to_owned(),
        },
    };
    let result = (|| {
        let mut table = sessions.lock()?;
        super::accept(&mut table, operation.clone())?;
        drop(table);
        let receipt = sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
            table
                .operations
                .get("seatfix-notice")
                .filter(|held| held.state == OperationState::Delivered)
                .cloned()
        })?;
        let mut table = sessions.lock()?;
        assert_eq!(
            super::accept(&mut table, operation)?.state,
            OperationState::Delivered
        );
        assert_eq!(receipt.text.as_ref().ok_or("no text digest")?.length, 12);
        drop(table);
        assert_eq!(
            *writes.lock().map_err(|error| error.to_string())?,
            vec![b"seat message".to_vec(), vec![13]]
        );
        Ok::<_, Box<dyn Error>>(())
    })();
    sessions.stop_all()?;
    drop(original);
    result?;
    drop(sessions);
    let recovered = Sessions::open(dir.path(), 4096)?;
    let table = recovered.lock()?;
    assert_eq!(
        table
            .operations
            .get("seatfix-notice")
            .ok_or("no durable receipt")?
            .state,
        OperationState::Delivered
    );
    Ok(())
}
