//! The trust dialog a Claude Code run may still show after its folder was
//! recorded as trusted. The row is written into a shared file the harness
//! itself rewrites whole during its own start, so another harness starting
//! between Lys's write and this run's launch can write the file back without
//! the row, and the run sits at the dialog nobody sees. The runner therefore
//! watches the run's first screen for the dialog it knows, answers it for the
//! named folder only, and says in the feed what it did.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::{Sessions, lifecycle, unknown};
use crate::error::RunnerError;
use crate::protocol::Key;

/// The question the dialog asks.
pub(crate) const QUESTION: &[u8] = b"Do you trust the files in this folder?";
/// The dialog's first option, painted after the folder it names; once this
/// is on the screen the dialog is whole and its folder can be read.
const OPTION: &[u8] = b"Yes, proceed";
/// How much output counts as the harness's first screen. Past this much with
/// no question, the harness is past its start and the watch ends; this is
/// a bound on what the watch reads, not a wait.
const FIRST_SCREEN_BYTES: u64 = 64 * 1024;
/// The feed state: the dialog showed for the run's folder and was answered.
pub(crate) const ANSWERED: &str = "trust_answered";
/// The feed state: the dialog showed for another folder and was left alone.
pub(crate) const UNANSWERED: &str = "trust_dialog_unanswered";

/// What the watch of the first screen saw.
enum Seen {
    /// The dialog, naming the run's folder.
    Named,
    /// The dialog, naming some other folder.
    Other,
    /// No dialog: the harness is past its start, or has ended.
    Passed,
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

impl Sessions {
    /// Watch session `id`'s first screen for the trust dialog, in its own
    /// thread; `directory` is the folder recorded as trusted for the run.
    pub(crate) fn watch_trust_dialog(
        self: &Arc<Self>,
        id: &str,
        directory: String,
    ) -> Result<(), RunnerError> {
        let sessions = Arc::clone(self);
        let owned = id.to_owned();
        std::thread::Builder::new()
            .name("runner-trust-dialog".to_owned())
            .spawn(move || {
                if let Err(error) = sessions.answer_trust_dialog(&owned, &directory) {
                    crate::error::said(&format!("trust_dialog_watch_failed: {owned}: {error}"));
                }
            })
            .map(drop)
            .map_err(|error| RunnerError::refused("trust_watch_failed", error.to_string()))
    }

    /// Read session `id`'s output from the spawn until the dialog is whole
    /// on the screen, the first screen has passed without it, or the process
    /// has ended. The dialog naming `directory` is answered with Enter, the
    /// selected `Yes, proceed`; one naming any other folder is left alone.
    /// Either is said in the feed.
    fn answer_trust_dialog(&self, id: &str, directory: &str) -> Result<(), RunnerError> {
        let never_left = AtomicBool::new(false);
        let mut from: Option<u64> = None;
        let seen = self.until(id, &never_left, |state, _| {
            if state.ended().is_some() {
                return Some(Ok(Seen::Passed));
            }
            let scrollback = state.scrollback();
            let start = *from.get_or_insert(scrollback.oldest());
            let kept = match scrollback.from(start.max(scrollback.oldest())) {
                Ok(kept) => kept,
                Err(expired) => return Some(Err(expired)),
            };
            let Some(at) = find(&kept, QUESTION) else {
                let read = u64::try_from(kept.len()).unwrap_or(u64::MAX);
                return (read > FIRST_SCREEN_BYTES).then_some(Ok(Seen::Passed));
            };
            let dialog = &kept[at + QUESTION.len()..];
            find(dialog, OPTION)?;
            let named = find(dialog, directory.as_bytes()).is_some();
            Some(Ok(if named { Seen::Named } else { Seen::Other }))
        })?;
        let (state, words) = match seen {
            Seen::Passed => return Ok(()),
            Seen::Named => {
                let writer = {
                    let mut table = self.lock()?;
                    let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
                    session.live(id)?.writer.clone()
                };
                writer.write(Key::Enter.bytes().to_vec())?;
                (
                    ANSWERED,
                    format!(
                        "the trust dialog for {directory} showed and was answered with Enter, the selected Yes, proceed; the row recorded before the spawn was not in the file when the harness read it"
                    ),
                )
            }
            Seen::Other => (
                UNANSWERED,
                format!(
                    "the trust dialog showed for a folder other than {directory} and was left alone"
                ),
            ),
        };
        let mut table = self.lock()?;
        lifecycle::trust_dialog(&mut table, id, state, words)?;
        drop(table);
        self.writer.barrier()?;
        self.wake();
        Ok(())
    }
}
