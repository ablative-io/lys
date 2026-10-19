//! The name the computer this service runs on gives itself, read once.
//!
//! It is offered as the name of this computer when a person adds it, so a
//! first computer never starts with an empty name. On macOS the Computer Name
//! a person set in System Settings is preferred (such as `Tom’s MacBook Pro`); else
//! the host name, without a trailing `.local`.

use std::process::Command;
use std::sync::OnceLock;

static NAME: OnceLock<Option<String>> = OnceLock::new();

/// This computer's own name, or `None` when the system names none.
pub(crate) fn read() -> Option<String> {
    NAME.get_or_init(|| {
        let computer = if cfg!(target_os = "macos") {
            output("scutil", &["--get", "ComputerName"])
        } else {
            None
        };
        computer.or_else(|| output("hostname", &[]).map(|host| trimmed_host(&host)))
    })
    .clone()
}

fn output(program: &str, arguments: &[&str]) -> Option<String> {
    let answer = Command::new(program).args(arguments).output().ok()?;
    if !answer.status.success() {
        return None;
    }
    cleaned(&String::from_utf8_lossy(&answer.stdout))
}

/// The first line of `text`, trimmed, when it is a name a computer may carry.
fn cleaned(text: &str) -> Option<String> {
    let line = text.lines().next()?.trim();
    (!line.is_empty() && !line.chars().any(char::is_control)).then(|| line.to_owned())
}

fn trimmed_host(host: &str) -> String {
    host.strip_suffix(".local").unwrap_or(host).to_owned()
}

#[cfg(test)]
mod tests {
    use super::{cleaned, read, trimmed_host};

    #[test]
    fn a_name_is_its_first_trimmed_line_and_never_empty_or_controlled() {
        assert_eq!(
            cleaned("Tom’s MacBook Pro\n"),
            Some("Tom’s MacBook Pro".to_owned())
        );
        assert_eq!(cleaned("  studio \nsecond\n"), Some("studio".to_owned()));
        assert_eq!(cleaned("\n"), None);
        assert_eq!(cleaned(""), None);
        assert_eq!(cleaned("bad\u{7}name"), None);
    }

    #[test]
    fn a_bonjour_host_name_loses_its_local_suffix() {
        assert_eq!(trimmed_host("Toms-MacBook-Pro.local"), "Toms-MacBook-Pro");
        assert_eq!(trimmed_host("builder"), "builder");
    }

    #[test]
    fn this_computer_reads_a_name_and_reads_it_once() {
        let first = read();
        assert!(first.as_deref().is_none_or(|name| !name.is_empty()));
        assert_eq!(read(), first);
    }
}
