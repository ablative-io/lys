//! A product receipt prevents a second execution after acknowledgment loss.

use lys_pass::drafts::{ApprovedDraft, Execution, Executor, execute_once};
use lys_pass::Target;

#[derive(Default)]
struct Product {
    receipt: Option<Execution>,
    writes: usize,
}

impl Executor for Product {
    type Error = std::io::Error;

    fn receipt(&self, draft: &str) -> Result<Option<Execution>, Self::Error> {
        assert_eq!(draft, "draft");
        Ok(self.receipt.clone())
    }

    fn execute_and_record(&mut self, draft: &ApprovedDraft) -> Result<Execution, Self::Error> {
        self.writes += 1;
        let receipt = Execution::Executed { request_digest: draft.request_digest.clone(), receipt_digest: lys_pass::drafts::request_digest("receipt") };
        self.receipt = Some(receipt.clone());
        Ok(receipt)
    }
}

#[test]
fn retry_after_acknowledgment_loss_does_not_execute_again() -> Result<(), Box<dyn std::error::Error>> {
    let draft = ApprovedDraft { id: "draft".to_owned(), app: "sample".to_owned(), grant: "grant".to_owned(), target: Target::new("sample.file", "held", "write")?, request_digest: lys_pass::drafts::request_digest("prepared"), words: "prepared".to_owned() };
    let mut product = Product::default();
    let first = execute_once(&draft, &mut product)?;
    assert_eq!(execute_once(&draft, &mut product)?, first);
    assert_eq!(product.writes, 1);
    let mut changed = draft;
    changed.request_digest = "1".repeat(64);
    assert!(execute_once(&changed, &mut product).is_err());
    assert_eq!(product.writes, 1);
    Ok(())
}
