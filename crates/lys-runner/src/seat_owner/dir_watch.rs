//! Told when the owner's own folder is gone, never polled for it
//! (AGENTS-018 R2).
//!
//! macOS registers `EVFILT_VNODE` on the folder's descriptor for
//! `NOTE_DELETE`, `NOTE_RENAME` and `NOTE_REVOKE`. The descriptor is held
//! open for the watch's life: the kernel names the folder by it after the
//! name is gone, which is what the event is for, and `NOTE_DELETE` comes
//! when the name is unlinked whoever still holds the folder.
//!
//! Linux cannot be told the same way by the folder alone. Its
//! `IN_DELETE_SELF` comes only when the kernel lets the folder go, which is
//! when the last thing holding it lets go, and the owner holds its own
//! folder for its whole life (its lock file and log are open inside it), so
//! a removal of the folder under a running owner was never told at all. The
//! name leaving the folder's parent is told at the moment of the unlink, so
//! Linux watches the parent too: `IN_DELETE`, `IN_MOVED_FROM` or
//! `IN_MOVED_TO` naming the folder end the watch, as do `IN_DELETE_SELF` and
//! `IN_MOVE_SELF` on the folder (a rename is told at once) and on the parent
//! (a parent renamed away takes the folder's path with it). A queue that
//! overflowed has lost what it would have told, so it is answered by asking
//! whether the folder's path still names the folder that was watched.
//!
//! The registration is made when the owner starts, so an owner whose folder
//! cannot be watched never starts; the wait is the kernel's event.

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
    /// The folder's own watch: renamed away, or let go by the kernel.
    folder: nix::sys::inotify::WatchDescriptor,
    /// The parent's watch: the folder's name leaving it, told at the unlink.
    parent: nix::sys::inotify::WatchDescriptor,
    /// The folder's name in its parent.
    name: std::ffi::OsString,
    /// The folder's path with no links left in it, asked after an overflow.
    path: std::path::PathBuf,
    /// The folder's device and inode when the watch was made.
    identity: (u64, u64),
}

#[cfg(target_os = "linux")]
impl DirWatch {
    /// Registers the watch on `dir` and on the folder that holds it.
    ///
    /// # Errors
    ///
    /// The words of the lookup or the registration that was refused.
    pub(super) fn open(dir: &Path) -> Result<Self, String> {
        use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify};
        // The real path: the name the kernel unlinks is the real folder's
        // name in its real parent, not a link's.
        let path = std::fs::canonicalize(dir).map_err(|error| format!("the path: {error}"))?;
        let name = path
            .file_name()
            .ok_or("the folder has no name in a parent folder")?
            .to_os_string();
        let parent_path = path.parent().ok_or("the folder has no parent folder")?;
        let inotify =
            Inotify::init(InitFlags::IN_CLOEXEC).map_err(|error| format!("inotify: {error}"))?;
        // The parent first: a folder removed between the two registrations
        // is then either told by the parent or refused by the folder's own.
        let parent = inotify
            .add_watch(
                parent_path,
                AddWatchFlags::IN_DELETE
                    | AddWatchFlags::IN_MOVED_FROM
                    | AddWatchFlags::IN_MOVED_TO
                    | AddWatchFlags::IN_DELETE_SELF
                    | AddWatchFlags::IN_MOVE_SELF
                    | AddWatchFlags::IN_ONLYDIR,
            )
            .map_err(|error| format!("inotify watch on the parent: {error}"))?;
        let folder = inotify
            .add_watch(
                &path,
                AddWatchFlags::IN_DELETE_SELF
                    | AddWatchFlags::IN_MOVE_SELF
                    | AddWatchFlags::IN_ONLYDIR,
            )
            .map_err(|error| format!("inotify watch: {error}"))?;
        let identity = identity_of(&path).map_err(|error| format!("the folder: {error}"))?;
        Ok(Self {
            inotify,
            folder,
            parent,
            name,
            path,
            identity,
        })
    }

    /// Waits until the folder is removed or renamed away, or its parent is,
    /// answering which; a watch that ends without that answers why.
    pub(super) fn wait(self) -> String {
        loop {
            match self.inotify.read_events() {
                Ok(events) => {
                    if let Some(words) = events.iter().find_map(|event| self.ends(event)) {
                        return words;
                    }
                }
                Err(nix::errno::Errno::EINTR) => {}
                Err(error) => return format!("inotify read: {error}"),
            }
        }
    }

    /// What one event says of the folder's end, if it says anything.
    fn ends(&self, event: &nix::sys::inotify::InotifyEvent) -> Option<String> {
        use nix::sys::inotify::AddWatchFlags;
        let mask = event.mask;
        if mask.contains(AddWatchFlags::IN_Q_OVERFLOW) {
            return match identity_of(&self.path) {
                Ok(identity) if identity == self.identity => None,
                Ok(_) => Some("replaced (told after the watch overflowed)".to_owned()),
                Err(error) => Some(format!("gone (told after the watch overflowed): {error}")),
            };
        }
        if event.wd == self.folder {
            if mask.contains(AddWatchFlags::IN_DELETE_SELF) {
                return Some("removed".to_owned());
            }
            if mask.contains(AddWatchFlags::IN_MOVE_SELF) {
                return Some("renamed away".to_owned());
            }
            if mask.contains(AddWatchFlags::IN_IGNORED) {
                return Some("no longer watched by the kernel".to_owned());
            }
            return None;
        }
        if event.wd != self.parent {
            return None;
        }
        if event.name.as_deref() == Some(self.name.as_os_str())
            && mask.contains(AddWatchFlags::IN_ISDIR)
        {
            if mask.contains(AddWatchFlags::IN_DELETE) {
                return Some("removed".to_owned());
            }
            if mask.contains(AddWatchFlags::IN_MOVED_FROM) {
                return Some("renamed away".to_owned());
            }
            if mask.contains(AddWatchFlags::IN_MOVED_TO) {
                return Some("replaced by a folder renamed onto its name".to_owned());
            }
        }
        if mask.contains(AddWatchFlags::IN_DELETE_SELF) {
            return Some("removed with its parent folder".to_owned());
        }
        if mask.contains(AddWatchFlags::IN_MOVE_SELF) {
            return Some("renamed away with its parent folder".to_owned());
        }
        if mask.contains(AddWatchFlags::IN_IGNORED) {
            return Some("no longer watched by the kernel through its parent folder".to_owned());
        }
        None
    }
}

/// A folder's device and inode, by its path.
#[cfg(target_os = "linux")]
fn identity_of(path: &Path) -> std::io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path)?;
    Ok((metadata.dev(), metadata.ino()))
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

    /// Opens the watch on `dir`, waits on it in a thread, does `act` once
    /// the thread runs, and answers the watch's words.
    fn told_after(
        dir: &std::path::Path,
        act: impl FnOnce() -> std::io::Result<()>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let watch = DirWatch::open(dir)?;
        let (started, start) = std::sync::mpsc::channel();
        let waited = std::thread::spawn(move || {
            started.send(()).map_err(|error| error.to_string())?;
            Ok::<String, String>(watch.wait())
        });
        start.recv()?;
        act()?;
        Ok(waited.join().map_err(|error| format!("{error:?}"))??)
    }

    #[test]
    fn a_removed_folder_is_told_as_removed() -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let dir = temporary.path().join("owner");
        std::fs::create_dir(&dir)?;
        let words = told_after(&dir, || std::fs::remove_dir(&dir))?;
        assert_eq!(words, "removed");
        Ok(())
    }

    /// The owner holds its own folder (its lock and log are open inside
    /// it); the removal is told while it still does.
    #[test]
    fn a_removed_folder_this_process_holds_is_told_as_removed()
    -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let dir = temporary.path().join("owner");
        std::fs::create_dir_all(dir.join("runner"))?;
        let held = std::fs::File::create(dir.join("runner/lock"))?;
        let words = told_after(&dir, || std::fs::remove_dir_all(&dir))?;
        assert_eq!(words, "removed");
        drop(held);
        Ok(())
    }

    #[test]
    fn a_renamed_folder_is_told_as_renamed_away() -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let dir = temporary.path().join("owner");
        std::fs::create_dir(&dir)?;
        let held = std::fs::File::create(dir.join("lock"))?;
        let words = told_after(&dir, || {
            std::fs::rename(&dir, temporary.path().join("elsewhere"))
        })?;
        assert_eq!(words, "renamed away");
        drop(held);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_parent_renamed_away_is_told() -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let parent = temporary.path().join("owners");
        let dir = parent.join("owner");
        std::fs::create_dir_all(&dir)?;
        let held = std::fs::File::create(dir.join("lock"))?;
        let words = told_after(&dir, || {
            std::fs::rename(&parent, temporary.path().join("elsewhere"))
        })?;
        assert_eq!(words, "renamed away with its parent folder");
        drop(held);
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
