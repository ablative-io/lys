struct SeededAuthority {
    directory: Directory<FileLeafStore>,
    administrator: LoginBinding,
    actor: Actor,
    admin: PersonId,
    organisation: lys_identity::OrganisationId,
    asker: PersonId,
    holder: AgentId,
    agent: AgentId,
    now: u64,
}
fn seeded_authority(config: &crate::Config) -> TestResult<SeededAuthority> {
    let key_file = &config.event_key_file;
    std::fs::write(key_file, [7; 32])?;
    std::fs::set_permissions(key_file, std::fs::Permissions::from_mode(0o600))?;
    FileLeafStore::create(&config.log_dir, &config.log_origin)?;
    FileLeafStore::create(&config.grant_log_dir, &config.grant_log_origin)?;
    let log_dir = config.log_dir.clone();
    let mut directory = Directory::open(
        Box::new(move || FileLeafStore::open(&log_dir)),
        Ed25519Identity::load(key_file)?,
    )?;
    let administrator = LoginBinding::new(&config.issuer, "administrator")?;
    let actor = Actor::new(
        LoginBinding::new(&config.issuer, "asker")?,
        Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    let setup_actor = Actor::new(
        administrator.clone(),
        Provenance::new(AuthMethod::Oidc, crate::session::now()),
    );
    let created_at = 1_800_000_000;
    let now = created_at + 10;
    let founded = directory.found_organisation(
        setup_actor.clone(),
        OperationId::generate()?,
        identity_contract::fixtures::organisation()?,
        Profile::new("Administrator")?,
        created_at,
    )?;
    let (admin, organisation) = (founded.founder, founded.organisation);
    let (responsible, _) = directory.register_person(
        setup_actor.clone(),
        OperationId::generate()?,
        Profile::new("Responsible")?,
        organisation,
        created_at,
    )?;
    let (asker, _) = directory.register_person(
        setup_actor.clone(),
        OperationId::generate()?,
        Profile::new("Asker")?,
        organisation,
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
    Ok(SeededAuthority {
        directory,
        administrator,
        actor,
        admin,
        organisation,
        asker,
        holder,
        agent,
        now,
    })
}

fn fixture_grants(
    config: &crate::Config,
    authority: &mut SeededAuthority,
    engine: &MemoryRelationships,
    fault: &Arc<AtomicBool>,
    model: &Model,
) -> TestResult<crate::grants::GrantState> {
    let (admin, organisation, asker, holder, agent, now) = (
        authority.admin,
        authority.organisation,
        authority.asker,
        authority.holder,
        authority.agent,
        authority.now,
    );
    let directory = &mut authority.directory;
    let log_dir = config.grant_log_dir.clone();
    let mut grants = Grants::open(
        Box::new(move || FileLeafStore::open(&log_dir)),
        Ed25519Identity::load(&config.event_key_file)?,
        Relationships::faulted(engine.clone(), Arc::clone(fault)),
        model.clone(),
        admin,
    )?;
    // The founder's administrator grant, from the epoch: the service judges
    // later roots at the system clock, before this fixture's instants.
    grants.found_organisation(directory.projection()?, organisation, 0)?;
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
    Ok(grants)
}

fn fixture_state(
    path: &Path,
    config: &crate::Config,
    oidc: crate::oidc::Oidc,
    authority: SeededAuthority,
    grants: crate::grants::GrantState,
    model: Model,
) -> TestResult<AppState> {
    let SeededAuthority {
        directory,
        administrator,
        ..
    } = authority;
    let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
    let say: crate::Say = Arc::new(|_| {});
    let apps = crate::apps_api::opened(config, Arc::clone(&key), &*say)?;
    let model_revision = apps.model_revision();
    Ok(AppState {
        draft_execution: crate::drafts_apply::Executor::default(),
        stores: crate::stores_receipt::Held::default(),
        changes: crate::changes::Changes::new()?,
        directory: Mutex::new(directory),
        oidc,
        sign_in: crate::sign_in::IssuerSignIn::configured(config)?,
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
        grant_setup: fixture_grant_setup(config, model, model_revision),
        secrets: None,
        mailer: None,
        foundings: Mutex::new(crate::foundings_store::Foundings::open(
            config.log_dir.with_file_name("foundings.json"),
        )?),
        pairings: Mutex::new(crate::pair_store::PairStore::beside(&config.log_dir)?),
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
        getting_started: crate::getting_started_api::GettingStartedStore::beside(&config.log_dir),
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
        seat_imports: crate::seat_import_store::SeatImports::open(&path.join("seat-imports"), key)?,
    })
}

fn fixture_grant_setup(config: &crate::Config, model: Model, model_revision: u64) -> GrantSetup {
    GrantSetup {
        log_dir: config.grant_log_dir.clone(),
        log_origin: config.grant_log_origin.clone(),
        key_file: config.event_key_file.clone(),
        model: RwLock::new(model),
        spicedb: None,
        model_revision: AtomicU64::new(model_revision),
        refresh: Mutex::new(()),
        changes: crate::grant_changes::GrantChanges::default(),
    }
}
