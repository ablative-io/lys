//! The status a browser gets for each refusal the folder routes pass on from
//! the computer's runner: a cancel or an absent dialog is a conflict the person
//! can act on, and a dialog answer that is not an absolute path is a bad request.

use axum::http::StatusCode;

use crate::error::ServerError;

fn status(refusal: &str) -> StatusCode {
    ServerError::Runner {
        refusal: refusal.to_owned(),
        words: format!("the runner refused {refusal}"),
    }
    .status()
}

#[test]
fn a_cancelled_folder_dialog_is_a_conflict_the_person_resolves() {
    assert_eq!(status("folder_choice_cancelled"), StatusCode::CONFLICT);
}

#[test]
fn a_computer_that_cannot_show_its_folder_dialog_is_a_conflict() {
    assert_eq!(status("folder_dialog_unavailable"), StatusCode::CONFLICT);
}

#[test]
fn a_folder_answer_that_is_not_an_absolute_path_is_a_bad_request() {
    assert_eq!(status("folder_invalid"), StatusCode::BAD_REQUEST);
}

#[test]
fn a_runner_with_no_home_folder_is_a_conflict() {
    assert_eq!(status("home_unknown"), StatusCode::CONFLICT);
}
