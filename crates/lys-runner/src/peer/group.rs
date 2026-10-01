use crate::error::RunnerError;

pub(super) fn members(
    entries: impl Iterator<Item = Result<Option<u32>, RunnerError>>,
    group: u32,
    mut read_group: impl FnMut(u32) -> Result<Option<u32>, RunnerError>,
) -> Result<Vec<u32>, RunnerError> {
    let mut members = Vec::new();
    for entry in entries {
        if let Some(pid) = entry?.filter(|pid| *pid > 1)
            && read_group(pid)? == Some(group)
        {
            members.push(pid);
        }
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::members;
    use crate::error::RunnerError;

    #[test]
    fn rare_group_cleanup_walks_all_processes_on_a_busy_host() {
        let mut reads = 0;
        let entries = (2..5002).map(|pid| Ok(Some(pid)));
        let result = members(entries, 3, |pid| {
            reads += 1;
            Ok(Some(if pid == 5001 { 3 } else { 4 }))
        });
        assert_eq!(reads, 5000);
        assert_eq!(result.ok(), Some(vec![5001]));
    }

    #[test]
    fn a_group_walk_selects_only_its_members_and_propagates_read_failures() {
        let entries = [Ok(Some(2)), Ok(Some(3)), Ok(None), Ok(Some(1))];
        assert_eq!(
            members(entries.into_iter(), 3, |pid| Ok(Some(pid))).ok(),
            Some(vec![3])
        );
        let result = members([Ok(Some(2))].into_iter(), 3, |_| {
            Err(RunnerError::refused("process_group_unreadable", "denied"))
        });
        assert!(result.is_err_and(|error| error.to_string().contains("process_group_unreadable")));
    }
}
