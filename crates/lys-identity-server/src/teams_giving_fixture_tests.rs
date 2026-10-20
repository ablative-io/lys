fn seeded_team(
    config: &identity_contract::lys_identity_server::Config,
    separate_member: bool,
) -> TestResult<(String, String, String, Arc<Ed25519Identity>)> {
    let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
    crate::routes::people_giving_tests::prepare_stores(
        &config.log_dir,
        &config.event_key_file,
        &config.apps_dir(),
        config.certificates_dir.as_deref(),
        config.teams_dir.as_deref(),
        &key,
    )?;
    FileLeafStore::create(&config.log_dir, &config.log_origin)?;
    let path = config.log_dir.clone();
    let mut directory = Directory::open(
        Box::new(move || FileLeafStore::open(&path)),
        Ed25519Identity::load(&config.event_key_file)?,
    )?;
    let actor = Actor::new(
        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let (person, _) = directory.setup_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Owner")?,
        lys_identity::Profile::new("Example organisation")?,
    1,
    )?;
    let (giver, _) = directory.register_agent(
        actor.clone(),
        OperationId::generate()?,
        person,
        Profile::new("Giver")?,
        2,
    )?;
    let member = if separate_member {
        directory
            .register_agent(
                actor.clone(),
                OperationId::generate()?,
                person,
                Profile::new("Member")?,
                2,
            )?
            .0
    } else {
        giver
    };
    let mut agents = vec![giver];
    if separate_member {
        agents.push(member);
    }
    for agent in agents {
        directory.transition(
            actor.clone(),
            OperationId::generate()?,
            IdentityId::Agent(agent),
            Transition::Activate,
            "",
            3,
        )?;
    }
    Ok((
        giver.to_string(),
        member.to_string(),
        person.to_string(),
        key,
    ))
}

fn member_directory() -> TestResult<(
    Directory<crate::apps_bench_scratch::memory::MemoryStore>,
    LoginBinding,
    lys_identity::PersonId,
    lys_identity::AgentId,
)> {
    use crate::apps_bench_scratch::memory::MemoryStore;
    let directory_leaves = MemoryStore::empty();
    let mut directory = Directory::open(
        Box::new(move || MemoryStore::open(Arc::clone(&directory_leaves), "directory")),
        Ed25519Identity::ephemeral(),
    )?;
    let binding = LoginBinding::new("https://issuer.test", "owner")?;
    let owner = Actor::new(binding.clone(), Provenance::new(AuthMethod::Oidc, 1));
    let (person, _) = directory.setup_person(
        owner.clone(),
        OperationId::generate()?,
        Profile::new("Owner")?,
        lys_identity::Profile::new("Example organisation")?,
    1,
    )?;
    let (agent, _) = directory.register_agent(
        owner.clone(),
        OperationId::generate()?,
        person,
        Profile::new("Caller")?,
        2,
    )?;
    directory.transition(
        owner,
        OperationId::generate()?,
        IdentityId::Agent(agent),
        Transition::Activate,
        "",
        3,
    )?;
    Ok((directory, binding, person, agent))
}

fn give_member(
    grants: &mut lys_identity::grants::Grants<
        crate::apps_bench_scratch::memory::MemoryStore,
        crate::spicedb::Relationships,
    >,
    projection: &lys_identity::projection::Projection,
    identities: (lys_identity::PersonId, lys_identity::AgentId),
    resource: lys_identity::grants::Resource,
    relation: &str,
    action: &str,
) -> TestResult<lys_identity::grants::GrantId> {
    use lys_identity::grants::{
        Action, DelegateRequest, GrantChange, PassOn, RecipientKind, Relation, RootRequest, Route,
        Window,
    };
    let (person, agent) = identities;
    let root = grants.issue_root(
        projection,
        &RootRequest {
            operation: OperationId::generate()?,
            caller: IdentityId::Person(person),
            route: Route::Api,
            holder: person,
            resource: resource.clone(),
            relation: Relation::new(relation)?,
            pass_on: PassOn::To {
                actions: [Action::new(action)?].into(),
                recipients: [RecipientKind::Agent].into(),
            },
            window: Window::new(0, None)?,
        },
        4,
    )?;
    let GrantChange::Issue(root) = root.event.change() else {
        return Err("root grant was not issued".into());
    };
    let given = grants.delegate(
        projection,
        &DelegateRequest {
            operation: OperationId::generate()?,
            caller: IdentityId::Person(person),
            route: Route::Api,
            source: root.id(),
            recipient: IdentityId::Agent(agent),
            responsible: person,
            resource,
            relation: Relation::new(relation)?,
            pass_on: PassOn::UseOnly,
            window: Window::new(0, None)?,
        },
        4,
    )?;
    let GrantChange::Issue(given) = given.event.change() else {
        return Err("member grant was not issued".into());
    };
    Ok(given.id())
}
