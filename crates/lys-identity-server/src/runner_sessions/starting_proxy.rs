//! A proxy argument may differ only by the model run key already admitted for the launch.
use super::{ServerError, record_unproved};
pub(super) fn arguments(
    record: &super::super::LaunchRecord,
    launch: &lys_runner::Launch,
    proxy: &str,
) -> Result<(), ServerError> {
    let run = launch
        .environment
        .get("LYS_RUN")
        .filter(|run| lys_home::proxy::usage::is_run_key(run))
        .ok_or_else(|| record_unproved("the rendered Codex model proxy has no valid run key"))?;
    if launch.arguments.len() != record.arguments.len() {
        return Err(record_unproved(
            "the signed argument count differs from its reviewed profile",
        ));
    }
    for (given, wanted) in record.arguments.iter().zip(&launch.arguments) {
        if given != wanted && !super::super::proxy_argument(given, wanted, proxy, run)? {
            return Err(record_unproved(
                "a signed Codex argument differs beyond its per-run proxy key",
            ));
        }
    }
    Ok(())
}
