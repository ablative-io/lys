use std::error::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};

use axum::http::StatusCode;
use serde_json::json;

use super::{Bench, Benches, By};

#[test]
fn an_interrupted_bench_change_refuses_the_partial_draft() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let benches = Benches::new(temporary.path().join("benches"), &|_| {})?;
    let interrupted = catch_unwind(AssertUnwindSafe(|| {
        if let Ok(mut open) = benches.open() {
            open.insert(
                "draft".to_owned(),
                Bench {
                    app: "example".to_owned(),
                    schema: json!({"incomplete": true}),
                    opened_by: By::Start,
                    dir: temporary.path().join("draft"),
                },
            );
            panic!("draft change interrupted");
        }
    }));
    assert!(interrupted.is_err());
    let refusal = benches
        .open()
        .err()
        .ok_or("partial draft became readable")?;
    assert_eq!(refusal.name(), "apps_unavailable");
    assert_eq!(refusal.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(benches.open().is_err());
    Ok(())
}
