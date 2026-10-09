//! Paging bounds view construction without changing exact filtered totals.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::error::Error;
use std::ops::Bound;

use axum::extract::Query;

use super::{LIMIT_DEFAULT, LIMIT_MAX, ListQuery, Page};
use crate::folded_work::{Work, count, reset};

type TestResult = Result<(), Box<dyn Error>>;

fn page(q: Option<String>, after: Option<String>) -> Result<Page, Box<dyn Error>> {
    Page::read(
        Ok(Query(ListQuery {
            q,
            after,
            limit: Some(5),
            team: None,
        })),
        "/network",
    )?
    .ok_or_else(|| "no page".into())
}

#[test]
fn unfiltered_page_reads_only_limit_plus_one_index_rows() -> TestResult {
    let rows: BTreeMap<_, _> = (0..512)
        .map(|n| (format!("id-{n:04}"), format!("id-{n:04}")))
        .collect();
    let mut query = page(None, None)?;
    for first in [0, 5] {
        let touched = Cell::new(0);
        let built = Cell::new(0);
        let (selected, totals) = query.select(
            rows.range::<str, _>((query.after(), Bound::Unbounded))
                .map(|(_, row)| row)
                .inspect(|_| touched.set(touched.get() + 1)),
            Some(rows.len()),
            |_| Ok(true),
            |row| row.as_str(),
            |row| {
                built.set(built.get() + 1);
                Ok((*row).clone())
            },
        )?;
        assert_eq!(totals.total, 512);
        assert_eq!(selected[0], format!("id-{first:04}"));
        assert_eq!(selected.len(), 5);
        assert_eq!(touched.get(), 6);
        assert_eq!(built.get(), 6);
        query = page(None, totals.next)?;
    }
    Ok(())
}

#[test]
fn filtered_page_counts_exactly_and_builds_only_limit_plus_one_views() -> TestResult {
    let rows: BTreeMap<_, _> = (0..512)
        .map(|n| (format!("id-{n:04}"), format!("id-{n:04}")))
        .collect();
    let query = page(Some("id-0".to_owned()), None)?;
    assert!(query.filtered());
    let built = Cell::new(0);
    let (selected, totals) = query.select(
        rows.values(),
        None,
        |row| Ok(query.matches([row.as_str()])),
        |row| row.as_str(),
        |row| {
            built.set(built.get() + 1);
            Ok((*row).clone())
        },
    )?;
    assert_eq!(totals.total, 512);
    assert_eq!(selected.len(), 5);
    assert_eq!(built.get(), 6);
    let query = page(Some("id-0".to_owned()), totals.next)?;
    let (_, totals) = query.select(
        rows.values(),
        None,
        |row| Ok(query.matches([row.as_str()])),
        |row| row.as_str(),
        |row| Ok((*row).clone()),
    )?;
    assert_eq!(totals.total, 512);
    Ok(())
}

#[test]
fn ten_thousand_character_search_is_accepted_and_its_cursor_reads_every_page() -> TestResult {
    let q = "a".repeat(10_000);
    let rows: BTreeMap<_, _> = (0..12)
        .map(|n| (format!("id-{n:04}"), format!("id-{n:04} {q}")))
        .collect();
    let mut read = Vec::new();
    let mut query = page(Some(q.clone()), None)?;
    loop {
        let (selected, totals) = query.select(
            rows.values(),
            None,
            |row| Ok(query.matches([row.as_str()])),
            |row| row.as_str(),
            |row| Ok((*row).clone()),
        )?;
        assert_eq!(totals.total, 12);
        read.extend(selected);
        let Some(next) = totals.next else { break };
        query = page(Some(q.clone()), Some(next))?;
    }
    assert_eq!(read, rows.values().cloned().collect::<Vec<_>>());
    Ok(())
}

#[test]
fn subtree_visits_each_descendant_once_without_unrelated_teams() -> TestResult {
    let mut teams: Vec<crate::teams_state::Team> = Vec::new();
    for n in (0..64).rev() {
        teams.push(serde_json::from_value(serde_json::json!({
            "created": {"id": format!("team-{n}"), "owner":"person", "name":"team", "description":"", "by":{"provider":"issuer","subject":"person"},"at":1},
            "parent": (n > 0).then(|| format!("team-{}", n - 1)), "members":[], "retired":null, "changes":[]
        }))?);
    }
    reset();
    let held: crate::teams_state::Held =
        serde_json::from_value(serde_json::json!({"teams":teams}))?;
    reset();
    assert_eq!(
        held.subtree("team-0")
            .map_err(|reason| format!("{reason:?}"))?
            .len(),
        64
    );
    assert!(count(Work::Team) <= 64);
    Ok(())
}

#[test]
fn an_always_paged_list_answers_the_default_page_and_caps_the_callers() -> TestResult {
    let rows: Vec<String> = (0..LIMIT_MAX + 7).map(|n| format!("id-{n:04}")).collect();
    for (limit, shown) in [
        (None, LIMIT_DEFAULT),
        (Some(3), 3),
        (Some(LIMIT_MAX + 1), LIMIT_MAX),
    ] {
        let page = Page::always(None, limit, "/product-drafts")?;
        let (selected, totals) = page.select(
            rows.iter(),
            None,
            |_| Ok(true),
            |row| row.as_str(),
            |row| Ok((*row).clone()),
        )?;
        assert_eq!((selected.len(), totals.total), (shown, rows.len()));
        let next = Page::always(totals.next, limit, "/product-drafts")?;
        let (following, _) = next.select(
            rows.iter(),
            None,
            |_| Ok(true),
            |row| row.as_str(),
            |row| Ok((*row).clone()),
        )?;
        assert_eq!(
            following.first(),
            rows.get(shown),
            "the cursor resumes after the page"
        );
    }
    assert!(
        Page::always(None, Some(0), "/product-drafts").is_err(),
        "zero is refused"
    );
    Ok(())
}
