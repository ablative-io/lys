//! A query narrows an already admitted list; its cursor never grants access.
//! Legacy reads retain their order and shape. Queried rows use identifier order,
//! and cursors belong to one route and its search and team filters.

use std::collections::BTreeSet;
use std::ops::Bound;

#[cfg(test)]
#[path = "list_page_tests.rs"]
mod tests;

use axum::extract::Query;
use axum::extract::rejection::QueryRejection;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::routes::AppState;
use crate::teams_api::with_teams;

/// Optional search and paging inputs shared by the list routes.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    /// Case-insensitive text contained in a row's searchable names or words.
    pub q: Option<String>,
    /// A team and every descendant in its current nesting tree.
    pub team: Option<String>,
    /// The opaque cursor returned by the preceding page with the same filters.
    pub after: Option<String>,
    /// Page size, defaulting to 50 and capped at 200; zero is refused.
    pub limit: Option<usize>,
}

/// Counters and continuation present only on a queried answer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Totals {
    /// Matching admitted rows before paging.
    pub total: usize,
    /// The next page's opaque cursor, or null when this is the last page.
    pub next: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    route: String,
    q: Option<String>,
    team: Option<String>,
    last: String,
}

pub(crate) type Input = Result<Query<ListQuery>, QueryRejection>;

pub(crate) struct Page {
    route: &'static str,
    query: ListQuery,
    last: Option<String>,
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

impl Page {
    pub(crate) fn read(input: Input, route: &'static str) -> Result<Option<Self>, ServerError> {
        let Query(mut query) = input.map_err(|refused| malformed(refused.body_text()))?;
        if query.q.is_none()
            && query.team.is_none()
            && query.after.is_none()
            && query.limit.is_none()
        {
            return Ok(None);
        }
        if query.limit == Some(0) {
            return Err(malformed("limit must be greater than zero"));
        }
        query.q = query.q.map(|words| words.to_lowercase());
        if query
            .q
            .as_ref()
            .is_some_and(|words| words.chars().count() > 500)
        {
            return Err(malformed("q is longer than 500 characters"));
        }
        let last = query
            .after
            .as_deref()
            .map(|encoded| {
                if encoded.len() > 16_384 {
                    return Err(malformed("after is longer than a paging cursor"));
                }
                let bytes = URL_SAFE_NO_PAD
                    .decode(encoded)
                    .map_err(|error| malformed(format!("after is not a paging cursor: {error}")))?;
                let cursor: Cursor = serde_json::from_slice(&bytes)
                    .map_err(|error| malformed(format!("after is not a paging cursor: {error}")))?;
                if cursor.version != 1
                    || cursor.route != route
                    || cursor.q != query.q
                    || cursor.team != query.team
                    || cursor.last.is_empty()
                {
                    return Err(malformed(
                        "after does not belong to this list and its filters",
                    ));
                }
                Ok(cursor.last)
            })
            .transpose()?;
        Ok(Some(Self { route, query, last }))
    }

    pub(crate) fn matches<'a>(&self, words: impl IntoIterator<Item = &'a str>) -> bool {
        self.query.q.as_ref().is_none_or(|query| {
            words
                .into_iter()
                .any(|word| word.to_lowercase().contains(query))
        })
    }

    pub(crate) fn members(
        &self,
        state: &AppState,
    ) -> Result<Option<BTreeSet<String>>, ServerError> {
        let Some(root) = &self.query.team else {
            return Ok(None);
        };
        with_teams(state, |store| {
            let subtree = store.subtree(root)?;
            Ok(Some(
                subtree
                    .iter()
                    .filter_map(|id| store.team(id))
                    .flat_map(|team| team.members.iter().cloned())
                    .collect(),
            ))
        })
    }

    pub(crate) fn teams(&self, state: &AppState) -> Result<Option<BTreeSet<String>>, ServerError> {
        self.query
            .team
            .as_deref()
            .map(|root| with_teams(state, |store| store.subtree(root)))
            .transpose()
    }

    pub(crate) fn after(&self) -> Bound<&str> {
        self.last
            .as_deref()
            .map_or(Bound::Unbounded, Bound::Excluded)
    }

    pub(crate) fn filtered(&self) -> bool {
        self.query.q.is_some() || self.query.team.is_some()
    }

    pub(crate) fn select<T, V>(
        &self,
        rows: impl Iterator<Item = T>,
        total: Option<usize>,
        mut matches: impl FnMut(&T) -> Result<bool, ServerError>,
        id: impl Fn(&T) -> &str,
        view: impl Fn(&T) -> Result<V, ServerError>,
    ) -> Result<(Vec<V>, Totals), ServerError> {
        let limit = self.query.limit.unwrap_or(50).min(200);
        let mut selected = Vec::with_capacity(limit + 1);
        let mut counted = 0;
        for row in rows {
            if !matches(&row)? {
                continue;
            }
            counted += 1;
            if self.last.as_deref().is_none_or(|last| id(&row) > last) && selected.len() <= limit {
                selected.push((id(&row).to_owned(), view(&row)?));
                if total.is_some() && selected.len() > limit {
                    break;
                }
            }
        }
        let more = selected.len() > limit;
        selected.truncate(limit);
        let next = if more {
            let last = selected
                .last()
                .ok_or_else(|| malformed("the page has no final row"))?;
            Some(self.cursor(&last.0)?)
        } else {
            None
        };
        Ok((
            selected.into_iter().map(|(_, view)| view).collect(),
            Totals {
                total: total.unwrap_or(counted),
                next,
            },
        ))
    }

    fn cursor(&self, last: &str) -> Result<String, ServerError> {
        let cursor = Cursor {
            version: 1,
            route: self.route.to_owned(),
            q: self.query.q.clone(),
            team: self.query.team.clone(),
            last: last.to_owned(),
        };
        Ok(
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).map_err(|error| {
                malformed(format!("the paging cursor could not be encoded: {error}"))
            })?),
        )
    }

    pub(crate) fn finish<T>(
        &self,
        rows: &mut Vec<T>,
        id: impl Fn(&T) -> &str,
    ) -> Result<Totals, ServerError> {
        rows.sort_by(|left, right| id(left).cmp(id(right)));
        let total = rows.len();
        if let Some(last) = &self.last {
            rows.retain(|row| id(row) > last.as_str());
        }
        let limit = self.query.limit.unwrap_or(50).min(200);
        let more = rows.len() > limit;
        rows.truncate(limit);
        let next = if more {
            let row = rows
                .last()
                .ok_or_else(|| malformed("the page has no final row"))?;
            let cursor = Cursor {
                version: 1,
                route: self.route.to_owned(),
                q: self.query.q.clone(),
                team: self.query.team.clone(),
                last: id(row).to_owned(),
            };
            Some(
                URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).map_err(|error| {
                    malformed(format!("the paging cursor could not be encoded: {error}"))
                })?),
            )
        } else {
            None
        };
        Ok(Totals { total, next })
    }
}

pub(crate) fn member(
    members: Option<&BTreeSet<String>>,
    identity: &str,
    person: Option<&str>,
) -> bool {
    members.is_none_or(|members| {
        members.contains(identity) || person.is_some_and(|person| members.contains(person))
    })
}

pub(crate) fn document_query(document: &mut serde_json::Value) -> Result<(), ServerError> {
    let invalid = || ServerError::ConfigInvalid {
        reason: "the list query schema or operation is missing from OpenAPI".to_owned(),
    };
    let properties = document
        .pointer("/components/schemas/ListQuery/properties")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(invalid)?;
    let parameters: Vec<serde_json::Value> = properties.iter().map(|(name, schema)| {
        serde_json::json!({ "name":name, "in":"query", "required":false, "schema":schema })
    }).collect();
    for path in [
        "/people",
        "/directory/people",
        "/network",
        "/runtime/live",
        "/requests",
    ] {
        let operation = document
            .get_mut("paths")
            .and_then(|paths| paths.get_mut(path))
            .and_then(|path| path.get_mut("get"))
            .and_then(serde_json::Value::as_object_mut)
            .ok_or_else(invalid)?;
        operation.remove("requestBody");
        operation.insert(
            "parameters".to_owned(),
            serde_json::Value::Array(parameters.clone()),
        );
    }
    Ok(())
}
