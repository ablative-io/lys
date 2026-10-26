//! Startup opens the configured stores before constructing the served router.

use std::sync::{Arc, Mutex};

use axum::Router;
use lys_identity::signer::load_service_key;

use crate::admission::Admission;
use crate::config::Config;
use crate::error::ServerError;
use crate::grants::GrantSetup;
use crate::oidc::Oidc;
use crate::reviews_store::ReviewStore;
use crate::routes::{AppState, Say, Shared, open_directory, start};
use crate::service_accounts_store::ServiceAccountStore;
use crate::session::Sessions;

#[path = "issuer_admin.rs"]
mod issuer_admin;

/// Every store is compared with its receipt before anything is opened
/// (DIRECTORY-107 R2): a lost whole store refuses the boot, and the service
/// serves its health and the sign-in page alone (`boot_refused`); a lost own
/// store is left out of what is opened.
pub(crate) async fn started_with_clock(
    config: &Config,
    say: Say,
    clock: lys_core::clock::ClockSource,
) -> Result<crate::stores_receipt::Started, ServerError> {
    config.reviews.validate()?;
    let mut checked = crate::stores_receipt::Checked::before_open(config, &say)?;
    if !checked.refused.is_empty() {
        return crate::boot_refused::refused(config, checked.refused);
    }
    let config = &checked.opened(config, &say)?;
    let primary = open_primary(config, &say, &mut checked)?;
    let catalogue = Arc::clone(&primary.catalogue);
    let auxiliary = open_auxiliary(config, &primary.key, &say)?;
    let state = Box::pin(make_state(config, say, clock, primary, auxiliary, &checked)).await?;
    recover(&state)?;
    // Every person's sign-in account is judged and repaired before anything
    // is served (DIRECTORY-124 R2); an unreachable issuer is said, not fatal.
    crate::sign_in_reconcile::at_start(&state).await;
    let at_stop = crate::stores_receipt::after_boot(config, &state, checked)?;
    Ok(crate::stores_receipt::Started {
        router: served_routes(config, state, catalogue)?,
        at_stop: Some(at_stop),
        refused: None,
    })
}

struct Primary {
    kept_responsibilities: crate::kept_responsibilities::Kept,
    catalogue: Arc<crate::harness_catalogue::Catalogue>,
    operator_token: Option<zeroize::Zeroizing<String>>,
    directory: lys_identity::Directory<lys_log_store::FileLeafStore>,
    key: Arc<lys_core::Ed25519Identity>,
    configuration: Box<crate::configuration_store::ConfigurationStore>,
    requests: Option<crate::requests_store::RequestStore>,
    mcp_requests: Option<Box<Mutex<crate::mcp_requests_store::McpRequestStore>>>,
    certificates: Option<crate::certificates_store::CertificateStore>,
    network: Option<crate::network_store::NetworkStore>,
    roles: Option<crate::roles_store::RolesStore>,
    provisioning: Option<crate::provisioning_store::ProvisioningStore>,
    runtime: Option<crate::runtime_store::RuntimeStore>,
    service_accounts: Option<ServiceAccountStore>,
    reviews: Option<ReviewStore>,
    teams: Option<crate::teams_store::TeamStore>,
    stops: Option<crate::stops_store::StopStore>,
    budgets: Option<crate::budgets_store::BudgetStore>,
    policies: Option<crate::agent_policy_store::PolicyStore>,
}
struct Auxiliary {
    goals: Option<crate::goals_store::GoalStore>,
    words: Option<crate::words_store::WordsKept>,
    variables: Option<crate::variables_store::VariablesKept>,
    schedules: Option<crate::schedules_store::SchedulesKept>,
    cord: crate::cord_store::CordStore,
    acts: crate::runner_acts::ActStore,
    seats: crate::seats_store::SeatStore,
    seat_imports: crate::seat_import_store::SeatImports,
    apps: crate::apps_store::AppStore,
}

fn open_primary(
    config: &Config,
    say: &Say,
    checked: &mut crate::stores_receipt::Checked,
) -> Result<Primary, ServerError> {
    let kept_responsibilities = crate::kept_responsibilities::Kept::load(
        &config.log_dir.with_file_name("kept-responsibilities.json"),
    )?;
    crate::openapi::prepare()?;
    crate::openapi::surface::prepare()?;
    let catalogue = Arc::new(crate::harness_catalogue::Catalogue::embedded()?);
    let operator_token = crate::operator::token(config, &**say)?;
    let mut directory = open_directory(config)?;
    say(&format!("directory log {}", directory.log()?.start()));
    let key = Arc::new(load_service_key(&config.event_key_file)?);
    let configuration = Box::new(crate::configuration_store::ConfigurationStore::open(
        &config.log_dir.with_file_name("organisation"),
        Arc::clone(&key),
    )?);
    say(&format!(
        "organisation configuration log {}",
        configuration.start()
    ));
    let requests = crate::requests_store::RequestStore::opened(config, Arc::clone(&key), &**say)?;
    let mcp_requests =
        crate::mcp_requests_store::McpRequestStore::configured(config, Arc::clone(&key), say)?
            .map(|store| Box::new(Mutex::new(store)));
    let certificates = config
        .certificates_dir
        .as_deref()
        .map(|dir| {
            crate::certificates_store::CertificateStore::counted(
                dir,
                Arc::clone(&key),
                checked.counts("certificates"),
                &**say,
            )
        })
        .transpose()?;
    let (network, roles, provisioning) =
        checked.file_stores(crate::file_stores::opened(config, &**say)?, say);
    let mut runtime = crate::runtime_store::RuntimeStore::configured(config, say)?;
    let service_accounts = ServiceAccountStore::configured(config, Arc::clone(&key), say)?;
    let reviews = ReviewStore::configured(config, Arc::clone(&key), &**say)?;
    let teams = crate::teams_store::TeamStore::configured(config, Arc::clone(&key), say)?;
    let stops = crate::stops_store::StopStore::configured(config, Arc::clone(&key), say)?;
    let mut budgets = crate::budgets_store::BudgetStore::configured(config, Arc::clone(&key), say)?;
    if let (Some(runtime), Some(budgets)) = (runtime.as_mut(), budgets.as_mut()) {
        let agents = budgets.held().context_availability.agents();
        let sessions = runtime.agents_with_sessions(&agents)?;
        budgets.reconcile_context(Some(&sessions))?;
    }
    let mut policies =
        crate::agent_policy_store::PolicyStore::configured(config, Arc::clone(&key), say)?;
    if let Some(store) = policies.as_mut() {
        for (id, _) in directory.projection()?.records() {
            if let lys_identity::IdentityId::Agent(agent) = id {
                store.ensure_default(&agent.to_string())?;
            }
        }
    }
    Ok(Primary {
        kept_responsibilities,
        catalogue,
        operator_token,
        directory,
        key,
        configuration,
        requests,
        mcp_requests,
        certificates,
        network,
        roles,
        provisioning,
        runtime,
        service_accounts,
        reviews,
        teams,
        stops,
        budgets,
        policies,
    })
}

fn open_auxiliary(
    config: &Config,
    key: &Arc<lys_core::Ed25519Identity>,
    say: &Say,
) -> Result<Auxiliary, ServerError> {
    let goals = crate::goals_store::GoalStore::configured(config, Arc::clone(key), say)?;
    let words = crate::words_store::configured(config, Arc::clone(key), say)?;
    let variables = crate::variables_store::configured(config, Arc::clone(key), say)?;
    let schedules = crate::schedules_store::configured(config, Arc::clone(key), say)?;
    let cord = crate::cord_store::CordStore::open(
        &config.log_dir.with_file_name("cord"),
        Arc::clone(key),
    )?;
    say(&format!("cord log {}", cord.start()));
    let acts = crate::runner_acts::ActStore::open(
        &config.log_dir.with_file_name("runner-acts"),
        Arc::clone(key),
    )?;
    say(&format!(
        "runner-acts log {}, holding {} acts",
        acts.start(),
        acts.len()
    ));
    let seats = crate::seats_store::SeatStore::open(
        &config.log_dir.with_file_name("seats"),
        Arc::clone(key),
    )?;
    say(&format!(
        "seats log {}, holding {} seats",
        seats.start(),
        seats.len()
    ));
    let seat_imports = crate::seat_import_store::SeatImports::open(
        &config.log_dir.with_file_name("seat-imports"),
        Arc::clone(key),
    )?;
    say(&seat_imports.started()?);
    let apps = crate::apps_api::opened(config, Arc::clone(key), &**say)?;
    Ok(Auxiliary {
        goals,
        words,
        variables,
        schedules,
        cord,
        acts,
        seats,
        seat_imports,
        apps,
    })
}

async fn make_state(
    config: &Config,
    say: Say,
    clock: lys_core::clock::ClockSource,
    primary: Primary,
    auxiliary: Auxiliary,
    checked: &crate::stores_receipt::Checked,
) -> Result<Shared, ServerError> {
    let model = auxiliary.apps.model()?;
    let spicedb = config.spicedb.clone().map(crate::spicedb_outbound::engine);
    // The provider reads its kept tokens on the sessions' clock, the
    // service's own (DIRECTORY-099 R1).
    let sessions = sessions(config, clock)?;
    let provider = provider(config, sessions.now()?)?;
    Ok(Arc::new(AppState {
        draft_execution: crate::drafts_apply::Executor::default(),
        changes: crate::changes::Changes::new()?,
        kept_responsibilities: primary.kept_responsibilities,
        import_credential_file: config.import_credential_file.clone(),
        estate_plan_file: config.log_dir.with_file_name("estate-approval.json"),
        identity_upstream: identity_upstream(config),
        directory: Mutex::new(primary.directory),
        oidc: Oidc::discover(config).await?,
        sign_in: crate::sign_in::IssuerSignIn::configured(config)?,
        provider,
        setup: config.setup.clone(),
        password_policy: config.password_policy.clone(),
        model_proxy: config.model_proxy.clone(),
        dialled_model_proxy: crate::model_proxy_route::dialled(config)?,
        proxy_dir: config.proxy_dir.clone(),
        setup_lock: tokio::sync::Mutex::new(()),
        sessions,
        admission: Admission::new(
            crate::setup::administrator(config)?,
            config.link_audit_binding()?,
        ),
        grants: Mutex::new(None),
        rights_cache: Mutex::default(),
        grant_setup: grant_setup(config, &auxiliary.apps, model, spicedb, checked),
        stores: checked.held(),
        secrets: config
            .secrets
            .as_ref()
            .map(crate::secrets_api::SecretsBroker::open)
            .transpose()?,
        mailer: config
            .mail
            .as_ref()
            .map(crate::mail::Mailer::open)
            .transpose()?,
        foundings: Mutex::new(crate::foundings_store::Foundings::open(
            config.log_dir.with_file_name("foundings.json"),
        )?),
        pairings: Mutex::new(crate::pair_store::PairStore::beside(&config.log_dir)?),
        requests: primary.requests.map(Mutex::new),
        mcp_requests: primary.mcp_requests,
        network: primary.network.map(Mutex::new),
        joins: crate::network_join::JoinStore::configured(config)?.map(Mutex::new),
        roles: primary.roles.map(Mutex::new),
        provisioning: primary.provisioning.map(Mutex::new),
        certificates: primary.certificates.map(Mutex::new),
        runtime: primary.runtime.map(Mutex::new),
        service_accounts: primary.service_accounts.map(Mutex::new),
        reviews: primary.reviews.map(Mutex::new),
        review_period_days: config.reviews.period_days,
        teams: primary.teams.map(Mutex::new),
        stops: primary.stops.map(Mutex::new),
        cord: Mutex::new(auxiliary.cord),
        canvas: crate::canvas_store::CanvasStore::beside(&config.log_dir),
        getting_started: crate::getting_started_api::GettingStartedStore::beside(&config.log_dir),
        budgets: primary.budgets.map(Mutex::new),
        configuration: Mutex::new(primary.configuration),
        policies: primary.policies.map(Mutex::new),
        goals: auxiliary.goals.map(crate::goals_store::Goals::new),
        words: auxiliary.words,
        variables: auxiliary.variables,
        schedules: auxiliary.schedules,
        agent_passes: Arc::new(Mutex::new(crate::agent_pass_store::Passes::open(
            config.log_dir.with_file_name("agent-passes.json"),
        )?)),
        grant_tokens: grant_tokens(config)?,
        apps: Mutex::new(auxiliary.apps),
        benches: crate::apps_bench::Benches::new(
            config.apps_dir().with_file_name("benches"),
            &*say,
        )?,
        sign_in_providers: sign_in_providers(config)?,
        agent_nonces: Mutex::default(),
        operator_token: primary.operator_token,
        operator_upgrade_file: config.operator_upgrade_file.clone(),
        runners: crate::runner_client::Runners::new(primary.key, config.runner_socket.clone()),
        acts: Mutex::new(auxiliary.acts),
        say,
        membership: crate::channel_membership_counts::Membership::new(config.membership),
        seats: Mutex::new(auxiliary.seats),
        seat_imports: auxiliary.seat_imports,
    }))
}

fn identity_upstream(config: &Config) -> String {
    format!(
        "http://{}:{}{}",
        if config.listen.is_ipv6() {
            "[::1]"
        } else {
            "127.0.0.1"
        },
        config.listen.port(),
        if config.surface_dir.is_some() {
            "/api"
        } else {
            ""
        }
    )
}

fn provider(
    config: &Config,
    at: u64,
) -> Result<Option<crate::provider::OpenIdProvider>, ServerError> {
    config
        .provider
        .as_ref()
        .map(|settings| {
            crate::provider::OpenIdProvider::open_at(
                settings,
                crate::sign_in::lys_origin(config)?,
                at,
            )
        })
        .transpose()
}

fn sessions(config: &Config, clock: lys_core::clock::ClockSource) -> Result<Sessions, ServerError> {
    Ok(match &config.sessions_file {
        Some(file) => Sessions::open_with_clock(
            file.clone(),
            config.session_seconds,
            config.secure_cookie,
            clock,
        )?,
        None => Sessions::new_with_clock(config.session_seconds, config.secure_cookie, clock),
    })
}

fn grant_setup(
    config: &Config,
    apps: &crate::apps_store::AppStore,
    model: lys_identity::grants::Model,
    spicedb: Option<crate::spicedb::SpiceDbEngine>,
    checked: &crate::stores_receipt::Checked,
) -> GrantSetup {
    GrantSetup {
        log_dir: config.grant_log_dir.clone(),
        log_origin: config.grant_log_origin.clone(),
        key_file: config.event_key_file.clone(),
        model: std::sync::RwLock::new(model),
        model_revision: std::sync::atomic::AtomicU64::new(apps.model_revision()),
        refresh: Mutex::new(()),
        changes: crate::grant_changes::GrantChanges::counted(checked.counts("grant-log")),
        spicedb,
    }
}

fn grant_tokens(config: &Config) -> Result<Mutex<crate::grant_token_store::Tokens>, ServerError> {
    Ok(Mutex::new(
        crate::grant_token_store::Tokens::open(config.log_dir.with_file_name("grant-tokens.json"))
            .map_err(|error| ServerError::ConfigInvalid {
                reason: error.to_string(),
            })?,
    ))
}

fn sign_in_providers(
    config: &Config,
) -> Result<Option<crate::sign_in_providers::SignInProviders>, ServerError> {
    config
        .sign_in_providers
        .as_ref()
        .map(|settings| {
            crate::sign_in_providers::SignInProviders::open(
                settings,
                config.provider_origins.clone(),
            )
        })
        .transpose()
}

fn recover(state: &Shared) -> Result<(), ServerError> {
    // Every person, agent and grant is under an organisation before any
    // read of them is served (DIRECTORY-109 R2).
    crate::organisation_boot::at_start(state)?;
    crate::teams_migration::at_start(state)?;
    crate::budgets_migration::advance(state)?;
    // A refused grant checkpoint holds the grants until the operator resets
    // them (ACCESS-006 R5); the rest of the service still starts and answers,
    // and the loader's grants are brought forward at a later start.
    match crate::import_bootstrap::ensure(state) {
        Err(ServerError::Grant(lys_identity::grants::GrantError::CheckpointRefused {
            refusal,
        })) => (state.say)(&format!(
            "the loader's grants wait for the operator's checkpoint reset: {refusal}"
        )),
        other => other?,
    }
    // Imports resume before any schedule timer runs, so a schedule an
    // interrupted import set is paused before it could fire.
    crate::seat_import_apply::resume_at_start(state);
    crate::goals_api::remind_from(state);
    crate::schedules_api::fire_from(state);
    crate::budgets_act::settle_at_start(state);
    crate::refusals_follow::follow_at_start(state);
    crate::grants_refusals::hold_at_start(state);
    Ok(())
}

fn served_routes(
    config: &Config,
    state: Shared,
    catalogue: Arc<crate::harness_catalogue::Catalogue>,
) -> Result<Router, ServerError> {
    let configured = crate::configuration_api::routes(config)
        .merge(crate::message_edges::routes(config)?)
        .merge(crate::memory_api::routes(config))
        .merge(crate::certificates_api::routes())
        .with_state(Arc::clone(&state));
    let start_service = crate::routes::door_handles::start_service(config, &state)?;
    crate::agent_pass_recovery::at_start(&state, &start_service)?;
    // The dashboard reads the launch records an agent gave (AGENTS-016 R5).
    let launches = axum::Extension(Arc::clone(&start_service));
    let starts = start::routes(start_service);
    let starts = crate::start_budget::guarded(
        starts.merge(crate::launch_api::routes().with_state(Arc::clone(&state))),
        Arc::clone(&state),
    );
    let origin = crate::sign_in::lys_origin(config)?;
    let mcp_path = if config.surface_dir.is_some() {
        "/api/mcp"
    } else {
        "/mcp"
    };
    let apps = crate::mcp_oauth::Apps::open(
        Arc::clone(&state),
        &config.log_dir,
        &origin,
        mcp_path,
        &config.trusted_proxies,
    )?;
    let provider_callback = crate::sign_in::callback_routes(Arc::clone(&state))
        .merge(crate::provider::routes(Arc::clone(&state)))
        .merge(crate::mcp_oauth::routes(Arc::clone(&apps)));
    // Authenticate the inner API/provider routes before body extraction, and
    // read a body whole only for a caller judged before it (`signed_first`).
    // Static screens remain public and read no body; the API fallback cannot
    // reach their wildcard.
    let guarded = |routes| {
        crate::session_admission::guarded(
            crate::signed_first::guarded(routes, Arc::clone(&state)),
            Arc::clone(&state),
        )
    };
    let api = guarded(
        crate::routes_table::router(Arc::clone(&state))
            .merge(configured)
            .merge(starts)
            .layer(axum::Extension(catalogue))
            .layer(launches)
            .fallback(crate::surface::not_an_api_route),
    );
    let dispatcher = api.clone().layer(axum::middleware::from_fn_with_state(
        Arc::clone(&state),
        crate::operator::guard,
    ));
    let api = api.merge(crate::mcp_oauth::challenged(
        crate::mcp_endpoint::run_refusals(
            guarded(crate::mcp_endpoint::admitted_routes(
                dispatcher,
                &origin,
                Arc::clone(&state),
                Arc::clone(&apps),
            )?),
            Arc::clone(&state),
        ),
        apps.metadata_address(),
    ));
    let served = match &config.surface_dir {
        Some(dir) => crate::surface::serving(
            dir,
            api.merge(crate::provider::api_keys(Arc::clone(&state))),
        )?,
        None => api,
    };
    Ok(served
        .merge(guarded(provider_callback))
        .merge(issuer_admin::routes(config)?)
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::operator::guard,
        ))
        // Outermost: a keyed model call from a run on a dialled-in computer
        // is admitted by its run key at the proxy, never by a Lys session.
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::model_proxy_route::carry,
        ))
        // Every refusal a lost store caused says so (DIRECTORY-107 R4).
        .layer(axum::middleware::from_fn_with_state(
            state,
            crate::stores_receipt::words::said,
        )))
}
