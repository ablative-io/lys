//! Retired top-level keys are refused before generic configuration decoding.

use std::path::Path;

use serde::Deserializer;
use serde::de::{Error, IgnoredAny, MapAccess, Visitor};

use super::{ServerError, invalid};

struct RetiredKeys;

impl<'de> Visitor<'de> for RetiredKeys {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a configuration object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == "LYS_MONITOR_URL" {
                return Err(A::Error::custom("LYS_MONITOR_URL is retired"));
            }
            map.next_value::<IgnoredAny>()?;
        }
        Ok(())
    }
}

pub(super) fn refuse(text: &str, path: &Path) -> Result<(), ServerError> {
    let mut decoder = serde_json::Deserializer::from_str(text);
    decoder.deserialize_map(RetiredKeys).map_err(|error| {
        if error.to_string().starts_with("LYS_MONITOR_URL is retired") {
            invalid(format!(
                "{}: LYS_MONITOR_URL is retired; delete line {} setting LYS_MONITOR_URL",
                path.display(),
                error.line(),
            ))
        } else {
            invalid(format!(
                "{} is not a configuration: {error}",
                path.display()
            ))
        }
    })
}
