//! Start and close a service fixture with its configured stores and issuer.

use std::error::Error;
use std::sync::Arc;

use lys_core::clock::{Clock, ClockSource};
use lys_identity_server::config::ConfiguredLogin;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_identity_server::sign_in_providers::{ProviderOrigins, SignInProvidersSettings};
use lys_identity_server::spicedb::SpiceDbSettings;
use lys_identity_server::{Config, Say};

use super::{
    ADMINISTRATOR, ClockServing, GRANT_MODEL, GRANT_ORIGIN, LINK_AUDIT_SOURCE,
    MEMBERSHIP_CURSOR_SECONDS, MEMBERSHIP_CURSORS_MAX, MEMBERSHIP_PAGE_BYTES_MAX,
    MEMBERSHIP_PAGE_ROWS_MAX, ORIGIN, Service, secret_file,
};
use crate::fake_issuer::{CLIENT_ID, CLIENT_SECRET, FakeIssuer};
use crate::harness_serve::{DropMarker, StageTimer, serve};

/// `model` with the shipped administrator relation added when it names
/// none: the install writes the shipped model, which always carries it, so
/// a fixture names only the relations its test is about and the
/// organisation's founder grant still issues at boot (DIRECTORY-109 R3).
pub fn with_administrator(model: &str) -> Result<String, Box<dyn Error>> {
    use lys_identity::grants::shipped::{ADMINISTRATOR_RELATION, ORGANISATION_ACTIONS};
    let mut file: serde_json::Value = serde_json::from_str(model)?;
    let relations = file
        .get_mut("relations")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("the fixture model names no relations")?;
    if !relations.contains_key(ADMINISTRATOR_RELATION) {
        relations.insert(
            ADMINISTRATOR_RELATION.to_owned(),
            serde_json::json!(ORGANISATION_ACTIONS),
        );
    }
    Ok(serde_json::to_string(&file)?)
}

impl Service {
    /// Stop and join every fixture task, reporting all shutdown failures.
    ///
    /// # Errors
    /// Returns the named worker, task or state-lock failure during shutdown.
    pub fn close(&mut self) -> std::io::Result<()> {
        let timing = StageTimer::new("fixture.close");
        let mut failures = Vec::new();
        if let Err(error) = self.server.stop() {
            failures.push(format!("service_shutdown_failed: {error}"));
        }
        let issuer_timing = StageTimer::new("fixture.issuer_shutdown");
        if let Err(error) = self.issuer.shutdown() {
            failures.push(format!("issuer_shutdown_failed: {error}"));
        }
        drop(issuer_timing);
        drop(timing);
        if failures.is_empty() {
            Ok(())
        } else {
            Err(std::io::Error::other(failures.join("; ")))
        }
    }

    /// Start the service over a fresh log, with the fake issuer's administrator
    /// and link-audit source configured.
    ///
    /// # Errors
    /// Propagates fixture configuration, preparation or service-start failures.
    pub async fn start() -> Result<Self, Box<dyn Error>> {
        Ok(Self::start_with(|_| Ok(())).await?.0)
    }

    /// Start the service as [`Service::start`] does, after `prepare` has
    /// written to the directory its configuration names, answering what
    /// `prepare` answered.
    ///
    /// # Errors
    /// Propagates fixture configuration, preparation or service-start failures.
    pub async fn start_with<T: Send>(
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_judging(GRANT_MODEL, None, prepare).await
    }

    /// Start the same HTTP service using a separately owned session/request clock.
    ///
    /// # Errors
    /// Propagates fixture preparation, clock and actual service startup failures.
    pub async fn start_with_clock<T: Send>(
        clock: Arc<dyn Clock>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Box::pin(Self::start_clock_saying(
            GRANT_MODEL,
            None,
            None,
            None,
            |_| {},
            ClockServing {
                say: None,
                clock: ClockSource::Supplied(clock),
            },
            prepare,
        ))
        .await
    }

    /// Start the service as [`Service::start_saying`] does, after `adjust`,
    /// reading the time from `clock`, so a test can move the time past an
    /// instant and read every line the service said.
    ///
    /// # Errors
    /// Propagates fixture configuration, preparation, clock or service-start failures.
    pub async fn start_clock_adjusted<T: Send>(
        clock: Arc<dyn Clock>,
        adjust: impl FnOnce(&mut Config) + Send,
        say: Option<Say>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Box::pin(Self::start_clock_saying(
            GRANT_MODEL,
            None,
            None,
            None,
            adjust,
            ClockServing {
                say,
                clock: ClockSource::Supplied(clock),
            },
            prepare,
        ))
        .await
    }

    /// Start the service as [`Service::start_with`] does, judging grants by
    /// `model` and keeping their relationships in the permission database
    /// `spicedb` names, or in the process when it names none.
    ///
    /// # Errors
    /// Propagates fixture configuration, preparation or service-start failures.
    pub async fn start_judging<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_asking(model, spicedb, None, prepare).await
    }

    /// Start the service as [`Service::start_judging`] does, asking the
    /// secrets broker `secrets` names for the secrets screens, when it names
    /// one.
    ///
    /// # Errors
    /// Propagates fixture configuration, preparation or service-start failures.
    pub async fn start_asking<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_setting(model, spicedb, secrets, None, prepare).await
    }

    /// Start the service as [`Service::start_asking`] does, setting the
    /// sign-in providers through the issuer API `sign_in_providers` names,
    /// when it names one.
    ///
    /// # Errors
    /// Propagates fixture configuration, preparation or service-start failures.
    pub async fn start_setting<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        sign_in_providers: Option<SignInProvidersSettings>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_adjusted(model, spicedb, secrets, sign_in_providers, |_| {}, prepare).await
    }

    /// Start the service as [`Service::start_setting`] does, after `adjust`
    /// has changed the configuration the harness wrote, so a test can
    /// configure an administrator the fake issuer never signs in.
    ///
    /// # Errors
    /// Propagates fixture configuration, preparation or service-start failures.
    pub async fn start_adjusted<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        sign_in_providers: Option<SignInProvidersSettings>,
        adjust: impl FnOnce(&mut Config) + Send,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Box::pin(Self::start_saying(
            model,
            spicedb,
            secrets,
            sign_in_providers,
            adjust,
            None,
            prepare,
        ))
        .await
    }

    /// Start the service as [`Service::start_adjusted`] does, saying each
    /// line of its log to `say` when one is given, so a test reads every
    /// line the service said.
    ///
    /// # Errors
    /// Propagates fixture configuration, preparation or service-start failures.
    pub async fn start_saying<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        sign_in_providers: Option<SignInProvidersSettings>,
        adjust: impl FnOnce(&mut Config) + Send,
        say: Option<Say>,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        Self::start_clock_saying(
            model,
            spicedb,
            secrets,
            sign_in_providers,
            adjust,
            ClockServing {
                say,
                clock: ClockSource::System,
            },
            prepare,
        )
        .await
    }

    async fn start_clock_saying<T: Send>(
        model: &str,
        spicedb: Option<SpiceDbSettings>,
        secrets: Option<SecretsSettings>,
        sign_in_providers: Option<SignInProvidersSettings>,
        adjust: impl FnOnce(&mut Config) + Send,
        serving: ClockServing,
        prepare: impl FnOnce(&Config) -> Result<T, Box<dyn Error>> + Send,
    ) -> Result<(Self, T), Box<dyn Error>> {
        let ClockServing { say, clock } = serving;
        let timing = StageTimer::new("fixture.start");
        let stage = StageTimer::new("fixture.bootstrap");
        let dir = tempfile::TempDir::new()?;
        secret_file(&dir.path().join("issuer.key"), &[3; 32])?;
        secret_file(&dir.path().join("service.key"), &[9; 32])?;
        secret_file(&dir.path().join("client.secret"), CLIENT_SECRET.as_bytes())?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let listen = listener.local_addr()?;
        let base = format!("http://{listen}");
        drop(stage);
        let stage = StageTimer::new("fixture.issuer_start");
        let issuer = FakeIssuer::start_behind(&base, &dir.path().join("issuer.key")).await?;
        issuer.set_public_callback(&format!("{base}/auth/v1/providers/callback"))?;
        drop(stage);
        let stage = StageTimer::new("fixture.configure_and_adjust");
        let mut config = configured_service(
            &dir,
            &issuer,
            listen,
            &base,
            spicedb,
            secrets,
            sign_in_providers,
        );
        adjust(&mut config);
        std::fs::write(&config.grant_model_file, with_administrator(model)?)?;
        config.validate()?;
        drop(stage);
        let stage = StageTimer::new("fixture.prepare_callback");
        let prepared = prepare(&config)?;
        drop(stage);
        let stage = StageTimer::new("fixture.restore");
        crate::service_template::restore(&config)?;
        drop(stage);
        let before = lys_log_store::process_flush_count();
        let (server, client, at_stop) = serve(listener, &config, say, &clock).await?;
        let startup_flushes = lys_log_store::process_flush_count() - before;
        drop(timing);
        Ok((
            Self {
                drop_start: DropMarker::new("service.fields_start"),
                base,
                startup_flushes,
                issuer,
                drop_issuer: DropMarker::new("service.issuer_dropped"),
                client,
                drop_client: DropMarker::new("service.client_dropped"),
                dir,
                drop_dir: DropMarker::new("service.tempdir_dropped"),
                config,
                clock,
                drop_config: DropMarker::new("service.config_dropped"),
                server,
                at_stop,
                drop_server: DropMarker::new("service.fields_end"),
            },
            prepared,
        ))
    }
}

fn configured_service(
    dir: &tempfile::TempDir,
    issuer: &FakeIssuer,
    listen: std::net::SocketAddr,
    base: &str,
    spicedb: Option<SpiceDbSettings>,
    secrets: Option<SecretsSettings>,
    sign_in_providers: Option<SignInProvidersSettings>,
) -> Config {
    let configured = |subject: &str| ConfiguredLogin {
        issuer: issuer.issuer().to_owned(),
        subject: subject.to_owned(),
    };
    Config {
        message_service: None,
        listen,
        log_dir: dir.path().join("log"),
        log_origin: ORIGIN.to_owned(),
        event_key_file: dir.path().join("service.key"),
        import_credential_file: None,
        issuer: issuer.issuer().to_owned(),
        client_id: CLIENT_ID.to_owned(),
        client_secret_file: dir.path().join("client.secret"),
        redirect_url: format!("{base}/callback"),
        sign_in_api: Some(issuer.api().to_owned()),
        trusted_proxies: Vec::new(),
        profile: lys_identity_server::config::Profile::Service,
        operator_token_file: None,
        operator_upgrade_file: None,
        administrator: Some(configured(ADMINISTRATOR)),
        link_audit_source: configured(LINK_AUDIT_SOURCE),
        issuer_moved_from: None,
        session_seconds: 600,
        sessions_file: Some(dir.path().join("sessions.json")),
        secure_cookie: false,
        grant_log_dir: dir.path().join("grant-log"),
        grant_log_origin: GRANT_ORIGIN.to_owned(),
        grant_model_file: dir.path().join("grant-model.json"),
        spicedb,
        secrets,
        mail: None,
        requests_dir: Some(dir.path().join("requests")),
        certificates_dir: Some(dir.path().join("certificates")),
        network_file: Some(dir.path().join("network.json")),
        roles_file: Some(dir.path().join("roles.json")),
        provisioning_file: Some(dir.path().join("provisioning.json")),
        homes_dir: Some(dir.path().join("homes")),
        runtime_dir: Some(dir.path().join("runtime")),
        service_accounts_dir: Some(dir.path().join("service-accounts")),
        teams_dir: Some(dir.path().join("teams")),
        stops_dir: Some(dir.path().join("stops")),
        budgets_dir: Some(dir.path().join("budgets")),
        policies_dir: Some(dir.path().join("policies")),
        goals_dir: Some(dir.path().join("goals")),
        words_dir: Some(dir.path().join("words")),
        variables_dir: Some(dir.path().join("variables")),
        schedules_dir: Some(dir.path().join("schedules")),
        reviews_dir: Some(dir.path().join("reviews")),
        reviews: lys_identity_server::config::ReviewSettings { period_days: 30 },
        sign_in_providers,
        provider_origins: Some(ProviderOrigins {
            google: issuer.provider_base().to_owned(),
            microsoft: issuer.provider_base().to_owned(),
            github: issuer.provider_base().to_owned(),
        }),
        provider: None,
        setup: Some(lys_identity_server::setup::SetupSettings {
            code_file: dir.path().join("setup").join("code"),
            administrator_file: dir.path().join("setup").join("administrator"),
            email: None,
            organisation: crate::fixtures::ORGANISATION.to_owned(),
        }),
        password_policy: None,
        model_proxy: Some("http://127.0.0.1:18484".to_owned()),
        proxy_dir: None,
        surface_dir: None,
        runner_socket: None,
        stores_receipt_file: dir.path().join("install").join("stores.json"),
        stores_accepted_file: dir.path().join("install").join("stores.accepted.json"),
        membership: Some(lys_identity_server::config::MembershipSettings {
            page_rows_max: MEMBERSHIP_PAGE_ROWS_MAX,
            page_bytes_max: MEMBERSHIP_PAGE_BYTES_MAX,
            cursor_seconds: MEMBERSHIP_CURSOR_SECONDS,
            cursors_max: MEMBERSHIP_CURSORS_MAX,
        }),
    }
}
