use super::lost_cleanup;
use crate::error::RunnerError;
use crate::pty::Left;
#[test]
fn successful_cleanup_diagnostics_prove_the_lost_group() -> Result<(), Box<dyn std::error::Error>> {
    let diagnostic = "EPERM errno 1; member exit observed";
    assert_eq!(
        lost_cleanup(Ok(Left::Ended {
            reason: Some(diagnostic.to_owned())
        }))?,
        Some(format!("control_group_cleanup_proved: {diagnostic}"))
    );
    for left in [
        Left::Unended {
            reason: "still live".to_owned(),
        },
        Left::Reused,
    ] {
        let error = lost_cleanup(Ok(left))
            .err()
            .ok_or("unproved cleanup passed")?;
        assert_eq!(error.name(), "control_process_end_unconfirmed");
    }
    let error = lost_cleanup(Err(RunnerError::refused(
        "process_group_unreadable",
        "denied",
    )))
    .err()
    .ok_or("unreadable cleanup passed")?;
    assert_eq!(error.name(), "process_group_unreadable");
    Ok(())
}
