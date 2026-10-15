impl Connection {
    pub(crate) fn proved_with(
        &mut self,
        sessions: &Sessions,
        processes: &dyn Processes,
        (pid, uid, own): (u32, u32, u32),
    ) -> Result<&Proof, RunnerError> {
        if uid != own {
            return Err(unproved("the peer and runner have different users"));
        }
        if let Some(proof) = self.proof.as_ref() {
            if proof.pid != pid
                || proof.uid != uid
                || processes.start(pid)? != proof.start
                || processes.start(proof.leader.pid)? != proof.leader.start
                || !sessions.peer_matches(&proof.session, proof.generation, &proof.leader)?
            {
                return Err(unproved(
                    "the connection's proved process or launch changed",
                ));
            }
        } else {
            let start = processes.start(pid)?;
            let mut walked = BTreeSet::new();
            let mut at = pid;
            let (session, generation, leader) = loop {
                if let Some(found) = sessions.peer_leader(at)? {
                    break found;
                }
                if at <= 1 || !walked.insert(at) {
                    return Err(unproved("the peer descends from no running session"));
                }
                at = processes.parent(at)?;
            };
            if processes.start(leader.pid)? != leader.start
                || processes.start(leader.pid)? != leader.start
                || processes.start(pid)? != start
                || !sessions.peer_matches(&session, generation, &leader)?
            {
                return Err(unproved("the peer or its launch changed during proof"));
            }
            self.proof = Some(Proof {
                session,
                generation,
                leader,
                pid,
                uid,
                start,
            });
        }
        self.proof
            .as_ref()
            .ok_or_else(|| unproved("the connection has no process proof"))
    }

    /// Answer a parsed request without repeating its ancestry walk.
    pub fn answer(
        &mut self,
        sessions: &Arc<Sessions>,
        stream: &UnixStream,
        request: PeerRequest,
        left: &AtomicBool,
    ) -> Answer {
        if request.version != PROTOCOL_VERSION {
            return Answer::refusal(&RunnerError::ProtocolMismatch {
                theirs: request.version,
                ours: PROTOCOL_VERSION,
            });
        }
        let proved = peer_of(stream)
            .and_then(|(pid, uid)| self.proved_with(sessions, &System, (pid, uid, own_user())));
        let proof = match proved {
            Ok(proof) => proof,
            Err(error) => {
                return match request.peer {
                    PeerAct::Judge(_) => Answer::Judged {
                        verdict: crate::refusals::Verdict::denied(
                            &error.name(),
                            error.to_string(),
                            "not_attributed",
                        ),
                    },
                    PeerAct::Restart { .. } if error.name() == "peer_unproved" => {
                        Answer::refusal(&RunnerError::refused("not_a_session", error.to_string()))
                    }
                    PeerAct::Restart { .. } | PeerAct::Collect(_) => Answer::refusal(&error),
                };
            }
        };
        match request.peer {
            PeerAct::Restart { operation, .. } => sessions
                .restart_proved(&proof.session, &operation, &proof.leader)
                .map_or_else(
                    |error| Answer::refusal(&error),
                    |outcome| Answer::Operation { outcome },
                ),
            PeerAct::Judge(asked) => Answer::Judged {
                verdict: crate::refusal_log::answer_proved(
                    sessions,
                    &proof.session,
                    proof.generation,
                    &proof.leader,
                    &asked,
                    left,
                ),
            },
            PeerAct::Collect(collected) => {
                sessions.collect(&proof.session, &collected).map_or_else(
                    |error| Answer::refusal(&error),
                    |words| Answer::Collected { words },
                )
            }
        }
    }
}
