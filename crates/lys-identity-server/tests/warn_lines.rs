//! DIRECTORY-095: the subscriber installed as main.rs installs it says a
//! warning raised on any thread, and a second install is refused by name.

use std::error::Error;
use std::sync::{Arc, Mutex};

use lys_identity_server::routes::Say;
use lys_identity_server::warn_lines;

#[test]
fn the_installed_subscriber_says_a_warning_from_any_thread() -> Result<(), Box<dyn Error>> {
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    let lines = Arc::clone(&heard);
    let say: Say = Arc::new(move |line: &str| match lines.lock() {
        Ok(mut lines) => lines.push(line.to_owned()),
        Err(poisoned) => panic!("warn_lines_fixture_poisoned: {poisoned}"),
    });
    warn_lines::install(Arc::clone(&say))?;
    std::thread::spawn(|| {
        tracing::warn!(target: "lys_identity_server::grants", reason = "the log refused", "settle failed");
        tracing::info!("said nowhere");
    })
    .join()
    .map_err(|panicked| format!("the warning thread panicked: {panicked:?}"))?;
    let said = heard
        .lock()
        .map_err(|poisoned| poisoned.to_string())?
        .clone();
    assert_eq!(
        said,
        vec!["WARN lys_identity_server::grants: settle failed reason=the log refused".to_owned()]
    );
    let again = warn_lines::install(say);
    let Err(refused) = again else {
        return Err("a second subscriber was installed".into());
    };
    assert!(
        refused.to_string().starts_with("log_subscriber_installed:"),
        "{refused}"
    );
    Ok(())
}
