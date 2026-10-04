#![cfg(test)]
//! `lys runner join` takes the server and the computer on its command line
//! and the connection code on standard input alone: the code is no argument
//! at all, so it never reaches shell history or the process list.

use clap::Parser;

use super::*;
use crate::cli::{Cli, Command, RunnerCommand};

#[test]
fn join_takes_the_server_and_the_computer_and_no_code() {
    let cli = Cli::try_parse_from([
        "lys",
        "runner",
        "join",
        "--server",
        "https://lys.example.test/api",
        "--machine",
        "op-0123456789abcdef0123456789abcdef",
    ])
    .expect("the join parses");
    let (server, machine, server_ca, scrollback) = match cli.command {
        Command::Runner(RunnerCommand::Join {
            server,
            machine,
            server_ca,
            scrollback,
        }) => (server, machine, server_ca, scrollback),
        other => panic!("not a join: {other:?}"),
    };
    assert_eq!(server, "https://lys.example.test/api");
    assert_eq!(machine, "op-0123456789abcdef0123456789abcdef");
    assert_eq!(server_ca, None);
    // One mebibyte, the scrollback `lys runner serve` keeps when none is given.
    assert_eq!(scrollback, 1 << 20);
}

#[test]
fn join_refuses_a_code_on_the_command_line() {
    for code in [["--code", "abc"], ["abc", "--server"]] {
        let mut args = vec![
            "lys",
            "runner",
            "join",
            "--server",
            "https://lys.example.test",
            "--machine",
            "m",
        ];
        args.extend(code);
        assert!(
            Cli::try_parse_from(&args).is_err(),
            "{args:?} must not parse"
        );
    }
}

#[test]
fn join_names_the_server_and_the_computer_or_does_not_parse() {
    for args in [
        vec!["lys", "runner", "join", "--machine", "m"],
        vec![
            "lys",
            "runner",
            "join",
            "--server",
            "https://lys.example.test",
        ],
    ] {
        assert!(
            Cli::try_parse_from(&args).is_err(),
            "{args:?} must not parse"
        );
    }
}

#[test]
fn the_code_is_one_line_of_standard_input_trimmed() {
    let mut input = std::io::Cursor::new("  0f1e2d3c\n and more\n");
    let code = read_code(&mut input, false).expect("a code is read");
    assert_eq!(code.as_str(), "0f1e2d3c");
}

#[test]
fn no_code_on_standard_input_is_refused_by_name() {
    for given in ["", "\n", "   \n"] {
        let mut input = std::io::Cursor::new(given);
        let refused = read_code(&mut input, false).expect_err("no code is refused");
        let CliError::Runner(error) = refused else {
            panic!("not the runner's refusal: {refused}");
        };
        assert_eq!(error.name(), CODE_MISSING);
    }
}

#[test]
fn an_address_another_computer_cannot_reach_is_refused_before_anything_is_sent() {
    for server in [
        "http://lys.example.test",
        "lys.example.test",
        "https://localhost:8490",
        "https://127.0.0.1:8490/api",
        "https://[::1]:8490",
    ] {
        let refused = reachable(server).expect_err(server);
        assert_eq!(
            refused.name(),
            lys_runner::dial::join::UNREACHABLE,
            "{server}"
        );
    }
    for server in [
        "https://lys.example.test",
        "https://lys.example.test:8490/api",
        "https://192.0.2.7/api",
    ] {
        reachable(server).expect(server);
    }
}
