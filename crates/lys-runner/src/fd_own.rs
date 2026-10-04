//! Descriptors known only by number, made the runner's own without unsafe
//! code.
//!
//! The terminal library keeps a terminal's runner end behind a number, and
//! a program that replaced its own image finds what it kept by number. A
//! number is turned into an owned descriptor by sending it over a pair of
//! connected sockets to this same process: the kernel checks the number
//! and gives the receiving end a new descriptor for the same open file, so
//! a number that names nothing is refused by the kernel, never trusted.

use std::io::{IoSlice, IoSliceMut};
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;

use rustix::io::FdFlags;
use rustix::net::{
    RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, SendAncillaryBuffer,
    SendAncillaryMessage, SendFlags,
};

use crate::error::RunnerError;

fn failed(what: &str, error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("descriptor_unowned", format!("{what}: {error}"))
}

/// A new descriptor of this process for the open file `raw` names.
pub(crate) fn own(raw: RawFd) -> Result<OwnedFd, RunnerError> {
    let (sender, receiver) =
        UnixStream::pair().map_err(|error| failed("a socket pair could not be made", error))?;
    let numbers = [raw];
    let marker = [0_u8];
    nix::sys::socket::sendmsg::<nix::sys::socket::UnixAddr>(
        sender.as_raw_fd(),
        &[IoSlice::new(&marker)],
        &[nix::sys::socket::ControlMessage::ScmRights(&numbers)],
        nix::sys::socket::MsgFlags::empty(),
        None,
    )
    .map_err(|error| failed(&format!("descriptor {raw} could not be sent"), error))?;
    receive(&receiver)
        .map(|(_, owned)| owned)
        .map_err(|error| failed(&format!("descriptor {raw}"), error))
}

/// Send `fd` over the connected `socket`, with the one byte `marker`.
pub fn send(socket: &UnixStream, fd: BorrowedFd<'_>, marker: u8) -> Result<(), RunnerError> {
    let fds = [fd];
    let mut space = vec![MaybeUninit::<u8>::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut control = SendAncillaryBuffer::new(&mut space);
    if !control.push(SendAncillaryMessage::ScmRights(&fds)) {
        return Err(failed("a descriptor could not be sent", "it does not fit its message"));
    }
    let byte = [marker];
    rustix::net::sendmsg(socket, &[IoSlice::new(&byte)], &mut control, SendFlags::empty())
        .map_err(|error| failed("a descriptor could not be sent", error))?;
    Ok(())
}

/// Receive one descriptor [`send`] sent over `socket`, with its marker byte,
/// made this process's own and closed on any program it starts.
pub fn receive(socket: &UnixStream) -> Result<(u8, OwnedFd), RunnerError> {
    let mut space = vec![MaybeUninit::<u8>::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut control = RecvAncillaryBuffer::new(&mut space);
    let mut byte = [0_u8];
    let received = rustix::net::recvmsg(
        socket,
        &mut [IoSliceMut::new(&mut byte)],
        &mut control,
        RecvFlags::empty(),
    )
    .map_err(|error| failed("a descriptor was not received", error))?;
    if received.bytes == 0 {
        return Err(failed(
            "a descriptor was not received",
            "the sender closed its end before it sent one",
        ));
    }
    let mut owned = None;
    for message in control.drain() {
        if let RecvAncillaryMessage::ScmRights(mut fds) = message
            && owned.is_none()
        {
            owned = fds.next();
        }
    }
    let owned = owned.ok_or_else(|| failed("a descriptor was not received", "none came with it"))?;
    rustix::io::fcntl_setfd(owned.as_fd(), FdFlags::CLOEXEC)
        .map_err(|error| failed("a received descriptor could not be marked", error))?;
    Ok((byte[0], owned))
}

/// Keep `fd` open across this process replacing its program, answering
/// its number.
pub(crate) fn keep(fd: BorrowedFd<'_>) -> Result<RawFd, RunnerError> {
    rustix::io::fcntl_setfd(fd, FdFlags::empty())
        .map_err(|error| failed("a descriptor could not be kept", error))?;
    Ok(fd.as_raw_fd())
}

/// Close `fd` again when this process replaces its program: undoes [`keep`].
pub(crate) fn unkeep(fd: BorrowedFd<'_>) -> Result<(), RunnerError> {
    rustix::io::fcntl_setfd(fd, FdFlags::CLOEXEC)
        .map_err(|error| failed("a descriptor could not be closed on replacement", error))
}

/// Own the descriptor `raw` names and close the number itself, so only the
/// owned descriptor holds the open file.
pub(crate) fn take(raw: RawFd) -> Result<OwnedFd, RunnerError> {
    let owned = own(raw)?;
    nix::unistd::close(raw)
        .map_err(|error| failed(&format!("descriptor {raw} could not be closed"), error))?;
    Ok(owned)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;

    #[test]
    fn an_owned_number_names_the_same_open_file() -> Result<(), Box<dyn std::error::Error>> {
        let (mut left, right) = std::os::unix::net::UnixStream::pair()?;
        let owned = super::own(right.as_raw_fd())?;
        drop(right);
        let mut again = std::fs::File::from(owned);
        left.write_all(b"same")?;
        let mut read = [0_u8; 4];
        again.read_exact(&mut read)?;
        assert_eq!(&read, b"same");
        Ok(())
    }

    #[test]
    fn a_number_that_names_nothing_is_refused() {
        let refused = super::own(1_000_000).map(drop);
        assert!(refused.is_err(), "the kernel refuses a number naming nothing");
    }
}
