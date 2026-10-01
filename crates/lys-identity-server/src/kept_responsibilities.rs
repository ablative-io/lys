//! Deployment responsibilities are validated once before serving requests.

use std::path::Path;

use serde::Deserialize;

use crate::error::ServerError;

pub(crate) const DEFAULT: &str = include_str!("kept-responsibilities.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Responsibility {
    method: String,
    path: String,
}

pub(crate) struct Kept {
    routes: Vec<Responsibility>,
}

impl Kept {
    pub(crate) fn load(file: &Path) -> Result<Self, ServerError> {
        let data = match std::fs::read(file) {
            Ok(data) => data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                DEFAULT.as_bytes().to_vec()
            }
            Err(error) => {
                return Err(invalid(format!(
                    "could not read kept responsibilities: {error}"
                )));
            }
        };
        let mut routes: Vec<Responsibility> = serde_json::from_slice(&data)
            .map_err(|error| invalid(format!("invalid kept responsibilities: {error}")))?;
        let api = crate::openapi::api();
        let mut seen = std::collections::BTreeSet::new();
        for route in &mut routes {
            route.method.make_ascii_uppercase();
            if !api.routes().iter().any(|declared| {
                declared.method.word().eq_ignore_ascii_case(&route.method)
                    && declared.path == route.path
            }) {
                return Err(invalid(format!(
                    "unknown kept route: {} {}",
                    route.method, route.path
                )));
            }
            if !seen.insert((&route.method, &route.path)) {
                return Err(invalid(format!(
                    "duplicate kept route: {} {}",
                    route.method, route.path
                )));
            }
        }
        Ok(Self { routes })
    }

    pub(crate) fn keeps(&self, method: &str, path: &str) -> bool {
        self.routes.iter().any(|route| {
            route.method.eq_ignore_ascii_case(method) && matches_path(&route.path, path)
        })
    }
}

fn matches_path(pattern: &str, path: &str) -> bool {
    let mut actual = path.split('/');
    pattern.split('/').all(|part| {
        actual.next().is_some_and(|value| {
            if part.starts_with('{') && part.ends_with('}') {
                !value.is_empty()
            } else {
                part == value
            }
        })
    }) && actual.next().is_none()
}

fn invalid(reason: String) -> ServerError {
    ServerError::ConfigInvalid { reason }
}

#[cfg(test)]
#[path = "kept_responsibilities_tests.rs"]
mod tests;
