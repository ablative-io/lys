//! Broker-held keys sign one exact request; refusal prevents an upstream connection.

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use identity_contract::fake_issuer::Login;
    use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
    use lys_core::Ed25519Identity;
    use lys_core::ca::create_certificate_request;
    use lys_identity::{
        Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
    };
    use lys_identity_server::certificates_store::{CertificateStore, Issued};
    use lys_identity_server::routes::open_directory;
    use lys_runner::dial::agent::{AgentClient, AgentLease, LeaseProof, Proof};
    use lys_secrets::{
        Broker, BrokerPaths, Checked, EntryClass, Holder, IssuedHandle, LocalGrants, Presentation,
        Secret, SecretRelation, Used, new_operation_id, request_digest, to_hex,
    };
    use serde_json::json;
    use std::error::Error;
    use std::sync::{Arc, Mutex};
    type TestResult<T = ()> = Result<T, Box<dyn Error>>;
    const BEA: &str = "bea-subject";
    fn operation() -> TestResult<String> {
        Ok(OperationId::generate()?.to_string())
    }
    fn login(subject: &str) -> Login {
        Login {
            subject: subject.to_owned(),
            email: "shared@example.test".to_owned(),
        }
    }
    struct Table {
        service: Service,
        agent: String,
        machine: String,
    }
    impl Table {
        async fn set() -> Result<Self, Box<dyn Error>> {
            Self::set_certificate(None).await
        }

        async fn set_certificate(der: Option<&'static [u8]>) -> Result<Self, Box<dyn Error>> {
            let serial = operation()?;
            let entered_serial = serial.clone();
            let (service, (agent, key)) = Service::start_adjusted(
                GRANT_MODEL,
                None,
                None,
                None,
                |config| {
                    config.requests_dir = None;
                    config.roles_file = None;
                    config.provisioning_file = None;
                    config.service_accounts_dir = None;
                    config.teams_dir = None;
                    config.stops_dir = None;
                    config.budgets_dir = None;
                    config.policies_dir = None;
                    config.reviews_dir = None;
                },
                move |config| {
                    let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                    let actor = Actor::new(
                        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                        Provenance::new(AuthMethod::Oidc, 1),
                    );
                    let mut directory = open_directory(config)?;
                    let (person, _) = directory.setup_person(
                        actor.clone(),
                        OperationId::generate()?,
                        Profile::new("Owner")?,
                        1,
                    )?;
                    directory.bind_login(
                        actor.clone(),
                        OperationId::generate()?,
                        person,
                        LoginBinding::new(&config.issuer, BEA)?,
                        1,
                    )?;
                    let (agent, _) = directory.register_agent(
                        actor.clone(),
                        OperationId::generate()?,
                        person,
                        Profile::new("Agent")?,
                        2,
                    )?;
                    directory.transition(
                        actor,
                        OperationId::generate()?,
                        IdentityId::Agent(agent),
                        Transition::Activate,
                        "",
                        3,
                    )?;
                    drop(lys_identity_server::runtime_store::RuntimeStore::open(
                        config
                            .runtime_dir
                            .as_deref()
                            .ok_or("runtime directory missing")?,
                        Arc::clone(&key),
                    )?);
                    drop(lys_identity_server::goals_store::GoalStore::open(
                        config
                            .goals_dir
                            .as_deref()
                            .ok_or("goals directory missing")?,
                        Arc::clone(&key),
                    )?);
                    let mut certificates = CertificateStore::open(
                        config
                            .certificates_dir
                            .as_deref()
                            .ok_or("certificate directory missing")?,
                        Arc::clone(&key),
                    )?;
                    if let Some(der) = der {
                        certificates.issue(Issued {
                            serial: entered_serial,
                            agent: agent.to_string(),
                            person: person.to_string(),
                            claims: json!({}),
                            der: STANDARD.encode(der),
                            issued_at: 0,
                        })?;
                    }
                    drop(
                        lys_identity_server::configuration_store::ConfigurationStore::open(
                            &config.log_dir.with_file_name("organisation"),
                            Arc::clone(&key),
                        )?,
                    );
                    drop(lys_identity_server::runner_acts::ActStore::open(
                        &config.log_dir.with_file_name("runner-acts"),
                        Arc::clone(&key),
                    )?);
                    drop(lys_identity::start::LaunchRecords::open(
                        &config.log_dir.with_file_name("launch-records"),
                        Ed25519Identity::load(&config.event_key_file)?,
                    )?);
                    drop(lys_identity_server::apps_api::opened(
                        config,
                        Arc::clone(&key),
                        &|_| {},
                    )?);
                    Ok((agent.to_string(), key))
                },
            )
            .await?;
            let ada = service.sign_in(login(ADMINISTRATOR)).await?;
            let bea = service.sign_in(login(BEA)).await?;
            let machine = operation()?;
            let body = json!({
                "operation": machine, "name": "Laptop 2", "kind": "laptop", "runtime": "local launcher",
                "slots": 2, "may_run": [agent], "may_reach": [],
            });
            let (status, named) = service.post("/network/machines", Some(&ada), &body).await?;
            assert_eq!(status, 200, "{named}");
            if der.is_none() {
                let body = json!({
                    "operation": serial,
                    "request": STANDARD.encode(create_certificate_request(&key, &agent)?),
                });
                let (status, issued) = service
                    .post(&format!("/agents/{agent}/certificates"), Some(&bea), &body)
                    .await?;
                assert_eq!(status, 200, "{issued}");
            }
            Ok(Self {
                service,
                agent,
                machine,
            })
        }
    }

    struct Lease {
        broker: Mutex<Broker<LocalGrants>>,
        issued: IssuedHandle,
        holder: Ed25519Identity,
        root: tempfile::TempDir,
    }
    impl LeaseProof for Lease {
        fn present(&self, payload: &[u8]) -> Result<Proof, lys_runner::error::RunnerError> {
            let signed_at = now();
            let digest = request_digest("SIGN", "agent-key", payload)
                .unwrap_or_else(|error| panic!("fixture_fixture_digest: {error:?}"));
            let presentation = Presentation::sign(
                &self.issued.id,
                &new_operation_id().unwrap_or_else(|error| panic!("fixture_operation: {error:?}")),
                signed_at,
                digest,
                &self.holder,
            )
            .unwrap_or_else(|error| panic!("fixture_presentation: {error:?}"));
            Ok(Proof {
                token: to_hex(self.issued.token.expose()),
                fields: presentation.to_wire(),
            })
        }
    }
    fn now() -> i64 {
        i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|error| panic!("fixture_clock: {error:?}"))
                .as_millis(),
        )
        .unwrap_or_else(|error| panic!("fixture_milliseconds: {error:?}"))
    }
    fn lease() -> TestResult<Arc<Lease>> {
        let root = tempfile::tempdir()?;
        let paths = BrokerPaths {
            store_dir: root.path().join("store"),
            log_dir: root.path().join("log"),
            store_key: root.path().join("store.key"),
            audit_key: root.path().join("audit.key"),
            anchor: root.path().join("anchor"),
        };
        let grants = LocalGrants::new();
        grants.grant(SecretRelation {
            identity: "agent:holder".to_owned(),
            secret: "agent-key".to_owned(),
            granted_by: Some("person:owner".to_owned()),
        })?;
        let mut broker = Broker::create(&paths, grants, Box::new(now))?;
        broker.seal_once(
            "agent-key",
            EntryClass::Key,
            "person:owner",
            &Secret::from_slice(&[9; 32]),
        )?;
        let holder = Ed25519Identity::ephemeral();
        let issued = broker.issue(
            &Holder {
                identity: "agent:holder".to_owned(),
                key: holder.public_key_bytes(),
            },
            "agent-key",
            10,
            now() + 600_000,
        )?;
        Ok(Arc::new(Lease {
            broker: Mutex::new(broker),
            issued,
            holder,
            root,
        }))
    }
    async fn signing(
        axum::extract::State(lease): axum::extract::State<Arc<Lease>>,
        headers: axum::http::HeaderMap,
        body: axum::body::Bytes,
    ) -> (axum::http::StatusCode, Vec<u8>) {
        let header = |name| {
            headers
                .get(name)
                .unwrap_or_else(|| panic!("fixture_header_missing"))
                .to_str()
                .unwrap_or_else(|error| panic!("fixture_header_text: {error:?}"))
        };
        let presentation = Presentation::from_wire(
            header("lys-handle-id"),
            header("lys-operation"),
            header("lys-signed-at"),
            Some(header("lys-presentation")),
            request_digest("SIGN", "agent-key", &body)
                .unwrap_or_else(|error| panic!("fixture_digest: {error:?}")),
        )
        .unwrap_or_else(|error| panic!("fixture_presentation: {error:?}"));
        let token = lys_secrets::HandleToken::from_bytes(
            &lys_secrets::from_hex(header("lys-handle"))
                .unwrap_or_else(|| panic!("fixture_token_invalid")),
        );
        let mut broker = lease
            .broker
            .lock()
            .unwrap_or_else(|error| panic!("fixture_broker: {error:?}"));
        let checked = Checked::answer(&broker.asks_for(&token), broker.permissions());
        match broker.sign_use_for_checked(&token, &presentation, "agent-key", &body, 0, &checked) {
            Ok(Used::Forwarded { answer, .. }) => {
                (axum::http::StatusCode::OK, answer.as_bytes().to_vec())
            }
            Ok(Used::Retried { outcome }) => (
                axum::http::StatusCode::CONFLICT,
                json!({"refusal":"SigningAlreadyPresented","outcome":outcome})
                    .to_string()
                    .into_bytes(),
            ),
            Err(error) => (
                axum::http::StatusCode::FORBIDDEN,
                json!({"refusal":error.name(),"reason":error.to_string()})
                    .to_string()
                    .into_bytes(),
            ),
        }
    }
    async fn broker_server(
        lease: Arc<Lease>,
    ) -> TestResult<(String, tokio::task::JoinHandle<std::io::Result<()>>)> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let app = axum::Router::new()
            .route("/_lys/sign/{secret}", axum::routing::post(signing))
            .with_state(lease);
        let task = tokio::spawn(async move { axum::serve(listener, app).await });
        Ok((url, task))
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn broker_signed_request_passes_the_real_verifier_and_tampering_fails() -> TestResult {
        let table = Table::set().await?;
        let lease = lease()?;
        let (broker, task) = broker_server(Arc::clone(&lease)).await?;
        let agent = table.agent.clone();
        let path = format!("/agents/{agent}/runtime/sessions/one/reports?scope=exact");
        let body=json!({"operation":operation()?,"state":"starting","machine":table.machine,"what":"request"}).to_string().into_bytes();
        let client = AgentClient::new(&table.service.base, &broker, None)?;
        let (answer, header) = tokio::task::spawn_blocking(move || {
            let header = client.sign(&agent, "agent-key", lease.as_ref(), "POST", &path, &body)?;
            let answer = client.send_signed("POST", &path, &[], &body, &header)?;
            Ok::<_, lys_runner::error::RunnerError>((answer, (path, body, header)))
        })
        .await??;
        assert_eq!(
            answer.status,
            200,
            "{}",
            String::from_utf8_lossy(&answer.body)
        );
        let (path, mut body, header) = header;
        body.push(b' ');
        let (status, value) = table
            .service
            .post_signed(&path, ("lys-agent-signature", &header), body)
            .await?;
        assert_eq!(status, 401, "{value}");
        assert_eq!(value["refusal"], "AgentSignatureRefused");
        assert!(value.to_string().contains("does not verify"));
        task.abort();
        assert!(task.await.expect_err("aborted").is_cancelled());
        Ok(())
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn revoked_lease_refuses_by_name_without_an_upstream_send() -> TestResult {
        let lease = lease()?;
        lease
            .broker
            .lock()
            .unwrap_or_else(|error| panic!("fixture_broker: {error:?}"))
            .drop_handle(&lease.issued.id)?;
        let (broker, task) = broker_server(Arc::clone(&lease)).await?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let server = format!("http://{}", listener.local_addr()?);
        let error = tokio::task::spawn_blocking(move || {
            AgentClient::new(&server, &broker, None)?.send(
                &AgentLease {
                    agent: "agent.test",
                    secret: "agent-key",
                    proof: lease.as_ref(),
                },
                "POST",
                "/target",
                &[],
                b"{}",
            )
        })
        .await?
        .expect_err("revoked");
        assert_eq!(error.name(), "HandleDropped");
        assert_eq!(
            listener.accept().expect_err("nothing sent").kind(),
            std::io::ErrorKind::WouldBlock
        );
        task.abort();
        assert!(task.await.expect_err("aborted").is_cancelled());
        Ok(())
    }
    #[test]
    fn cookie_is_refused_before_signing_or_sending() -> TestResult {
        let lease = lease()?;
        assert!(lease.root.path().exists());
        let client = AgentClient::new("http://127.0.0.1:9", "http://127.0.0.1:9", None)?;
        let error = client
            .send(
                &AgentLease {
                    agent: "agent.test",
                    secret: "agent-key",
                    proof: lease.as_ref(),
                },
                "POST",
                "/target",
                &[("Cookie", "administrator=fixture")],
                b"{}",
            )
            .expect_err("mixed authority");
        assert_eq!(error.name(), "AgentCookieRefused");
        Ok(())
    }
}
