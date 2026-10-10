//! The identity server's one tracing subscriber (DIRECTORY-095): every
//! event at WARN or ERROR is said as one line through the `say` sink every
//! log line goes through, which the install keeps as identity.log, with its
//! level, its target, its message and every other field as `name=value`.
//! Events below WARN are said nowhere and cost no formatting. Spans are
//! accepted and ignored. Without it the server's warnings reached no one.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

use crate::routes::Say;

/// Says each WARN and ERROR event through `say`.
pub struct WarnLines {
    say: Say,
    spans: AtomicU64,
}

impl WarnLines {
    /// A subscriber saying through `say`.
    #[must_use]
    pub fn new(say: Say) -> Self {
        Self {
            say,
            spans: AtomicU64::new(0),
        }
    }
}

/// An event's message and its other fields, in the order it gives them.
#[derive(Default)]
struct Fields {
    message: String,
    rest: String,
}

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            self.rest.push(' ');
            self.rest.push_str(field.name());
            self.rest.push('=');
            self.rest.push_str(value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            let shown = format!("{value:?}");
            self.message.push_str(&shown);
        } else {
            let shown = format!("{value:?}");
            self.rest.push(' ');
            self.rest.push_str(field.name());
            self.rest.push('=');
            self.rest.push_str(&shown);
        }
    }
}

/// The line an event is said as.
fn line(event: &Event<'_>) -> String {
    let mut fields = Fields::default();
    event.record(&mut fields);
    let metadata = event.metadata();
    format!(
        "{} {}: {}{}",
        metadata.level(),
        metadata.target(),
        fields.message,
        fields.rest
    )
}

impl Subscriber for WarnLines {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= Level::WARN
    }

    // Spans are accepted and ignored: each gets an id, and nothing of it is said.
    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        debug_assert!(attributes.metadata().is_span());
        Id::from_u64(self.spans.fetch_add(1, Ordering::Relaxed) + 1)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        debug_assert_ne!(span.into_u64(), 0);
        // A span's values are visited and dropped; spans are never said.
        let mut dropped = Fields::default();
        values.record(&mut dropped);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        debug_assert!(span.into_u64() != 0 && follows.into_u64() != 0);
    }

    fn event(&self, event: &Event<'_>) {
        (self.say)(&line(event));
    }

    fn enter(&self, span: &Id) {
        debug_assert_ne!(span.into_u64(), 0);
    }

    fn exit(&self, span: &Id) {
        debug_assert_ne!(span.into_u64(), 0);
    }
}

/// A second subscriber refused: one is already the global default.
#[derive(Debug)]
pub struct SubscriberInstalled(String);

impl fmt::Display for SubscriberInstalled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "log_subscriber_installed: a tracing subscriber is already the global default: {}",
            self.0
        )
    }
}

impl std::error::Error for SubscriberInstalled {}

/// Install [`WarnLines`] on `say` as the global default, as the server's
/// start does before the service is built.
///
/// # Errors
/// Refuses `log_subscriber_installed` when a global default is already set.
pub fn install(say: Say) -> Result<(), SubscriberInstalled> {
    tracing::subscriber::set_global_default(WarnLines::new(say))
        .map_err(|refused| SubscriberInstalled(refused.to_string()))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::WarnLines;
    use crate::routes::Say;

    fn capture() -> (Say, Arc<Mutex<Vec<String>>>) {
        let heard: Arc<Mutex<Vec<String>>> = Arc::default();
        let lines = Arc::clone(&heard);
        let say: Say = Arc::new(move |line: &str| match lines.lock() {
            Ok(mut lines) => lines.push(line.to_owned()),
            Err(poisoned) => panic!("warn_lines_fixture_poisoned: {poisoned}"),
        });
        (say, heard)
    }

    #[test]
    fn a_warn_event_is_said_with_its_target_and_fields_and_info_is_not()
    -> Result<(), Box<dyn std::error::Error>> {
        let (say, heard) = capture();
        tracing::subscriber::with_default(WarnLines::new(say), || {
            tracing::warn!(
                target: "lys_identity_server::apps_client_credentials",
                app = "studio",
                "a credential could not be ended at the broker"
            );
            tracing::info!(app = "studio", "said nowhere");
            tracing::debug!(app = "studio", "said nowhere");
        });
        let lines = heard.lock().map_err(|poisoned| poisoned.to_string())?;
        assert_eq!(
            *lines,
            vec![
                "WARN lys_identity_server::apps_client_credentials: a credential could not be ended at the broker app=studio"
                    .to_owned()
            ]
        );
        Ok(())
    }
}
