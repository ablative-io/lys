//! A person's canvas as it is kept: one file for each person, in a folder
//! beside the directory log, holding the arrangement they are working in
//! and every layout they saved by name.
//!
//! A change replaces the person's file whole: written beside it, flushed,
//! moved over it, and the folder flushed. So the file holds the old canvas
//! or the new and never part of either; nothing is appended, nothing is
//! replayed when the service starts, and nothing a person replaced is left
//! behind to be swept. Nothing is opened or written at start: a person's
//! file is read when they ask and made at their first change.
//!
//! A canvas is a person's own picture. It gives nothing and takes nothing:
//! no grant, check or record reads it.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lys_identity::PersonId;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::error_canvas::CanvasError;

/// Where a window sits on the canvas and how large it is.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasPlace)]
pub struct Place {
    /// Its left edge.
    pub x: f64,
    /// Its top edge.
    pub y: f64,
    /// Its width.
    pub w: f64,
    /// Its height.
    pub h: f64,
}

/// A labelled box a person drew around windows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasGroup)]
pub struct Group {
    /// The name the page gave it.
    pub id: String,
    /// Its label, in the person's words.
    pub label: String,
    /// The colour the person gave it, by the palette's name; absent for Lys's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    /// Its left edge.
    pub x: f64,
    /// Its top edge.
    pub y: f64,
    /// Its width.
    pub w: f64,
    /// Its height.
    pub h: f64,
}

/// A note a person wrote on the canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasNote)]
pub struct Note {
    /// The name the page gave it.
    pub id: String,
    /// What the person wrote.
    pub text: String,
    /// The colour the person gave it, by the palette's name; absent for Lys's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    /// Its left edge.
    pub x: f64,
    /// Its top edge.
    pub y: f64,
    /// Its width.
    pub w: f64,
    /// Its height.
    pub h: f64,
}

/// The view a widget is in beyond its pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[schema(as = CanvasWidgetView)]
pub enum WidgetView {
    /// Its medium face: a few figures side by side.
    Faces,
    /// Opened out to everything it holds.
    Detail,
    /// Its settings.
    Settings,
}

/// How a widget draws a figure that is a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[schema(as = CanvasWidgetLook)]
pub enum WidgetLook {
    /// With a bar.
    Bar,
    /// As a dial.
    Dial,
}

/// A widget a person put on the canvas: one kind of what Lys holds about
/// agents. The lines drawn to it say which agents it counts; the page
/// knows the kinds, and one it does not know is kept as it was sent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasWidget)]
pub struct Widget {
    /// The name the page gave it.
    pub id: String,
    /// Its kind, by the page's name for it.
    pub kind: String,
    /// The one figure it shows as a pill, when the person chose one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shows: Option<String>,
    /// The figures its medium face holds, when the person chose them;
    /// an empty list is a face the person emptied, and is kept as one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faces: Option<Vec<String>>,
    /// The view it is in; absent while it is a pill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<WidgetView>,
    /// How a level is drawn; absent for its number alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<WidgetLook>,
    /// Locked, it is not moved, changed or taken away until it is unlocked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    /// The colour the person gave it, by the palette's name; absent for Lys's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    /// Its left edge.
    pub x: f64,
    /// Its top edge.
    pub y: f64,
    /// Its width when it is opened out.
    pub w: f64,
    /// Its height when it is opened out.
    pub h: f64,
}

/// An edge of a thing on the canvas, where a line leaves it or meets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[schema(as = CanvasSide)]
pub enum Side {
    /// Its top edge.
    Top,
    /// Its right edge.
    Right,
    /// Its bottom edge.
    Bottom,
    /// Its left edge.
    Left,
}

/// A line a person drew between two things on the canvas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasLink)]
pub struct Link {
    /// The name the page gave it.
    pub id: String,
    /// The thing it starts from.
    pub from: String,
    /// The thing it goes to.
    pub to: String,
    /// The edge it leaves from; absent when the facing edge is taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_side: Option<Side>,
    /// The edge it meets; absent when the facing edge is taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_side: Option<Side>,
}

/// The view the Operations page is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[schema(as = CanvasOperationsView)]
pub enum OperationsView {
    /// The network: every computer, its agents and their seats, as a tree
    /// beside the chosen seat's terminal.
    Tree,
    /// The canvas: the windows and marks arranged on the board.
    Canvas,
    /// The sessions: the running list and the seats, as lists.
    Sessions,
}

/// The width the network tree may be kept at, in pixels: about a fifth of a
/// screen, never so narrow that names are cut nor so wide that the terminal
/// beside it is.
pub const PANE_WIDTH: std::ops::RangeInclusive<f64> = 200.0..=640.0;

/// How the Operations page was left: which view, which seat is chosen in
/// the tree, and how wide the tree is. Kept with the arrangement so it is
/// the same on any computer the person signs in at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasOperations)]
pub struct Operations {
    /// The view the page is in.
    pub view: OperationsView,
    /// The seat chosen in the tree, by its name on the page; null for none.
    #[serde(default)]
    pub selected: Option<String>,
    /// The width of the network tree, in pixels, within [`PANE_WIDTH`].
    pub pane_width: f64,
}

impl Operations {
    /// Refuses, in words, what cannot be kept: a pane width that is not a
    /// finite number within [`PANE_WIDTH`], or a chosen seat with no name.
    ///
    /// # Errors
    /// Refuses a pane width outside the finite permitted range or a chosen
    /// seat with an empty name.
    pub fn checked(&self) -> Result<(), ServerError> {
        let refused = |words: String| ServerError::Canvas(CanvasError::Refused { words });
        if !self.pane_width.is_finite() || !PANE_WIDTH.contains(&self.pane_width) {
            return Err(refused(format!(
                "the network tree's width {} is outside {} to {} pixels: keep it within them",
                self.pane_width,
                PANE_WIDTH.start(),
                PANE_WIDTH.end()
            )));
        }
        if self
            .selected
            .as_deref()
            .is_some_and(|seat| seat.trim().is_empty())
        {
            return Err(refused(
                "the chosen seat has no name: name it, or choose none with null".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Everything a person arranged: where each window sits, which terminals
/// are open, what they drew, and how they left the Operations page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasArrangement)]
pub struct Arrangement {
    /// Each window's place. An agent's window is named by the agent, so
    /// the place holds when the agent starts again.
    pub boxes: BTreeMap<String, Place>,
    /// The windows whose terminals are open.
    pub open: Vec<String>,
    /// The labelled boxes.
    #[serde(default)]
    pub groups: Vec<Group>,
    /// The notes.
    #[serde(default)]
    pub notes: Vec<Note>,
    /// The lines.
    #[serde(default)]
    pub links: Vec<Link>,
    /// The widgets.
    #[serde(default)]
    pub widgets: Vec<Widget>,
    /// How the Operations page was left; absent until the person changed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operations: Option<Operations>,
}

impl Arrangement {
    /// Refuses, in words, what cannot be kept as it is.
    ///
    /// # Errors
    /// Refuses a pane width outside the finite permitted range or a chosen
    /// seat with an empty name in the retained layout.
    pub fn checked(&self) -> Result<(), ServerError> {
        match &self.operations {
            Some(operations) => operations.checked(),
            None => Ok(()),
        }
    }
}

/// A layout saved by name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasLayout)]
pub struct Layout {
    /// Its name, in the person's words.
    pub name: String,
    /// When it was last saved, in seconds since the Unix epoch.
    pub saved_at: u64,
    /// What it holds.
    pub arrangement: Arrangement,
}

/// A person's canvas: the arrangement they are working in and their saved
/// layouts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = CanvasView)]
pub struct Canvas {
    /// The arrangement being worked in; null before the first change.
    pub arrangement: Option<Arrangement>,
    /// Every layout saved by name, in the order first saved.
    pub layouts: Vec<Layout>,
}

/// The canvases, one file for each person.
pub struct CanvasStore {
    dir: PathBuf,
    /// One change at a time, so a change reads what the one before it kept.
    changing: Mutex<()>,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::Canvas(CanvasError::Unavailable {
        reason: what.to_string(),
    })
}

/// A layout's name as it is kept: without the space around it, holding
/// something, and holding no control character.
///
/// # Errors
/// Refuses an empty name after trimming or a name with a control character.
pub fn layout_name(given: &str) -> Result<String, ServerError> {
    let name = given.trim();
    if name.is_empty() {
        return Err(ServerError::Canvas(CanvasError::Refused {
            words: "the layout has no name: give it one".to_owned(),
        }));
    }
    if name.chars().any(char::is_control) {
        return Err(ServerError::Canvas(CanvasError::Refused {
            words: "the layout's name holds a control character: name it in plain words".to_owned(),
        }));
    }
    Ok(name.to_owned())
}

impl CanvasStore {
    /// The canvases kept in the folder `canvas` beside the directory log
    /// `log_dir`. Nothing is read or made here.
    #[must_use]
    pub fn beside(log_dir: &Path) -> Self {
        Self {
            dir: log_dir.with_file_name("canvas"),
            changing: Mutex::new(()),
        }
    }

    fn file(&self, person: PersonId) -> PathBuf {
        self.dir.join(format!("{person}.json"))
    }

    /// `person`'s canvas; an empty one when they have never changed it.
    ///
    /// # Errors
    /// Refuses unreadable or malformed retained canvas bytes.
    pub fn read(&self, person: PersonId) -> Result<Canvas, ServerError> {
        match fs::read(self.file(person)) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                unavailable(format!(
                    "the canvas file kept for {person} cannot be read: {error}"
                ))
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Canvas::default()),
            Err(error) => Err(unavailable(error)),
        }
    }

    /// Change `person`'s canvas and keep it whole before answering it.
    ///
    /// # Errors
    /// Refuses a poisoned canvas lock, unreadable canvas, or the caller's
    /// change. Propagates encoding, directory creation and durable replacement
    /// failures.
    pub fn change(
        &self,
        person: PersonId,
        change: impl FnOnce(&mut Canvas) -> Result<(), ServerError>,
    ) -> Result<Canvas, ServerError> {
        // A lock a panic left poisoned is refused by name, as every other lock of the
        // service is; the canvas on disk is whole, and reading it is not refused.
        let _one = self
            .changing
            .lock()
            .map_err(|error| unavailable(format!("the canvas lock is poisoned: {error}")))?;
        let mut canvas = self.read(person)?;
        change(&mut canvas)?;
        let bytes = serde_json::to_vec(&canvas).map_err(unavailable)?;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)
            .map_err(unavailable)?;
        replace(&self.file(person), &bytes).map_err(unavailable)?;
        Ok(canvas)
    }
}

/// Write `bytes` as the whole of `path`: written beside it, flushed, moved
/// over it, and the folder flushed.
pub(crate) fn replace(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let beside = path.with_extension("writing");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&beside)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&beside, path)?;
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => fs::File::open(parent)?.sync_all(),
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "canvas_store_tests.rs"]
mod tests;
