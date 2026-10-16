fn wire(value: impl serde::Serialize) -> Result<serde_json::Value, ServerError> {
    serde_json::to_value(value).map_err(|error| ServerError::RequestMalformed {
        reason: format!("grant fixture answer could not be encoded: {error}"),
    })
}

fn refusal() -> GrantError {
    GrantError::PermissionEngineUnavailable {
        reason: "instance-private settlement refusal".to_owned(),
    }
}

async fn discover(
    config: &crate::Config,
    listener: tokio::net::TcpListener,
) -> TestResult<crate::oidc::Oidc> {
    let issuer = &config.issuer;
    let document = json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "jwks_uri": format!("{issuer}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["EdDSA"],
    });
    let router = axum::Router::new()
        .route(
            "/.well-known/openid-configuration",
            axum::routing::get(move || {
                let answer = document.clone();
                async move { Json(answer) }
            }),
        )
        .route(
            "/jwks",
            axum::routing::get(|| async { Json(json!({ "keys": [] })) }),
        );
    let worker = tokio::spawn(async move { axum::serve(listener, router).await });
    let result = crate::oidc::Oidc::discover(config).await;
    worker.abort();
    match worker.await {
        Ok(ended) => ended?,
        Err(error) if error.is_cancelled() => {}
        Err(error) => return Err(error.into()),
    }
    Ok(result?)
}

struct Table {
    state: Arc<AppState>,
    fault: Arc<AtomicBool>,
    engine: MemoryRelationships,
    dir: tempfile::TempDir,
    asker: PersonId,
    agent: AgentId,
    holder: AgentId,
    actor: Actor,
}

struct ReadyTable {
    state: AppState,
    fault: Arc<AtomicBool>,
    engine: MemoryRelationships,
    asker: PersonId,
    agent: AgentId,
    holder: AgentId,
    actor: Actor,
}

impl Table {
    fn projection_pending(&self) -> TestResult {
        let path = self.dir.path().join("grants");
        let administrator = self
            .state
            .admission
            .administrator_login()?
            .ok_or("administrator absent")?;
        let root = self
            .state
            .directory
            .lock()
            .map_err(|error| error.to_string())?
            .projection()?
            .person_for(&administrator)
            .ok_or("administrator unbound")?;
        let grants = Grants::open(
            Box::new(move || FileLeafStore::open(&path)),
            Ed25519Identity::load(&self.state.grant_setup.key_file)?,
            Relationships::faulted_projection(
                self.engine.clone(),
                Arc::clone(&self.fault),
                Arc::new(AtomicU64::new(0)),
            ),
            self.state.grant_setup.model()?,
            root,
        )?;
        *self
            .state
            .grants
            .lock()
            .map_err(|error| error.to_string())? = Some(grants);
        self.fault.store(true, Ordering::Release);
        crate::grants::with_grants(&self.state, |judged| {
            let request = RootRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(judged.root),
                route: Route::Api,
                holder: self.asker,
                resource: Resource::new("agent", &self.holder.to_string())?,
                relation: Relation::new("reader")?,
                pass_on: PassOn::UseOnly,
                window: Window::new(0, None)?,
            };
            match judged
                .grants
                .issue_root(judged.directory, &request, crate::session::now())
            {
                Err(GrantError::ProjectionPending { .. }) => Ok(()),
                Err(error) => Err(error.into()),
                Ok(_) => Err(ServerError::RequestMalformed {
                    reason: "pending projection was acknowledged".to_owned(),
                }),
            }
        })?;
        Ok(())
    }

    async fn fresh() -> TestResult<Self> {
        let dir = tempfile::TempDir::new()?;
        match Self::prepare(dir.path()).await {
            Ok(ready) => Ok(Self {
                state: Arc::new(ready.state),
                fault: ready.fault,
                engine: ready.engine,
                dir,
                asker: ready.asker,
                agent: ready.agent,
                holder: ready.holder,
                actor: ready.actor,
            }),
            Err(error) => match dir.close() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!(
                    "fixture construction failed: {error}; fixture cleanup failed: {cleanup}"
                )
                .into()),
            },
        }
    }

    async fn prepare(path: &Path) -> TestResult<ReadyTable> {
        let fault = Arc::new(AtomicBool::new(false));
        let engine = MemoryRelationships::default();
        let key_file = path.join("service.key");
        let model_file = path.join("model.json");
        std::fs::write(
            &model_file,
            br#"{"version":1,"relations":{"operator":["read","operate"],"reader":["read"]}}"#,
        )?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("http://{}", listener.local_addr()?);
        let config: crate::Config = serde_json::from_value(json!({
            "listen": "127.0.0.1:0", "log_dir": path.join("log"),
            "log_origin": "example.test/lys/directory", "event_key_file": key_file,
            "issuer": issuer, "client_id": "settlement-test", "client_secret_file": path.join("client-secret"),
            "redirect_url": "http://127.0.0.1/callback", "session_seconds": 300, "secure_cookie": false,
            "link_audit_source": {"issuer": issuer, "subject": "audit"},
            "grant_log_dir": path.join("grants"), "grant_log_origin": "example.test/lys/grants",
            "grant_model_file": model_file,
        }))?;
        std::fs::write(&config.client_secret_file, b"synthetic-test-credential")?;
        let oidc = discover(&config, listener).await?;
        std::fs::write(&key_file, [7; 32])?;
        std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o600))?;
        FileLeafStore::create(&config.log_dir, &config.log_origin)?;
        FileLeafStore::create(&config.grant_log_dir, &config.grant_log_origin)?;
        let log_dir = config.log_dir.clone();
        let mut directory = Directory::open(
            Box::new(move || FileLeafStore::open(&log_dir)),
            Ed25519Identity::load(&key_file)?,
        )?;
        let administrator = LoginBinding::new(&issuer, "administrator")?;
        let actor = Actor::new(
            LoginBinding::new(&issuer, "asker")?,
            Provenance::new(AuthMethod::Oidc, crate::session::now()),
        );
        let setup_actor = Actor::new(
            administrator.clone(),
            Provenance::new(AuthMethod::Oidc, crate::session::now()),
        );
        let created_at = 1_800_000_000;
        let now = created_at + 10;
        let (admin, _) = directory.register_person(
            setup_actor.clone(),
            OperationId::generate()?,
            Profile::new("Administrator")?,
            created_at,
        )?;
        let (responsible, _) = directory.register_person(
            setup_actor.clone(),
            OperationId::generate()?,
            Profile::new("Responsible")?,
            created_at,
        )?;
        let (asker, _) = directory.register_person(
            setup_actor.clone(),
            OperationId::generate()?,
            Profile::new("Asker")?,
            created_at,
        )?;
        let (holder, _) = directory.register_agent(
            setup_actor.clone(),
            OperationId::generate()?,
            asker,
            Profile::new("Holder")?,
            created_at,
        )?;
        let (agent, _) = directory.register_agent(
            setup_actor.clone(),
            OperationId::generate()?,
            responsible,
            Profile::new("Target")?,
            created_at,
        )?;
        for identity in [admin, responsible, asker]
            .map(IdentityId::Person)
            .into_iter()
            .chain([holder, agent].map(IdentityId::Agent))
        {
            directory.transition(
                setup_actor.clone(),
                OperationId::generate()?,
                identity,
                Transition::Activate,
                "",
                created_at,
            )?;
        }
        directory.bind_login(
            setup_actor.clone(),
            OperationId::generate()?,
            admin,
            administrator.clone(),
            now,
        )?;
        directory.bind_login(
            setup_actor,
            OperationId::generate()?,
            asker,
            actor.binding().clone(),
            now,
        )?;
        let model: Model = config.grant_model()?;
        let log_dir = path.join("grants");
        let mut grants = Grants::open(
            Box::new(move || FileLeafStore::open(&log_dir)),
            Ed25519Identity::load(&key_file)?,
            Relationships::faulted(engine.clone(), Arc::clone(&fault)),
            model.clone(),
            admin,
        )?;
        let resource = Resource::new("agent", &agent.to_string())?;
        let pass_on = PassOn::to(
            [Action::new("read")?].into_iter().collect(),
            [RecipientKind::Agent].into_iter().collect(),
        )?;
        let root = grants
            .issue_root(
                directory.projection()?,
                &RootRequest {
                    operation: OperationId::generate()?,
                    caller: IdentityId::Person(admin),
                    route: Route::Api,
                    holder: asker,
                    resource: resource.clone(),
                    relation: Relation::new("operator")?,
                    pass_on,
                    window: Window::new(0, None)?,
                },
                now,
            )?
            .event
            .grant();
        grants.delegate(
            directory.projection()?,
            &DelegateRequest {
                operation: OperationId::generate()?,
                caller: IdentityId::Person(asker),
                route: Route::Api,
                source: root,
                recipient: IdentityId::Agent(holder),
                responsible: asker,
                resource,
                relation: Relation::new("reader")?,
                pass_on: PassOn::UseOnly,
                window: Window::new(0, None)?,
            },
            now,
        )?;
        let key = Arc::new(Ed25519Identity::load(&key_file)?);
        let say: crate::Say = Arc::new(|_| {});
        let apps = crate::apps_api::opened(&config, Arc::clone(&key), &*say)?;
        let model_revision = apps.model_revision();
        let state = AppState {
        draft_execution:Default::default(),
            changes: crate::changes::Changes::new()?,
            directory: Mutex::new(directory),
            oidc,
            sign_in: crate::sign_in::IssuerSignIn::configured(&config)?,
            provider: None,
            setup: None,
            setup_lock: tokio::sync::Mutex::new(()),
            import_credential_file: None,
            identity_upstream: "http://127.0.0.1".to_owned(),
            estate_plan_file: path.join("estate.json"),
            password_policy: None,
            model_proxy: None,
            proxy_dir: None,
            dialled_model_proxy: None,
            sessions: crate::session::Sessions::new(300, false),
            admission: crate::admission::Admission::new(
                Some(administrator),
                config.link_audit_binding()?,
            ),
            grants: Mutex::new(Some(grants)),
            rights_cache: Mutex::default(),
            grant_setup: GrantSetup {
                log_dir: config.grant_log_dir.clone(),
                log_origin: config.grant_log_origin.clone(),
                key_file,
                model: RwLock::new(model),
                spicedb: None,
                model_revision: AtomicU64::new(model_revision),
                refresh: Mutex::new(()),
                changes: crate::grant_changes::GrantChanges::default(),
            },
            secrets: None,
            requests: None,
            mcp_requests: None,
            network: None,
            joins: None,
            roles: None,
            provisioning: None,
            certificates: None,
            runtime: None,
            service_accounts: None,
            reviews: None,
            teams: None,
            budgets: None,
            policies: None,
            stops: None,
            goals: None,
            words: None,
            variables: None,
            schedules: None,
            configuration: Mutex::new(Box::new(
                crate::configuration_store::ConfigurationStore::open(
                    &path.join("organisation"),
                    Arc::clone(&key),
                )?,
            )),
            cord: Mutex::new(crate::cord_store::CordStore::open(
                &path.join("cord"),
                Arc::clone(&key),
            )?),
            canvas: crate::canvas_store::CanvasStore::beside(&config.log_dir),
            apps: Mutex::new(apps),
            grant_tokens: Mutex::new(crate::grant_token_store::Tokens::open(
                path.join("grant-tokens.json"),
            )?),
            agent_passes: Arc::new(Mutex::new(crate::agent_pass_store::Passes::open(
                path.join("agent-passes.json"),
            )?)),
            kept_responsibilities: crate::kept_responsibilities::Kept::load(
                &path.join("kept-responsibilities.json"),
            )?,
            benches: crate::apps_bench::Benches::new(path.join("benches"), &*say)?,
            sign_in_providers: None,
            agent_nonces: Mutex::default(),
            operator_token: None,
            operator_upgrade_file: None,
            runners: crate::runner_client::Runners::new(Arc::clone(&key), None),
            acts: Mutex::new(crate::runner_acts::ActStore::open(
                &path.join("runner-acts"),
                Arc::clone(&key),
            )?),
            say,
            membership: crate::channel_membership_counts::Membership::new(None),
            seats: Mutex::new(crate::seats_store::SeatStore::open(
                &path.join("seats"),
                Arc::clone(&key),
            )?),
            seat_imports: crate::seat_import_store::SeatImports::open(
                &path.join("seat-imports"),
                key,
            )?,
        };
        Ok(ReadyTable {
            state,
            fault,
            engine,
            asker,
            agent,
            holder,
            actor,
        })
    }

    fn count(&self) -> TestResult<u64> {
        Ok(FileLeafStore::open(&self.dir.path().join("grants"))?.extent())
    }

    fn headers(&self) -> TestResult<HeaderMap> {
        let cookie = self.state.sessions.begin(self.actor.clone())?;
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            cookie
                .split(';')
                .next()
                .ok_or("session cookie absent")?
                .parse()?,
        );
        Ok(headers)
    }

    fn close(self) -> TestResult {
        self.fault.store(false, Ordering::Release);
        drop(self.state);
        self.dir.close()?;
        Ok(())
    }

    fn finish<T>(self, result: TestResult<T>) -> TestResult<T> {
        match (result, self.close()) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(cleanup)) => Err(format!(
                "fixture act failed: {error}; fixture cleanup failed: {cleanup}"
            )
            .into()),
        }
    }

    fn goal(&self) -> Item {
        Item {
            goal: Goal {
                id: "judgement".to_owned(),
                holder: Holder {
                    kind: HolderKind::Agent,
                    id: self.agent.to_string(),
                },
                kind: Kind::Goal,
                words: "Read the decision".to_owned(),
                deadline: None,
                active: true,
                evidence: None,
                judged_by: Some("read".to_owned()),
                reminders: Vec::new(),
                responsible: IdentityId::Agent(self.agent).to_string(),
                set_by: self.asker.to_string(),
                at: 0,
            },
            standing: Standing::Open,
            marked: None,
            timers: Vec::new(),
            fired: Vec::new(),
            changes: Vec::new(),
        }
    }
}
