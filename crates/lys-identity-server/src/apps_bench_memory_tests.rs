use super::{Draft, Examples, answer_in};
use crate::apps_bench::{Example, Holding, Question};
use crate::apps_state::By;
use lys_identity::grants::{Action, Model, Relation};
use serde_json::json;
use std::error::Error;

#[test]
fn scratch_questions_preserve_decisions_without_disk_syncs() -> Result<(), Box<dyn Error>> {
    let model = Model::new(
        1,
        [(Relation::new("viewer")?, [Action::new("read")?].into())],
    )?;
    let schema =
        json!({"kinds":{"notes.doc":{"actions":["read","write"],"relations":{"viewer":["read"]}}}});
    let draft = Draft {
        app: "notes",
        schema: &schema,
        by: &By::Start,
        lys: &model,
        engine: None,
    };
    let resource = Example {
        kind: "notes.doc".to_owned(),
        id: "one".to_owned(),
    };
    let holdings = [Holding {
        subject: "reader".to_owned(),
        relation: "viewer".to_owned(),
        resource: resource.clone(),
    }];
    let mut question = Question {
        subject: "reader".to_owned(),
        action: "read".to_owned(),
        resource,
    };
    let before = lys_log_store::flush_count();
    let answer = answer_in(
        &draft,
        &Examples {
            holdings: &holdings,
            placements: &[],
            question: &question,
        },
    )?;
    assert!(answer.allowed);
    assert_eq!(
        answer.path,
        [
            "reader holds viewer on notes.doc:one",
            "viewer carries read"
        ]
    );
    question.action = "write".to_owned();
    let refused = answer_in(
        &draft,
        &Examples {
            holdings: &holdings,
            placements: &[],
            question: &question,
        },
    )?;
    assert!(!refused.allowed);
    assert_eq!(refused.refusal.as_deref(), Some("NotHeld"));
    assert_eq!(
        lys_log_store::flush_count() - before,
        0,
        "throwaway questions synced disk"
    );
    Ok(())
}
