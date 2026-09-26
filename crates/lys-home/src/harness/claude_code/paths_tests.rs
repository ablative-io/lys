//! Gates on the project slug: the rule measured on Claude Code 2.1.283
//! against dotted, underscored and hyphenated working directories.

use crate::harness::claude_code::projects_slug;

#[test]
fn a_dotted_ancestor_becomes_two_dashes() {
    assert_eq!(
        projects_slug("/home/u/.aion/clones/w"),
        "-home-u--aion-clones-w"
    );
}

#[test]
fn underscore_dot_and_dash_each_give_one_dash_and_none_are_collapsed() {
    assert_eq!(projects_slug("/tmp/x_y/p-q.r/w"), "-tmp-x-y-p-q-r-w");
}

#[test]
fn a_plain_directory_is_unchanged_from_the_earlier_rule() {
    assert_eq!(projects_slug("/srv/plain"), "-srv-plain");
}

#[test]
fn letters_and_digits_stay_and_a_space_becomes_a_dash() {
    assert_eq!(projects_slug("/Users/t/Dev 2/a1"), "-Users-t-Dev-2-a1");
}

#[test]
fn an_empty_directory_gives_an_empty_slug() {
    assert_eq!(projects_slug(""), "");
}
