//! The deployment choices passed to the installer.

use std::path::PathBuf;

/// Which kind of install this is. A service install keeps no operator token:
/// nothing on disk can act as the administrator. A development install keeps
/// one for the person developing on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Profile {
    /// The install people depend on.
    #[default]
    Service,
    /// An install being developed on.
    Development,
}

impl Profile {
    /// The word the service's configuration carries.
    pub fn word(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Development => "development",
        }
    }

    /// The profile a configuration's word names; none for any other word.
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "service" => Some(Self::Service),
            "development" => Some(Self::Development),
            _ => None,
        }
    }
}

/// What the operator chose.
#[derive(Debug)]
pub struct Options {
    /// The host's built executable folder, or the installer's siblings.
    pub from: Option<PathBuf>,
    /// Built client pairs grouped by Rust target.
    pub clients: Option<PathBuf>,
    /// The data root; the platform's application data path when absent.
    pub root: Option<PathBuf>,
    /// Which kind of install this is; absent, an earlier install's profile
    /// is kept, else service.
    pub profile: Option<Profile>,
    /// For an unattended install, the administrator's email: the only one
    /// the setup page then takes. Never read from the machine.
    pub admin_email: Option<String>,
    /// A compiled screens package to verify and place.
    pub surface: Option<PathBuf>,
    /// The message service connection to keep in the configuration.
    pub message_service: Option<PathBuf>,
    /// An explicit local identity listener; otherwise keep the recorded listener.
    pub service_port: Option<u16>,
    /// An explicit local broker listener; otherwise keep the recorded listener.
    pub broker_port: Option<u16>,
    /// An explicit local model proxy listener; otherwise keep the recorded listener.
    pub proxy_port: Option<u16>,
}
