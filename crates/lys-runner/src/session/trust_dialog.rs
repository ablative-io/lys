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
    /// No dialog before the process ended.
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
    /// on the screen or the process has ended; nothing else ends the watch.
    /// Each look reads only the bytes since the last, holding a
    /// question's length of overlap, so a long run costs nothing. The dialog
    /// naming `directory` is answered with Enter, the selected
    /// `Yes, proceed`; one naming any other folder is left alone. Either is
    /// said in the feed.
    fn answer_trust_dialog(&self, id: &str, directory: &str) -> Result<(), RunnerError> {
        let never_left = AtomicBool::new(false);
        let mut cursor: Option<u64> = None;
        let mut held: Vec<u8> = Vec::new();
        let seen = self.until(id, &never_left, |state, _| {
            if state.ended().is_some() {
                return Some(Ok(Seen::Passed));
            }
            let scrollback = state.scrollback();
            let from = cursor.get_or_insert(scrollback.oldest());
            let fresh = match scrollback.from((*from).max(scrollback.oldest())) {
                Ok(fresh) => fresh,
                Err(expired) => return Some(Err(expired)),
            };
            *from = scrollback.end();
            held.extend_from_slice(&fresh);
            let Some(at) = find(&held, QUESTION) else {
                // Keep only what a question split across two looks needs.
                let keep_from = held.len().saturating_sub(QUESTION.len() - 1);
                held.drain(..keep_from);
                return None;
            };
            let dialog = &held[at + QUESTION.len()..];
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
