//! The named keys a session can be sent, and the bytes each one types.

use serde::{Deserialize, Serialize};

/// A named key a session can be sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    /// Return.
    Enter,
    /// Tab.
    Tab,
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// Delete forward.
    Delete,
    /// Cursor up.
    Up,
    /// Cursor down.
    Down,
    /// Cursor left.
    Left,
    /// Cursor right.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Space.
    Space,
    /// Control-C, interrupt.
    CtrlC,
    /// Control-D, end of input.
    CtrlD,
    /// Control-L, redraw.
    CtrlL,
    /// Control-R, reverse search.
    CtrlR,
    /// Control-Z, suspend.
    CtrlZ,
}

impl Key {
    /// The bytes the key sends to a terminal.
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Self::Enter => b"\r",
            Self::Tab => b"\t",
            Self::Escape => b"\x1b",
            Self::Backspace => b"\x7f",
            Self::Delete => b"\x1b[3~",
            Self::Up => b"\x1b[A",
            Self::Down => b"\x1b[B",
            Self::Right => b"\x1b[C",
            Self::Left => b"\x1b[D",
            Self::Home => b"\x1b[H",
            Self::End => b"\x1b[F",
            Self::PageUp => b"\x1b[5~",
            Self::PageDown => b"\x1b[6~",
            Self::Space => b" ",
            Self::CtrlC => b"\x03",
            Self::CtrlD => b"\x04",
            Self::CtrlL => b"\x0c",
            Self::CtrlR => b"\x12",
            Self::CtrlZ => b"\x1a",
        }
    }
}
