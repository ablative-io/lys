#![cfg(test)]
//! Current rollout discovery does not walk older session directories.

use std::error::Error;

#[test]
fn a_current_rollout_touches_only_its_date_directory() -> Result<(), Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    let now = crate::session::now_ms();
    let date = jiff::Timestamp::from_millisecond(i64::try_from(now)?)?
        .to_zoned(jiff::tz::TimeZone::system())
        .date();
    let current = home.path().join(format!(
        "sessions/{:04}/{:02}/{:02}",
        date.year(),
        date.month(),
        date.day()
    ));
    std::fs::create_dir_all(&current)?;
    for year in 1990..2020 {
        std::fs::create_dir_all(home.path().join(format!("sessions/{year}/01/01")))?;
    }
    let path = current.join("rollout-current-thread.jsonl");
    std::fs::write(
        &path,
        b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"thread\"}}\n",
    )?;
    super::ROLLOUT_DIRECTORIES.with(|count| count.set(0));
    assert_eq!(super::rollout(home.path(), "thread")?, path);
    assert_eq!(super::ROLLOUT_DIRECTORIES.with(std::cell::Cell::get), 1);
    Ok(())
}
