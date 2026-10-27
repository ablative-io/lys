impl Sessions {
    /// The sessions recorded in `state_dir`, each kept with a scrollback of
    /// `scrollback` bytes, refused `scrollback_invalid` when that keeps
    /// none. Every session the record holds without an end is reported
    /// ended by the runner's restart, now, with no exit status.
    ///
    /// # Errors
    /// - `RunnerError::State` if persisted sessions, feed or operation state cannot be read, migrated or written.
    /// - `RunnerError::StateHeld` if another runner holds the state directory.
    /// - `RunnerError::Refused` if scrollback is zero, canonicalizing the state directory or durable-writer setup fails, the table is poisoned, or owner recovery fails.
    pub fn open(state_dir: &Path, scrollback: usize) -> Result<Arc<Self>, RunnerError> {
        if scrollback == 0 {
            return Err(RunnerError::refused(
                "scrollback_invalid",
                "a session's scrollback keeps at least one byte",
            ));
        }
        let mut state = StateFile::open(state_dir)?;
        let found_at = now_ms();
        let mut table = Table {
            owner: Weak::new(),
            sessions: BTreeMap::new(),
            responsible: BTreeMap::new(),
            starting: BTreeSet::new(),
            stopping: false,
            feed: Feed::open(state_dir)?,
            desk: Desk::default(),
            gaps: BTreeMap::new(),
            operations: Operations::open(state_dir)?,
        };
        let kept = state.read()?;
        table.responsible = kept.responsible;
        for kept in kept.sessions {
            let ended = if let Some(ended) = kept.ended {
                ended
            } else {
                let (signal, reason) = left_behind(&kept.session, kept.leader_start.as_ref());
                Ended {
                    how: EndedHow::EndedByRunnerRestart,
                    at: found_at,
                    status: None,
                    signal,
                    reason,
                    stopped: None,
                }
            };
            table.sessions.insert(
                kept.session,
                Session {
                    started_at: kept.started_at,
                    pid: kept.pid,
                    leader_start: kept.leader_start,
                    columns: kept.columns,
                    rows: kept.rows,
                    output: Arc::new(output::OutputHandle::new(scrollback, Some(ended.clone()))),
                    ended: Some(ended),
                    live: None,
                    generation: 0,
                    launch: None,
                    managed: None,
                    rotation: None,
                    ending: false,
                    guard: Guard::default(),
                    follower: None,
                    pending_status: None,
                    plan_windows: None,
                    stopped: None,
                    attach: crate::attach::Ring::new(scrollback),
                    last_signal: None,
                    workers: Vec::new(),
                },
            );
        }
        let state_dir = state_dir
            .canonicalize()
            .map_err(|error| RunnerError::refused("launch_config_refused", error.to_string()))?;
        let writer = crate::durable::Writer::new()?;
        state.writer(writer.clone());
        table.feed.writer(writer.clone());
        table.operations.writer(writer.clone());
        let sessions = Arc::new_cyclic(|owner| {
            table.owner = Weak::clone(owner);
            Self {
                upgrading: std::sync::atomic::AtomicBool::new(false),
                table: Mutex::new(table),
                changed: Condvar::new(),
                state,
                state_dir,
                proxy_usage: std::sync::OnceLock::new(),
                serving: std::sync::OnceLock::new(),
                server_key: std::sync::OnceLock::new(),
                owner_state: std::sync::OnceLock::new(),
                owned: Mutex::new(BTreeMap::new()),
                owner_starts: Mutex::new(BTreeSet::new()),
                owner_program: std::sync::OnceLock::new(),
                unreachable: Mutex::new(Vec::new()),
                scrollback,
                writer,
                #[cfg(test)]
                spawn_probe: Mutex::new(None),
                #[cfg(test)]
                worker_probes: workers::Probes::default(),
            }
        });
        let table = sessions.lock()?;
        sessions.persist(&table)?;
        drop(table);
        sessions.writer.barrier()?;
        // The owners a previous runner started are found and proved, never
        // restarted or replayed (AGENTS-004 R2).
        sessions.recover_owners()?;
        Ok(sessions)
    }

    /// The runner's own id, which every request it answers names.
    pub fn runner(&self) -> &str {
        self.state.runner()
    }

    fn persist(&self, table: &Table) -> Result<(), RunnerError> {
        let sessions = table
            .sessions
            .iter()
            .map(|(id, session)| KeptSession {
                session: id.clone(),
                pid: session.pid,
                leader_start: session.leader_start.clone(),
                started_at: session.started_at,
                columns: session.columns,
                rows: session.rows,
                ended: session.ended.clone(),
            })
            .collect();
        self.state
            .write_responsible(&Kept::new(sessions), &table.responsible)
    }

    /// Record the table, naming any failure where the runner's log shows it:
    /// a thread that saw an exit cannot answer anyone.
    fn persist_logged(&self, table: &Table) {
        if let Err(error) = self.persist(table) {
            crate::error::said(&format!("the runner's record was not written: {error}"));
        }
    }

    /// Start `launch` in its own pseudo-terminal, answering its process id
    /// and when it started.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if launch, policy, tracking, config, controls, terminal/process preparation or session admission fails, the table is poisoned, or durable completion fails.
    /// - `RunnerError::State` if the started session cannot be persisted.
    pub fn start(self: &Arc<Self>, launch: Launch) -> Result<(u32, u64), RunnerError> {
        self.begin(launch, None, None)
    }

    /// Start `launch` as [`Sessions::start`] does, holding `policy` for its
    /// judge and tracking its harness as `tracking` says. A tracking the
    /// runner was not measured against is refused before anything runs, and
    /// so is an executable that reports another version.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if launch, policy, tracking, config, controls, terminal/process preparation or session admission fails, the table is poisoned, or durable completion fails.
    /// - `RunnerError::State` if the started session cannot be persisted.
    pub fn begin(
        self: &Arc<Self>,
        launch: Launch,
        policy: Option<Policy>,
        tracking: Option<Tracking>,
    ) -> Result<(u32, u64), RunnerError> {
        self.begin_owned(launch, policy, tracking, None, None)
    }
}
