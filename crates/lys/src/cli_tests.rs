#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clap::CommandFactory;

use super::Cli;
use crate::commands::ca::capability_claims_oid;

/// An OID in the dotted-decimal form help text quotes it in.
fn dotted(oid: &[u64]) -> String {
    let components: Vec<String> = oid.iter().map(u64::to_string).collect();
    components.join(".")
}

/// The OID in the `ca issue` help is a hand-written literal; this pins it
/// to the OID the CLI actually issues under, so changing the arc without
/// changing the help text fails here rather than misleading an operator.
#[test]
fn ca_issue_help_quotes_the_capability_claims_oid() {
    let mut command = Cli::command();
    let issue = command
        .find_subcommand_mut("ca")
        .expect("`ca` subcommand is declared")
        .find_subcommand_mut("issue")
        .expect("`ca issue` subcommand is declared");
    let help = issue.render_long_help().to_string();
    let expected = dotted(&capability_claims_oid());

    assert!(
        help.contains(&expected),
        "`lys ca issue --help` must name OID {expected}; rendered help was:\n{help}"
    );
}
