//! Told when the owner's own folder is gone, never polled for it
//! (AGENTS-018 R2).
//!
//! macOS registers `EVFILT_VNODE` on the folder's descriptor for
//! `NOTE_DELETE`, `NOTE_RENAME` and `NOTE_REVOKE`; Linux adds an inotify
//! watch for `IN_DELETE_SELF` and `IN_MOVE_SELF`. The registration is made
//! when the owner starts, so an owner whose folder cannot be watched never
//! starts; the wait is the kernel's event. The descriptor is held open for
//! the watch's life: the kernel names the folder by it after the name is
//! gone, which is what the event is for.

use std::path::Path;

/// The folder watch, registered; waited on once.
#[cfg(target_os = "macos")]
pub(super) struct DirWatch {
    queue: nix::sys::event::Kqueue,
    /// Kept open: the kqueue identifies the folder by this descriptor.
    held: std::fs::File,
}

#[cfg(target_os = "macos")]
impl DirWatch {
    /// Registers the watch on `dir`.
    ///
    /// # Errors
    ///
    /// The words of the open or the registration that was refused.
    pub(super) fn open(dir: &Path) -> Result<Self, String> {
        use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent, Kqueue};
        use std::os::fd::AsRawFd;
        let held = std::fs::File::open(dir).map_err(|error| format!("open: {error}"))?;
        let queue = Kqueue::new().map_err(|error| format!("kqueue: {error}"))?;
        let ident = usize::try_from(held.as_raw_fd())
            .map_err(|error| format!("the folder's descriptor: {error}"))?;
        let change = KEvent::new(
            ident,
            EventFilter::EVFILT_VNODE,
            EventFlag::EV_ADD | EventFlag::EV_CLEAR | EventFlag::EV_RECEIPT,
            FilterFlag::NOTE_DELETE | FilterFlag::NOTE_RENAME | FilterFlag::NOTE_REVOKE,
            0,
            0,
        );
        let mut receipt = [change];
        match queue.kevent(&[change], &mut receipt, None) {
            Ok(1) if receipt[0].flags().contains(EventFlag::EV_ERROR) => match receipt[0].data() {
                0 => {}
                value => return Err(format!("EVFILT_VNODE registration: errno {value}")),
            },
            Ok(count) => {
                return Err(format!("EVFILT_VNODE registration answered {count} events"));
            }
            Err(error) => return Err(format!("EVFILT_VNODE registration: {error}")),
        }
        Ok(Self { queue, held })
    }

    /// Waits until the folder is removed, renamed away or revoked, answering
    /// which; a watch that ends without that answers why.
    pub(super) fn wait(self) -> String {
        use nix::sys::event::{EventFilter, FilterFlag, KEvent};
        let mut events = [KEvent::new(
            0,
            EventFilter::EVFILT_VNODE,
            nix::sys::event::EventFlag::empty(),
            FilterFlag::empty(),
            0,
            0,
        )];
        let words = loop {
            match self.queue.kevent(&[], &mut events, None) {
                Ok(0) => break "the folder watch ended without an event".to_owned(),
                Ok(_) => {
                    let flags = events[0].fflags();
                    break if flags.contains(FilterFlag::NOTE_DELETE) {
                        "removed".to_owned()
                    } else if flags.contains(FilterFlag::NOTE_RENAME) {
                        "renamed away".to_owned()
                    } else if flags.contains(FilterFlag::NOTE_REVOKE) {
                        "revoked".to_owned()
                    } else {
                        format!(
                            "the kqueue answered an event that is not the folder's end ({flags:?})"
                        )
                    };
                }
                Err(nix::errno::Errno::EINTR) => {}
                Err(error) => break format!("kevent: {error}"),
            }
        };
        drop(self.held);
        words
    }
}

/// The folder watch, registered; waited on once.
#[cfg(target_os = "linux")]
pub(super) struct DirWatch {
    inotify: nix::sys::inotify::Inotify,
}

#[cfg(target_os = "linux")]
impl DirWatch {
    /// Registers the watch on `dir`.
    ///
    /// # Errors
    ///
    /// The words of the registration that was refused.
    pub(super) fn open(dir: &Path) -> Result<Self, String> {
        use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify};
        let inotify =
            Inotify::init(InitFlags::IN_CLOEXEC).map_err(|error| format!("inotify: {error}"))?;
        inotify
            .add_watch(
                dir,
                AddWatchFlags::IN_DELETE_SELF | AddWatchFlags::IN_MOVE_SELF,
            )
            .map_err(|error| format!("inotify watch: {error}"))?;
        Ok(Self { inotify })
    }

    /// Waits until the folder is removed or renamed away, answering which;
    /// a watch that ends without that answers why.
    pub(super) fn wait(self) -> String {
        use nix::sys::inotify::AddWatchFlags;
        loop {
            match self.inotify.read_events() {
                Ok(events) => {
                    for event in events {
                        if event.mask.contains(AddWatchFlags::IN_DELETE_SELF) {
                            return "removed".to_owned();
                        }
                        if event.mask.contains(AddWatchFlags::IN_MOVE_SELF) {
                            return "renamed away".to_owned();
                        }
                        if event.mask.contains(AddWatchFlags::IN_IGNORED) {
                            return "no longer watched by the kernel".to_owned();
                        }
                    }
                }
                Err(nix::errno::Errno::EINTR) => {}
                Err(error) => return format!("inotify read: {error}"),
            }
        }
    }
}

/// No folder watch on this system: an owner refuses to start.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) struct DirWatch;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl DirWatch {
    /// Refuses: this system has no folder watch this runner uses.
    ///
    /// # Errors
    ///
    /// Always.
    pub(super) fn open(_dir: &Path) -> Result<Self, String> {
        Err("this system has no folder watch this runner uses".to_owned())
    }

    /// Never reached: `open` refuses.
    pub(super) fn wait(self) -> String {
        "no folder watch".to_owned()
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::DirWatch;

    #[test]
    fn a_removed_folder_is_told_as_removed() -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let dir = temporary.path().join("owner");
        std::fs::create_dir(&dir)?;
        let watch = DirWatch::open(&dir)?;
        let (started, start) = std::sync::mpsc::channel();
        let waited = std::thread::spawn(move || {
            started.send(()).map_err(|error| error.to_string())?;
            Ok::<String, String>(watch.wait())
        });
        start.recv()?;
        std::fs::remove_dir(&dir)?;
        let words = waited.join().map_err(|error| format!("{error:?}"))??;
        assert_eq!(words, "removed");
        Ok(())
    }

    #[test]
    fn a_folder_that_is_not_there_cannot_be_watched() {
        let temporary = match tempfile::tempdir() {
            Ok(temporary) => temporary,
            Err(error) => panic!("tempdir: {error}"),
        };
        assert!(DirWatch::open(&temporary.path().join("absent")).is_err());
    }
}
