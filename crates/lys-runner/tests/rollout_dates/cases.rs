#![cfg(test)]
//! Current rollout discovery does not walk older session directories.

use std::error::Error;

#[test]
fn a_current_rollout_touches_only_its_date_directory() -> Result<(), Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    let now = crate::session::now_ms();
    let date = jiff::Timestamp::from_millisecond(i64::try_from(now)?)?
        .to_zoned(jiff::tz::TimeZone::try_system()?)
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
    assert_eq!(super::rollout_since(home.path(), "thread", now, now)?, path);
    assert_eq!(super::ROLLOUT_DIRECTORIES.with(std::cell::Cell::get), 1);
    Ok(())
}

#[test]
fn fallback_visits_only_dates_since_launch_and_stops_at_the_match() -> Result<(), Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    let zone = jiff::tz::TimeZone::try_system()?;
    let start = jiff::civil::date(2026, 9, 29);
    let end = jiff::civil::date(2026, 10, 1);
    let stamp = |date: jiff::civil::Date| -> Result<u64, Box<dyn Error>> {
        Ok(u64::try_from(
            date.to_zoned(zone.clone())?.timestamp().as_millisecond(),
        )?)
    };
    let current = home.path().join("sessions/2026/09/30");
    std::fs::create_dir_all(&current)?;
    std::fs::create_dir_all(home.path().join("sessions/1980/01/01"))?;
    let path = current.join("rollout-prior-thread.jsonl");
    std::fs::write(
        &path,
        b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"thread\"}}\n",
    )?;
    super::ROLLOUT_DIRECTORIES.with(|count| count.set(0));
    assert_eq!(
        super::rollout_since(home.path(), "thread", stamp(start)?, stamp(end)?)?,
        path
    );
    assert_eq!(super::ROLLOUT_DIRECTORIES.with(std::cell::Cell::get), 2);
    Ok(())
}

#[test]
fn malformed_dates_and_thread_ids_are_refused_at_the_boundary() -> Result<(), Box<dyn Error>> {
    let home = tempfile::tempdir()?;
    assert_eq!(
        super::rollout_since(home.path(), "thread", 2, 1)
            .err()
            .ok_or("reversed date accepted")?
            .name(),
        "rollout_date_invalid"
    );
    assert_eq!(
        super::rollout_since(home.path(), "../thread", 1, 2)
            .err()
            .ok_or("invalid thread accepted")?
            .name(),
        "notify_malformed"
    );
    Ok(())
}
