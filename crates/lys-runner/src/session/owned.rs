//! Owned starts: a session begun for a verified responsible person, whose
//! input then needs a verified caller and a live grant (see
//! [`crate::legacy_input`]).

use std::sync::Arc;

use crate::error::RunnerError;
use crate::judge::Policy;
use crate::protocol::Launch;
use crate::tracking::Tracking;

use super::Sessions;

impl Sessions {
    /// Start with a verified responsible person supplied by the admitting caller.
    ///
    /// # Errors
    /// Returns invalid responsibility, launch or persistence errors by name.
    pub fn start_for(
        self: &Arc<Self>,
        launch: Launch,
        responsible: &str,
    ) -> Result<(u32, u64), RunnerError> {
        self.begin_for(launch, None, None, responsible)
    }

    /// Begin an owned session without deriving a person from a server key.
    ///
    /// # Errors
    /// Returns invalid responsibility, tracking, launch or persistence errors by name.
    pub fn begin_for(
        self: &Arc<Self>,
        launch: Launch,
        policy: Option<Policy>,
        tracking: Option<Tracking>,
        responsible: &str,
    ) -> Result<(u32, u64), RunnerError> {
        if responsible.is_empty()
            || responsible.trim() != responsible
            || responsible.chars().any(char::is_control)
            || responsible == "lys"
        {
            return Err(RunnerError::refused(
                "SessionResponsibleInvalid",
                "a verified responsible person is required",
            ));
        }
        self.begin_owned(launch, policy, tracking, None, Some(responsible.to_owned()))
    }

    /// Start `launch` as [`Sessions::start`] does, holding `policy` for its
    /// judge and tracking its model calls through the proxy as `proxy`
    /// says: its figures are read from the proxy's usage file for its run
    /// key, not from its harness.
    ///
    /// # Errors
    /// Refuses a run key that is not one before anything runs, and returns
    /// launch or persistence errors by name.
    pub fn begin_proxied(
        self: &Arc<Self>,
        launch: Launch,
        policy: Option<Policy>,
        proxy: crate::tracking_proxy::ProxyTracking,
    ) -> Result<(u32, u64), RunnerError> {
        proxy.checked()?;
        self.begin_owned(launch, policy, None, Some(proxy), None)
    }

    /// Say where the proxy on this machine keeps its state: the directory
    /// that holds its `journal` and its `usage`. Said once, when the runner
    /// starts; a session tracked through the proxy is followed there.
    pub fn proxy_state(&self, state: &std::path::Path) {
        if self.proxy_usage.set(state.join("usage")).is_err() {
            crate::error::said("the proxy's state was already said; the first stands");
        }
    }

    /// Begin an owned session tracked through the proxy, as
    /// [`Sessions::begin_proxied`] does.
    ///
    /// # Errors
    /// Returns invalid responsibility, tracking, launch or persistence errors by name.
    pub fn begin_proxied_for(
        self: &Arc<Self>,
        launch: Launch,
        policy: Option<Policy>,
        proxy: crate::tracking_proxy::ProxyTracking,
        responsible: &str,
    ) -> Result<(u32, u64), RunnerError> {
        if responsible.is_empty()
            || responsible.trim() != responsible
            || responsible.chars().any(char::is_control)
            || responsible == "lys"
        {
            return Err(RunnerError::refused(
                "SessionResponsibleInvalid",
                "a verified responsible person is required",
            ));
        }
        proxy.checked()?;
        self.begin_owned(
            launch,
            policy,
            None,
            Some(proxy),
            Some(responsible.to_owned()),
        )
    }
}
