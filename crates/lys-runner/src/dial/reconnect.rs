//! A broken server connection is replaced without replaying the failed request.

use crate::error::RunnerError;

pub(super) fn ready<T>(
    mut take: impl FnMut() -> Result<T, RunnerError>,
    disconnected: &mut bool,
    mut wait: impl FnMut(),
) -> Result<T, RunnerError> {
    loop {
        match take() {
            Err(RunnerError::Unreachable { .. }) => {
                if !*disconnected {
                    crate::error::said(
                        "runner_dial_disconnected: the server connection was lost; reconnecting",
                    );
                }
                *disconnected = true;
                wait();
            }
            answer => return answer,
        }
    }
}

pub(super) fn pause() {
    std::thread::sleep(std::time::Duration::from_millis(250));
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_lost_server_connection_retries_but_an_admission_refusal_does_not()
    -> Result<(), crate::RunnerError> {
        let mut reads = 0;
        let mut waits = 0;
        let mut disconnected = false;
        let value = super::ready(
            || {
                reads += 1;
                if reads == 1 {
                    Err(crate::RunnerError::Unreachable {
                        reason: "closed".to_owned(),
                    })
                } else {
                    Ok(42)
                }
            },
            &mut disconnected,
            || waits += 1,
        )?;
        assert_eq!(value, 42);
        assert_eq!(reads, 2);
        assert_eq!(waits, 1);
        assert!(disconnected);
        let refused = super::ready(
            || {
                Err::<(), _>(crate::RunnerError::Dial {
                    reason: "admission refused".to_owned(),
                })
            },
            &mut disconnected,
            || waits += 1,
        );
        assert!(refused.is_err());
        assert_eq!(waits, 1);
        Ok(())
    }
}
