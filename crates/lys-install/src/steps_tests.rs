#![cfg(test)]

use std::collections::BTreeSet;

use super::*;

#[test]
fn every_step_has_its_own_words_and_name() {
    let words: BTreeSet<&str> = Step::ALL.iter().map(|step| step.words()).collect();
    let names: BTreeSet<&str> = Step::ALL.iter().map(|step| step.name()).collect();
    assert_eq!(words.len(), Step::ALL.len());
    assert_eq!(names.len(), Step::ALL.len());
}

#[test]
fn the_name_is_the_serialised_form() -> Result<(), serde_json::Error> {
    let mut compared = 0;
    for step in Step::ALL {
        assert_eq!(serde_json::to_value(step)?, step.name());
        compared += 1;
    }
    assert_eq!(compared, 8);
    Ok(())
}

#[test]
fn the_words_the_brief_names_are_the_words_shown() {
    assert_eq!(Step::Directory.words(), "Preparing your directory");
    assert_eq!(Step::SignIn.words(), "Starting sign-in");
}

/// No step's words name the issuer, a program or a port: a person reads
/// what is happening, never what runs it.
#[test]
fn no_words_name_the_issuer_a_program_or_a_port() {
    let refused = [
        "rauthy",
        "spicedb",
        "postgres",
        "docker",
        "compose",
        "lys-",
        "127.0.0.1",
        "localhost",
    ];
    let mut read = 0;
    for step in Step::ALL {
        let words = step.words().to_lowercase();
        for name in refused {
            assert!(!words.contains(name), "{} names {name}", step.words());
        }
        read += 1;
    }
    assert_eq!(read, Step::ALL.len());
}
