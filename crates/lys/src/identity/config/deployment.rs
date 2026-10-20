//! A deployment is loaded and validated before its fields serve an install.
use super::{
    BUNDLED_DATABASE_HOST, Client, DeploymentConfig, ErrorKind, IdentityError, IdentityResult,
    Ipv4Addr, LOOPBACK_HOSTS, MailTls, Path, PathBuf, TLS_MODES, authority_of, bring_forward,
    is_email, is_mailbox, network_gateway, refuse, validate_admin_url, validate_client,
    validate_origin,
};
impl DeploymentConfig {
    /// Reads and validates the configuration at `path`, first bringing a
    /// file an earlier build wrote forward to this build's shape and writing
    /// it back, so an install is never refused for the build it began on.
    ///
    /// # Errors
    /// Refuses unreadable configuration, malformed values or inconsistent install settings.
    pub fn load(path: &Path) -> IdentityResult<Self> {
        Self::load_brought_forward(path, false)
    }

    /// Reads and validates an install's own `deployment.toml` as [`load`]
    /// does, also moving a sign-in address an earlier install wrote on the
    /// sign-in service's own port to Lys's origin, which every install
    /// since serves it at.
    ///
    /// [`load`]: Self::load
    ///
    /// # Errors
    /// Refuses unreadable configuration, malformed values or inconsistent install settings.
    pub fn load_install(path: &Path) -> IdentityResult<Self> {
        Self::load_brought_forward(path, true)
    }

    fn load_brought_forward(path: &Path, install: bool) -> IdentityResult<Self> {
        let read = std::fs::read_to_string(path).map_err(|error| {
            IdentityError::new(
                ErrorKind::ConfigUnreadable,
                "read configuration",
                "configuration",
                error.to_string(),
            )
            .at(path)
        })?;
        let text =
            match bring_forward::bring_forward(&read, install).map_err(|error| error.at(path))? {
                Some(brought) => {
                    Self::parse(&brought, PathBuf::new()).map_err(|error| error.at(path))?;
                    bring_forward::write_back(path, &brought)?;
                    brought
                }
                None => read,
            };
        let base = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Self::parse(&text, base).map_err(|error| error.at(path))
    }

    /// Parses and validates configuration text whose relative paths resolve
    /// against `base`.
    ///
    /// # Errors
    /// Refuses unreadable configuration, malformed values or inconsistent install settings.
    pub fn parse(text: &str, base: PathBuf) -> IdentityResult<Self> {
        let mut config: Self = toml::from_str(text)
            .map_err(|error| refuse(ErrorKind::ConfigInvalid, "configuration", error.message()))?;
        config.base = base;
        config.validate()?;
        Ok(config)
    }

    /// The private state directory, resolved against the configuration file.
    #[must_use]
    pub fn state_dir(&self) -> PathBuf {
        self.base.join(&self.deployment.state_dir)
    }

    /// The managed clients, platform first.
    #[must_use]
    pub fn managed_clients(&self) -> Vec<(ClientRole, &Client)> {
        let mut clients = vec![(ClientRole::Platform, &self.clients.platform)];
        if let Some(app) = &self.clients.app {
            clients.push((ClientRole::App, app));
        }
        clients
    }

    /// Rauthy's `PUB_URL`: the public origin's authority.
    #[must_use]
    pub fn pub_url(&self) -> &str {
        authority_of(&self.issuer.public_origin)
    }

    /// The relying-party id for passkeys: the public origin's host.
    #[must_use]
    pub fn rp_id(&self) -> &str {
        let authority = self.pub_url();
        if authority.starts_with('[') {
            return authority
                .split_once(']')
                .map_or(authority, |(host, _)| host.trim_start_matches('['));
        }
        authority.split(':').next().unwrap_or(authority)
    }

    /// The compose network's gateway: the address the directory service
    /// reaches the sign-in service from.
    ///
    /// # Errors
    /// Refuses unreadable configuration, malformed values or inconsistent install settings.
    pub fn gateway(&self) -> IdentityResult<Ipv4Addr> {
        network_gateway(&self.deployment.network)
    }

    /// Whether the public origin is served over TLS by a fronting proxy.
    #[must_use]
    pub fn public_tls(&self) -> bool {
        self.issuer.public_origin.starts_with("https://")
    }

    /// Rauthy's `COOKIE_MODE`. A browser keeps a `Secure` cookie only over
    /// TLS, so an origin served over plain http, which is accepted for a
    /// loopback host alone, asks for the session cookie without that flag;
    /// without it no session survives the sign-in. An https origin keeps
    /// Rauthy's host-bound `Secure` cookie.
    #[must_use]
    pub fn cookie_mode(&self) -> &'static str {
        if self.public_tls() {
            "host"
        } else {
            "danger-insecure"
        }
    }

    fn validate(&self) -> IdentityResult<()> {
        self.validate_deployment()?;
        self.password_policy.validate()?;
        validate_origin(&self.issuer.public_origin)?;
        if self.public_tls() && self.issuer.trusted_proxies.is_empty() {
            return Err(refuse(
                ErrorKind::IssuerInvalid,
                "issuer.trusted_proxies",
                "an https origin runs the sign-in service in proxy mode, which answers its trusted proxies alone; name the network's gateway in trusted_proxies",
            ));
        }
        let gateway = self.gateway()?.to_string();
        let proxies_ok = self
            .issuer
            .trusted_proxies
            .iter()
            .all(|proxy| proxy.strip_suffix("/32").unwrap_or(proxy) == gateway);
        if !proxies_ok {
            return Err(refuse(
                ErrorKind::IssuerInvalid,
                "issuer.trusted_proxies",
                format!(
                    "the sign-in service takes a person's address from the directory service alone, which reaches it from {gateway}, the gateway of deployment.network"
                ),
            ));
        }
        validate_admin_url(&self.issuer.admin_url)?;
        if self.identity.pass_lifetime == 0 {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "identity.pass_lifetime",
                "a pass lives at least one second: every pass has an expiry after its issue",
            ));
        }
        self.validate_database()?;
        self.validate_mail()?;
        if self
            .clients
            .app
            .as_ref()
            .is_some_and(|app| app.id == self.clients.platform.id)
        {
            return Err(refuse(
                ErrorKind::ClientInvalid,
                "clients",
                "the platform and app clients need distinct ids",
            ));
        }
        for (role, client) in self.managed_clients() {
            validate_client(role, client)?;
        }
        Ok(())
    }

    fn validate_deployment(&self) -> IdentityResult<()> {
        let deployment = &self.deployment;
        if deployment.node.trim().is_empty() {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "deployment.node",
                "name the node this instance is installed on",
            ));
        }
        let project_ok = !deployment.project.is_empty()
            && deployment.project.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
            });
        if !project_ok {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "deployment.project",
                "use lowercase letters, digits, - and _ only",
            ));
        }
        if let Some(email) = &deployment.admin_email
            && !is_email(email)
        {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "deployment.admin_email",
                "expected an email address",
            ));
        }
        Ok(())
    }

    /// The `[mail]` table, when present: a host and a port, a sender that is
    /// a mailbox, a user that is a name when set, and plain SMTP for a
    /// loopback host alone (DIRECTORY-110 R1).
    fn validate_mail(&self) -> IdentityResult<()> {
        let Some(mail) = &self.mail else {
            return Ok(());
        };
        if mail.host.is_empty() || mail.host.chars().any(char::is_whitespace) {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "mail.host",
                "name the mail server's host",
            ));
        }
        if mail.port == 0 {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "mail.port",
                "name the mail server's port",
            ));
        }
        if !is_mailbox(&mail.from) {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "mail.from",
                format!("{} is not an address or `Name <address>`", mail.from),
            ));
        }
        if mail
            .user
            .as_deref()
            .is_some_and(|user| user.trim().is_empty())
        {
            return Err(refuse(
                ErrorKind::ConfigInvalid,
                "mail.user",
                "name the user, or leave it out",
            ));
        }
        if mail.tls == MailTls::None && !LOOPBACK_HOSTS.contains(&mail.host.as_str()) {
            return Err(refuse(
                ErrorKind::MailTlsRequired,
                "mail.tls",
                format!(
                    "{} is not a loopback host; letters to it go over tls or starttls, never none",
                    mail.host
                ),
            ));
        }
        Ok(())
    }

    fn validate_database(&self) -> IdentityResult<()> {
        let database = &self.database;
        let host_ok = !database.host.is_empty()
            && !database
                .host
                .contains(|c: char| c.is_whitespace() || "/@?#:".contains(c));
        if !host_ok {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.host",
                "expected a host name or address, without scheme, port or credentials",
            ));
        }
        if database.port == 0 {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.port",
                "expected a port from 1 to 65535",
            ));
        }
        if database.bundled != (database.host == BUNDLED_DATABASE_HOST) {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.host",
                "the bundled service answers as postgres; a database elsewhere is named by its own address with bundled = false",
            ));
        }
        if database.bundled && database.publish_port.is_none() {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.publish_port",
                "the bundled service needs a loopback port to be published on",
            ));
        }
        let check_ok = database
            .check_address
            .rsplit_once(':')
            .is_some_and(|(host, port)| {
                !host.is_empty() && port.parse::<u16>().is_ok_and(|p| p > 0)
            });
        if !check_ok {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.check_address",
                "expected host:port",
            ));
        }
        if !TLS_MODES.contains(&database.tls.as_str()) {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.tls",
                "expected disable, prefer or require",
            ));
        }
        let name_ok = !database.name.is_empty()
            && database
                .name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !name_ok {
            return Err(refuse(
                ErrorKind::DatabaseAddressInvalid,
                "database.name",
                "use lowercase letters, digits and _ only",
            ));
        }
        Ok(())
    }
}

/// Which managed client a configuration entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientRole {
    /// The platform's own client.
    Platform,
    /// The one installed app's client. Every further app registers itself
    /// through the directory's Apps API.
    App,
}
