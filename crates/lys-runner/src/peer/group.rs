use crate::error::RunnerError;

const RECORD_MAX: usize = 4096;

pub(super) fn members(
    entries: impl Iterator<Item = Result<Option<u32>, RunnerError>>,
    group: u32,
    mut read_group: impl FnMut(u32) -> Result<Option<u32>, RunnerError>,
) -> Result<Vec<u32>, RunnerError> {
    let mut members = Vec::new();
    for (visited, entry) in entries.enumerate() {
        if visited == RECORD_MAX {
            return Err(RunnerError::refused(
                "process_group_scan_full",
                format!("group cleanup exceeds {RECORD_MAX} process records"),
            ));
        }
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
    use super::{RECORD_MAX, members};
    use crate::error::RunnerError;

    #[test]
    fn an_excessive_group_walk_is_refused_without_unbounded_process_reads() {
        let mut reads = 0;
        let entries = std::iter::repeat_with(|| Ok(Some(2)));
        let result = members(entries, 3, |_| {
            reads += 1;
            Ok(Some(3))
        });
        assert_eq!(reads, RECORD_MAX);
        assert!(result.is_err_and(|error| error.to_string().contains("process_group_scan_full")));
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
