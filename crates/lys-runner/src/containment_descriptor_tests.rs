//! Descriptor cleanup checks use real owned files and preserve test-process handles.

use std::error::Error;
use std::os::fd::AsRawFd;

use nix::fcntl::{FcntlArg, FdFlag, fcntl};

use super::{mark, seal_for_exec};

#[test]
fn inherited_file_is_marked_without_closing_its_owned_handle() -> Result<(), Box<dyn Error>> {
    let file = tempfile::tempfile()?;
    let fd = file.as_raw_fd();
    fcntl(fd, FcntlArg::F_SETFD(FdFlag::empty()))?;
    mark(fd)?;
    let flags = FdFlag::from_bits_retain(fcntl(fd, FcntlArg::F_GETFD)?);
    assert!(flags.contains(FdFlag::FD_CLOEXEC));
    assert!(file.metadata()?.is_file());
    Ok(())
}

#[test]
fn missing_descriptor_names_the_refused_descriptor() -> Result<(), Box<dyn Error>> {
    let error = mark(-1).err().ok_or("invalid descriptor accepted")?;
    assert_eq!(error.name(), "containment_descriptors_unavailable");
    assert!(error.to_string().contains("descriptor -1"));
    Ok(())
}

#[test]
fn multithreaded_runner_is_refused_before_descriptors_are_changed() -> Result<(), Box<dyn Error>> {
    let file = tempfile::tempfile()?;
    let fd = file.as_raw_fd();
    fcntl(fd, FcntlArg::F_SETFD(FdFlag::empty()))?;
    let error = seal_for_exec()
        .err()
        .ok_or("multithreaded runner accepted")?;
    let actual = FdFlag::from_bits_retain(fcntl(fd, FcntlArg::F_GETFD)?);
    // Restore even if an assertion below discovers a regression.
    mark(fd)?;
    assert_eq!(error.name(), "containment_descriptors_unavailable");
    assert!(error.to_string().contains("threads"));
    assert!(!actual.contains(FdFlag::FD_CLOEXEC));
    Ok(())
}
