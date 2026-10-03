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
use crate::routes::{AppState, Say, Shared, door_handles, open_directory, start};
use crate::service_accounts_store::ServiceAccountStore;
use crate::session::Sessions;

pub(crate) async fn service_saying(config: &Config, say: Say) -> Result<Router, ServerError> {
    let kept_responsibilities = crate::kept_responsibilities::Kept::load(
        &config.log_dir.with_file_name("kept-responsibilities.json"),
    )?;
    crate::openapi::prepare()?;
    crate::openapi::surface::prepare()?;
    let catalogue = Arc::new(crate::harness_catalogue::Catalogue::embedded()?);
    let operator_token = crate::operator::token(config, &*say)?;
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
    let requests = crate::requests_store::RequestStore::opened(config, Arc::clone(&key), &*say)?;
    let mcp_requests =
        crate::mcp_requests_store::McpRequestStore::configured(config, Arc::clone(&key), &say)?
            .map(|store| Box::new(Mutex::new(store)));
    let certificates = config
        .certificates_dir
        .as_deref()
        .map(|dir| {
            crate::certificates_store::CertificateStore::opened(dir, Arc::clone(&key), &*say)
        })
        .transpose()?;
    let (network, roles, provisioning) = crate::file_stores::opened(config, &*say)?;
    let mut runtime = crate::runtime_store::RuntimeStore::configured(config, &say)?;
    let service_accounts = ServiceAccountStore::configured(config, Arc::clone(&key), &say)?;
    let reviews = ReviewStore::configured(config, Arc::clone(&key), &*say)?;
    let teams = crate::teams_store::TeamStore::configured(config, Arc::clone(&key), &say)?;
    let stops = crate::stops_store::StopStore::configured(config, Arc::clone(&key), &say)?;
    let mut budgets =
        crate::budgets_store::BudgetStore::configured(config, Arc::clone(&key), &say)?;
    if let (Some(runtime), Some(budgets)) = (runtime.as_mut(), budgets.as_mut()) {
        let agents = budgets.held().context_availability.agents();
        let sessions = runtime.agents_with_sessions(&agents)?;
        budgets.reconcile_context(Some(&sessions))?;
    }
    let mut policies =
        crate::agent_policy_store::PolicyStore::configured(config, Arc::clone(&key), &say)?;
    if let Some(store) = policies.as_mut() {
        for (id, _) in directory.projection()?.records() {
            if let lys_identity::IdentityId::Agent(agent) = id {
                store.ensure_default(&agent.to_string())?;
            }
        }
    }
    let goals = crate::goals_store::GoalStore::configured(config, Arc::clone(&key), &say)?;
    let acts = crate::runner_acts::ActStore::open(
        &config.log_dir.with_file_name("runner-acts"),
        Arc::clone(&key),
    )?;
    say(&format!(
        "runner-acts log {}, holding {} acts",
        acts.start(),
        acts.len()
    ));
    let apps = crate::apps_api::opened(config, Arc::clone(&key), &*say)?;
    let model = apps.model()?;
    let spicedb = config
        .spicedb
        .clone()
        .map(crate::spicedb::SpiceDbEngine::new);
    let state = Arc::new(AppState {
        changes: crate::changes::Changes::new()?,
        kept_responsibilities,
        import_credential_file: config.import_credential_file.clone(),
        estate_plan_file: config.log_dir.with_file_name("estate-approval.json"),
        identity_upstream: format!(
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
        ),
        directory: Mutex::new(directory),
        oidc: Oidc::discover(config).await?,
        sign_in: crate::sign_in::IssuerSignIn::configured(config)?,
        provider: config
            .provider
            .as_ref()
            .map(|settings| {
                crate::provider::OpenIdProvider::open(settings, crate::sign_in::lys_origin(config)?)
            })
            .transpose()?,
        setup: config.setup.clone(),
        password_policy: config.password_policy.clone(),
        model_proxy: config.model_proxy.clone(),
        setup_lock: tokio::sync::Mutex::new(()),
        sessions: match &config.sessions_file {
            Some(file) => {
                Sessions::open(file.clone(), config.session_seconds, config.secure_cookie)?
            }
            None => Sessions::new(config.session_seconds, config.secure_cookie),
        },
        admission: Admission::new(
            crate::setup::administrator(config)?,
            config.link_audit_binding()?,
        ),
        grants: Mutex::new(None),
        grant_setup: GrantSetup {
            log_dir: config.grant_log_dir.clone(),
            log_origin: config.grant_log_origin.clone(),
            key_file: config.event_key_file.clone(),
            model: std::sync::RwLock::new(model),
            model_revision: std::sync::atomic::AtomicU64::new(apps.model_revision()),
            refresh: Mutex::new(()),
            spicedb,
        },
        secrets: config
            .secrets
            .as_ref()
            .map(crate::secrets_api::SecretsBroker::open)
            .transpose()?,
        requests: requests.map(Mutex::new),
        mcp_requests,
        network: network.map(Mutex::new),
        roles: roles.map(Mutex::new),
        provisioning: provisioning.map(Mutex::new),
        certificates: certificates.map(Mutex::new),
        runtime: runtime.map(Mutex::new),
        service_accounts: service_accounts.map(Mutex::new),
        reviews: reviews.map(Mutex::new),
        teams: teams.map(Mutex::new),
        stops: stops.map(Mutex::new),
        budgets: budgets.map(Mutex::new),
        configuration: Mutex::new(configuration),
        policies: policies.map(Mutex::new),
        goals: goals.map(crate::goals_store::Goals::new),
        agent_passes: Arc::new(Mutex::new(crate::agent_pass_store::Passes::open(
            config.log_dir.with_file_name("agent-passes.json"),
        )?)),
        grant_tokens: Mutex::new(
            crate::grant_token_store::Tokens::open(
                config.log_dir.with_file_name("grant-tokens.json"),
            )
            .map_err(|error| ServerError::ConfigInvalid {
                reason: error.to_string(),
            })?,
        ),
        apps: Mutex::new(apps),
        benches: crate::apps_bench::Benches::new(
            config.apps_dir().with_file_name("benches"),
            &*say,
        )?,
        sign_in_providers: config
            .sign_in_providers
            .as_ref()
            .map(|settings| {
                crate::sign_in_providers::SignInProviders::open(
                    settings,
                    config.provider_origins.clone(),
                )
            })
            .transpose()?,
        agent_nonces: Mutex::default(),
        operator_token,
        operator_upgrade_file: config.operator_upgrade_file.clone(),
        runners: crate::runner_client::Runners::new(key, config.runner_socket.clone()),
        acts: Mutex::new(acts),
        say,
    });
    crate::teams_migration::at_start(&state)?;
    crate::budgets_migration::advance(&state)?;
    crate::import_bootstrap::ensure(&state)?;
    crate::goals_api::remind_from(&state);
    crate::budgets_act::settle_at_start(&state);
    crate::refusals_follow::follow_at_start(&state);
    crate::grants_refusals::hold_at_start(&state);
    let configured = crate::configuration_api::routes(config)
        .merge(crate::message_edges::routes(config)?)
        .merge(crate::memory_api::routes(config))
        .merge(crate::certificates_api::routes())
        .with_state(Arc::clone(&state));
    let start_service = start_service(config, &state)?;
    crate::agent_pass_recovery::at_start(&state, &start_service)?;
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
    let apps =
        crate::mcp_oauth::Apps::open(Arc::clone(&state), &config.log_dir, &origin, mcp_path)?;
    let provider_callback = crate::sign_in::callback_routes(Arc::clone(&state))
        .merge(crate::provider::routes(Arc::clone(&state)))
        .merge(crate::mcp_oauth::routes(Arc::clone(&apps)));
    // Authenticate the inner API/provider routes before body extraction. Static
    // screens remain public; the API fallback cannot reach their wildcard.
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
            .fallback(crate::surface::not_an_api_route),
    );
    let dispatcher = api.clone().layer(axum::middleware::from_fn_with_state(
        Arc::clone(&state),
        crate::operator::guard,
    ));
    let api = api.merge(crate::mcp_oauth::challenged(
        guarded(crate::mcp_endpoint::admitted_routes(
            dispatcher,
            &origin,
            Arc::clone(&state),
            Arc::clone(&apps),
        )?),
        apps.metadata_address(),
    ));
    let served = match &config.surface_dir {
        Some(dir) => crate::surface::serving(dir, api)?,
        None => api,
    };
    Ok(served
        .merge(guarded(provider_callback))
        .layer(axum::middleware::from_fn_with_state(
            state,
            crate::operator::guard,
        )))
}

/// The start route's service over the directory `state` holds. The route
/// reads the handle record through the door's client, [`door_handles::DoorHandles`],
/// as its `HandleRecords`. No configuration field names the door yet, so the
/// client is built without an address and answers that no handle record
/// exists, naming SECRETS-002, until one does. Launch records are kept in
/// `launch-records` beside the directory log.
fn start_service(config: &Config, state: &Shared) -> Result<Arc<start::StartService>, ServerError> {
    let handles = door_handles::DoorHandles::unconfigured();
    let dir = config.log_dir.with_file_name("launch-records");
    start::directory_service(
        state,
        Box::new(handles),
        door_handles::credential_id,
        &dir,
        load_service_key(&config.event_key_file)?,
    )
}
