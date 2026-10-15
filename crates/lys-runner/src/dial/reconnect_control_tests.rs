//! A person's reconnect signal releases an already registered backoff waiter.

#[test]
fn reconnect_ends_backoff_on_its_signal() -> Result<(), Box<dyn std::error::Error>> {
    let wait = std::sync::Arc::new(super::RetryWait::default());
    let waiting = std::sync::Arc::clone(&wait);
    let (entered, ready) = std::sync::mpsc::channel();
    let (checks, checked) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        waiting.wait_observed(
            std::time::Duration::from_secs(30),
            || {
                assert!(entered.send(()).is_ok());
            },
            |generation, pending| {
                assert!(checks.send((generation, pending)).is_ok());
            },
        )
    });
    ready.recv()?;
    let generation = wait.reconnect()?;
    let pending = loop {
        let (observed, pending) = checked.recv()?;
        if observed == generation {
            break pending;
        }
    };
    if pending {
        wait.stop()?;
    }
    let result = worker.join().map_err(|_| "backoff waiter panicked")?;
    assert!(!pending, "Reconnect left the registered backoff active");
    result?;
    Ok(())
}
