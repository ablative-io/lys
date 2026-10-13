//! Named decoding failures keep useful causes without echoing private JSON values.

use std::fmt::Display;

use crate::commands::error::CliError;
use crate::commands::seat_client::refused;

pub(super) fn source_refused(name: &str, words: &str, error: impl Display) -> CliError {
    refused(name, format!("{words}: {error}"))
}

pub(super) fn json_refused(name: &str, words: &str, error: &serde_json::Error) -> CliError {
    match error.classify() {
        serde_json::error::Category::Data => refused(
            name,
            format!(
                "{words}: JSON data does not match the recorded shape at line {}, column {}",
                error.line(),
                error.column()
            ),
        ),
        _ => source_refused(name, words, error),
    }
}

#[cfg(test)]
mod tests {
    use super::{json_refused, source_refused};
    use base64::Engine;

    #[test]
    fn json_data_errors_are_named_without_echoing_private_values()
    -> Result<(), Box<dyn std::error::Error>> {
        let private =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
        let error = serde_json::from_value::<u64>(serde_json::Value::String(private.clone()))
            .err()
            .ok_or("invalid numeric field accepted")?;
        let words =
            json_refused("person_pass_unreadable", "the pass cannot be read", &error).to_string();
        assert!(words.contains("person_pass_unreadable"));
        assert!(words.contains("JSON data"));
        assert!(words.contains("line"));
        assert!(
            !words.contains(&private),
            "a diagnostic echoed private data"
        );
        Ok(())
    }

    #[test]
    fn source_errors_retain_their_name_and_cause() {
        let error = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let words = source_refused(
            "sign_in_file_unwritable",
            "the file cannot be written",
            error,
        )
        .to_string();
        assert!(words.contains("sign_in_file_unwritable"));
        assert!(words.contains("permission denied"));
    }
}
