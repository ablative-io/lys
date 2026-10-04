//! What a session was started with beside its launch.

use crate::judge::Policy;
use crate::peer::Leader;
use crate::tracking::Tracking;
use crate::tracking_proxy::ProxyTracking;

/// What a session was started with beside its launch, and where its turns
/// stand.
#[derive(Debug, Clone, Default)]
pub struct Guard {
    /// The tool-boundary policy installed for the launch.
    pub policy: Option<Policy>,
    /// How its harness is tracked.
    pub tracking: Option<Tracking>,
    /// How it is tracked through the proxy, when its calls go through it.
    pub proxy: Option<ProxyTracking>,
    /// Its leader, as spawned.
    pub leader: Option<Leader>,
    /// Its bound working directory.
    pub cwd: String,
    /// Whether it is between turns, as its harness last said.
    pub idle: bool,
}
