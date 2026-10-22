//! Install phases retain their order while each owns its inputs and readbacks.
use super::{
    BINARIES, Compose, DeploymentConfig, Emitter, ErrorKind, IdentityError, IdentityResult, Layout,
    Options, Outcome, adopt, broker_trust, configure, directory_key, in_lys_words, layout,
    operator_token, place_runner, ports, prepare, private_files, proxy, server_config,
    server_state, service_key, services, setup_code, start_runner, surface, swap, upgrade,
    write_absent, write_plain,
};
use base64::Engine;
use lys_core::Ed25519Identity;
use std::path::Path;
use std::sync::Arc;
pub(super) fn install(options: &Options, json: bool) -> IdentityResult<()> {
    let layout = match &options.root {
        Some(root) => Layout::at(root.clone()),
        None => Layout::discover()?,
    };
    // Before anything is read from a build or changed (DIRECTORY-107 R2).
    super::super::accept_loss::check_before(&layout, "install")?;
    let sibling = |name: &str| sibling_binary(options, name);
    let client_program = sibling("lys")?;
    let client_folder = client_program.parent().ok_or_else(|| {
        IdentityError::new(
            ErrorKind::BinaryMissing,
            "prepare clients",
            "lys",
            "the built client has no containing folder",
        )
    })?;
    let clients = layout::prepare_clients(
        options.clients.as_deref(),
        client_folder,
        &upgrade::version(&client_program, "lys")?,
    )?;
    let mut emitter = Emitter::new(json);
    let ports = ports::Ports::load(&layout)?.chosen(
        options.service_port,
        options.broker_port,
        options.proxy_port,
    )?;
    let units = upgrade::units_at(&layout, ports);
    swap::recover(&layout, &units, &mut Compose, &mut |line| {
        emitter.note(line);
    })?;
    adopt::check_placed_build(&layout, &BINARIES, &sibling)?;
    let config = prepare_configuration(&layout, options, ports, &mut emitter)?;
    let key = prepare_services(&layout, &config, &mut emitter)?;
    if let Some(package) = &options.surface {
        let manifest = surface::place(package, &layout.surface_dir())?;
        emitter.note(&format!("screens placed from commit {}", manifest.commit));
    }
    let surface_present = layout.surface_dir().join("index.html").is_file();
    let changed = configure_identity(
        &layout,
        &config,
        options,
        ports,
        surface_present,
        &mut emitter,
    )?;
    let code = if setup_code::has_administrator(&layout)? {
        None
    } else {
        Some(setup_code::issue_pending(
            &layout,
            setup_code::Purpose::FirstRun,
            &super::super::presence::SystemPresence,
        )?)
    };
    record_clients(&layout, &clients, &mut emitter)?;
    swap::announce_restart(&units, changed || options.surface.is_some())?;
    let build = adopt::settle(
        &layout,
        &units,
        &sibling,
        changed,
        options.surface.is_some(),
        &mut |line| {
            emitter.note(line);
        },
    )?;
    let build = serde_json::to_value(&build).map_err(|error| {
        IdentityError::new(
            ErrorKind::RenderFailed,
            "render",
            "build.json",
            error.to_string(),
        )
    })?;
    if emitter.is_json() {
        emitter.field("build", "build", build);
    }
    place_runner(&layout, &sibling, &mut |line| {
        emitter.note(line);
    })?;
    start_runner(&layout, &key, &layout.binary("lys"), &mut |line| {
        emitter.note(line);
    })?;
    proxy::start(&layout, ports, &layout.binary("lys"), &mut |line| {
        emitter.note(line);
    })?;
    let address = ports::PersonAddress::load(&layout, ports)?;
    // The setup code's handover says the same when a code is handed over.
    if code.is_none() {
        address.say_fallback(&mut emitter);
    }
    emitter.field("open", "url", address.origin.clone());
    emitter.field("sign-in for products", "issuer", address.origin);
    setup_code::hand_over_first_run(&layout, code, &mut emitter)?;
    if !surface_present {
        emitter.note("no screens placed: pass --surface with a compiled screens package");
    }
    emitter.finish();
    Ok(())
}

fn prepare_services(
    layout: &Layout,
    config: &DeploymentConfig,
    emitter: &mut Emitter,
) -> IdentityResult<Arc<Ed25519Identity>> {
    let materialised = prepare::materialise_all(config)?;
    emitter.note(&format!(
        "{} private files ready (only you can read them)",
        materialised.len()
    ));
    services::compose_up(layout, config)?;
    emitter.note("database, sign-in and permission services up");
    services::wait_ready(layout, config, &mut |line| {
        println!("{}", in_lys_words(line));
    })?;
    emitter.note("database, sign-in and permission services ready");
    configure::reconcile(config)?;
    emitter.note("sign-in clients registered");
    configure::apply_password_policy(config)?;
    emitter.note("Lys's password policy set");
    let key = Arc::new(service_key(layout)?);
    let public = base64::engine::general_purpose::STANDARD.encode(key.public_key_bytes());
    emitter.note(&format!("service key public {public}"));
    if services::broker_init(layout)? {
        emitter.note("secrets broker made");
    }
    broker_trust::trust(layout, server_config::SERVICE_NAME)?;
    emitter.note("secrets broker trusts the identity service");
    Ok(key)
}

fn configure_identity(
    layout: &Layout,
    config: &DeploymentConfig,
    options: &Options,
    ports: ports::Ports,
    surface_present: bool,
    emitter: &mut Emitter,
) -> IdentityResult<bool> {
    server_state(layout, config)?;
    let service_key_file = directory_key::provide(config)?;
    private_files::write(
        &layout.data_dir().join("estate-approval.json"),
        layout::ESTATE_PLAN.as_bytes(),
    )?;
    let mut carried = server_config::carried(layout)?.unwrap_or_default();
    carried.ports = ports;
    let profile = options.profile.or(carried.profile).unwrap_or_default();
    carried.profile = Some(profile);
    emitter.note(&format!("profile: {}", profile.word()));
    operator_token(config, profile, &mut |line| emitter.note(line))?;
    if let Some(path) = &options.message_service {
        carried.message_service = Some(server_config::messages_from(path)?);
        emitter.note("message service connection kept");
    }
    let rendered = server_config::render(layout, config, &carried, surface_present);
    let encoded = serde_json::to_vec_pretty(&rendered).map_err(|error| {
        IdentityError::new(
            ErrorKind::RenderFailed,
            "render",
            "identity.json",
            error.to_string(),
        )
    })?;
    let configuration = private_files::write(&layout.service_config(), &encoded)?;
    emitter.note(&server_config::membership_readback(
        rendered.get("membership"),
    ));
    emitter.note(&server_config::provider_readback(
        rendered["provider"].get("rights_bytes"),
        &layout.logs_dir().join("identity.log"),
    )?);
    Ok(configuration != Outcome::Unchanged || service_key_file != Outcome::Unchanged)
}

fn record_clients(
    layout: &Layout,
    clients: &layout::ClientSet,
    emitter: &mut Emitter,
) -> IdentityResult<()> {
    clients.place(layout, &mut layout::PlacedClients::default(), &mut |line| {
        emitter.note(line);
    })?;
    let manifest = clients.manifest(layout)?;
    if manifest.target.exists() {
        let bytes = std::fs::read(&manifest.target).map_err(|error| {
            IdentityError::new(
                ErrorKind::PrivateFileIo,
                "back up",
                "clients",
                error.to_string(),
            )
            .at(&manifest.target)
        })?;
        private_files::write(&layout.root.join("clients.json.previous"), &bytes)?;
    }
    swap::place_file(&manifest.target, &manifest.bytes, false)?;
    emitter.note(&format!(
        "clients recorded: {} platforms",
        clients.releases.len()
    ));
    Ok(())
}

fn prepare_configuration(
    layout: &Layout,
    options: &Options,
    ports: ports::Ports,
    emitter: &mut Emitter,
) -> IdentityResult<DeploymentConfig> {
    services::require_docker(Path::new("docker"))?;
    private_files::ensure_dir(&layout.root)?;
    for dir in [
        layout.deploy_dir(),
        layout.logs_dir(),
        layout.run_dir(),
        layout.data_dir(),
    ] {
        private_files::ensure_dir(&dir)?;
    }
    emitter.field("root", "root", layout.root.display().to_string());
    write_plain(
        &layout.deploy_dir().join("compose.yaml"),
        layout::COMPOSE_YAML,
    )?;
    write_plain(
        &layout.deploy_dir().join("postgres-init.sql"),
        layout::POSTGRES_INIT_SQL,
    )?;
    let config_path = layout.deployment_config();
    let deployment = if ports.service == layout::SERVICE_PORT {
        layout::render_deployment(options.admin_email.as_deref())
    } else {
        layout::render_deployment_at(options.admin_email.as_deref(), ports.service)
    };
    if write_absent(&config_path, &deployment)? {
        emitter.note("deployment.toml written");
    }
    DeploymentConfig::load_install(&config_path)
}

fn sibling_binary(options: &Options, name: &str) -> IdentityResult<std::path::PathBuf> {
    match &options.from {
        Some(from) => Ok(from.join(name)),
        None => services::sibling(name),
    }
}
