//! Build service sections separately so configuration size does not expand one recursive macro.

use super::{
    Carried, DeploymentConfig, GRANT_LOG_ORIGIN, LINK_AUDIT_SUBJECT, LOG_ORIGIN, Layout,
    MESSAGE_SERVICE, OPERATOR_TOKEN_FILE, PROVIDER_KEY_FILE, PROVIDERS_KEY_FILE, SERVICE_NAME,
    SESSION_SECONDS, Value, issuer, json, redirect_url,
};

/// Render the service configuration while retaining the carried installation settings.
#[must_use]
pub fn render(
    layout: &Layout,
    config: &DeploymentConfig,
    carried: &Carried,
    surface: bool,
) -> Value {
    let issuer = issuer(config);
    let mut rendered = base_configuration(layout, config, carried);
    rendered["model_proxy"] = Value::String(carried.ports.proxy_url());
    // Where that proxy keeps its records, so the service can show a person an agent's calls.
    rendered["proxy_dir"] = Value::String(layout.data_dir().join("proxy").display().to_string());
    // When the issuer an earlier configuration named is not this one, the
    // sign-in service moved: the service is handed the earlier issuer and
    // records the move in the directory once, and the administrator's login
    // moves with every other. A move handed over before is handed over
    // again, which the directory finds already recorded.
    let moved_from = carried
        .issuer
        .clone()
        .filter(|earlier| *earlier != issuer)
        .or_else(|| carried.issuer_moved_from.clone())
        .filter(|earlier| *earlier != issuer);
    if let Some(administrator) = &carried.administrator {
        let mut administrator = administrator.clone();
        if moved_from.is_some() && administrator["issuer"].as_str() == moved_from.as_deref() {
            administrator["issuer"] = Value::String(issuer);
        }
        rendered["administrator"] = administrator;
    }
    if let Some(from) = moved_from {
        rendered["issuer_moved_from"] = Value::String(from);
    }
    if let Some(bridge) = &carried.message_service {
        rendered[MESSAGE_SERVICE] = bridge.clone();
    }
    if let Some(proxies) = &carried.trusted_proxies {
        rendered["trusted_proxies"] = proxies.clone();
    }
    if let Some(membership) = &carried.membership {
        rendered["membership"] = membership.clone();
    }
    // The installer and upgrade share this renderer. Only an absent key
    // gets 30; a malformed written value remains for the service to refuse.
    let mut reviews = carried.reviews.clone().unwrap_or_else(|| json!({}));
    if let Some(settings) = reviews.as_object_mut() {
        settings.entry("period_days").or_insert(json!(30));
    }
    rendered["reviews"] = reviews;
    if let Some(cap) = &carried.rights_bytes {
        rendered["provider"]["rights_bytes"] = cap.clone();
    }
    if surface {
        rendered["surface_dir"] = Value::String(layout.surface_dir().display().to_string());
    }
    if let Some(mail) = &config.mail {
        // The password stays in its file (DIRECTORY-110 R1); the record of
        // letters sent sits beside the directory's data.
        let password_file = mail.user.as_ref().map(|_| {
            config
                .state_dir()
                .join(crate::identity::prepare::MAIL_PASSWORD_FILE)
                .display()
                .to_string()
        });
        rendered["mail"] = json!({
            "host": mail.host,
            "port": mail.port,
            "user": mail.user,
            "password_file": password_file,
            "from": mail.from,
            "tls": mail.tls.word(),
            "sent_file": layout.data_dir().join("letters-sent.jsonl").display().to_string(),
        });
    }
    rendered
}

fn base_configuration(layout: &Layout, config: &DeploymentConfig, carried: &Carried) -> Value {
    let issuer = issuer(config);
    let state = config.state_dir();
    let data = layout.data_dir();
    let policy = &config.password_policy;
    let dir = |name: &str| Value::String(data.join(name).display().to_string());
    Value::Object([
        ("listen", json!(format!("127.0.0.1:{}", carried.ports.service))),
        ("log_dir", json!(dir("directory-log"))),
        ("log_origin", json!(LOG_ORIGIN)),
        ("event_key_file", json!(layout.service_key().display().to_string())),
        ("profile", json!(carried.profile.unwrap_or_default().word())),
        ("operator_token_file", json!(match carried.profile.unwrap_or_default() {
            super::super::Profile::Development => Value::String(state.join(OPERATOR_TOKEN_FILE).display().to_string()),
            super::super::Profile::Service => Value::Null,
        })),
        ("operator_upgrade_file", json!(layout.upgrade_intent().display().to_string())),
        ("import_credential_file", json!(super::super::super::import::credential_path(layout).display().to_string())),
        ("issuer", json!(issuer)),
        ("client_id", json!(config.clients.platform.id)),
        ("client_secret_file", json!(state
            .join(format!("{}-client-secret", config.clients.platform.id))
            .display()
            .to_string())),
        ("redirect_url", json!(redirect_url(config, carried.ports))),
        ("sign_in_api", json!(format!("{}/auth/v1", config.issuer.admin_url.trim_end_matches('/')))),
        ("setup", json!({
            "code_file": state.join(super::super::setup_code::CODE_FILE).display().to_string(),
            "administrator_file": layout.administrator_file().display().to_string(),
            "email": config.deployment.admin_email,
            // The install's own organisation, founded at setup (DIRECTORY-109 R2).
            "organisation": config.deployment.project,
        })),
        ("password_policy", json!({
            "length_min": policy.length_min,
            "length_max": policy.length_max,
            "lower_case": policy.lower_case,
            "upper_case": policy.upper_case,
            "digits": policy.digits,
            "special": policy.special,
            "not_recently_used": policy.not_recently_used,
        })),
        ("link_audit_source", json!({"issuer": issuer, "subject": LINK_AUDIT_SUBJECT})),
        ("session_seconds", json!(SESSION_SECONDS)),
        ("sessions_file", json!(state.join("sessions.json").display().to_string())),
        ("secure_cookie", json!(false)),
        ("grant_log_dir", json!(dir("grant-log"))),
        ("grant_log_origin", json!(GRANT_LOG_ORIGIN)),
        ("grant_model_file", json!(layout.grant_model().display().to_string())),
        ("spicedb", json!({
            "endpoint": format!("127.0.0.1:{}", config.spicedb.http_port),
            "key_file": state.join("spicedb-preshared-key").display().to_string(),
        })),
        ("secrets", json!({
            "broker": format!("http://127.0.0.1:{}", carried.ports.broker),
            "service": SERVICE_NAME,
            "service_key_file": layout.service_key().display().to_string(),
        })),
        ("sign_in_providers", json!({
            "api": format!("{}/auth/v1", config.issuer.admin_url.trim_end_matches('/')),
            "api_key_file": state.join(PROVIDERS_KEY_FILE).display().to_string(),
        })),
        ("provider", json!({
            "key_file": state.join(PROVIDER_KEY_FILE).display().to_string(),
            "pass_seconds": config.identity.pass_lifetime,
        })),
        ("requests_dir", json!(dir("requests"))),
        ("certificates_dir", json!(dir("certificates"))),
        ("network_file", json!(dir("network.json"))),
        ("roles_file", json!(dir("roles.json"))),
        ("provisioning_file", json!(dir("provisioning.json"))),
        ("homes_dir", json!(dir("homes"))),
        ("runtime_dir", json!(dir("runtime"))),
        ("service_accounts_dir", json!(dir("service-accounts"))),
        ("teams_dir", json!(dir("teams"))),
        ("budgets_dir", json!(dir("budgets"))),
        ("policies_dir", json!(dir("policies"))),
        ("stops_dir", json!(dir("stops"))),
        ("goals_dir", json!(dir("goals"))),
        ("words_dir", json!(dir("words"))),
        ("variables_dir", json!(dir("variables"))),
        ("schedules_dir", json!(dir("schedules"))),
        ("reviews_dir", json!(dir("reviews"))),
        ("runner_socket", json!(layout.runner_socket().display().to_string())),
        // The stores receipt beside the build record, outside data/, and the
        // operator's acceptances beside it (DIRECTORY-107 R2).
        ("stores_receipt_file", json!(layout.stores_receipt().display().to_string())),
        ("stores_accepted_file", json!(layout.stores_accepted().display().to_string())),
    ].into_iter().map(|(key, value)| (key.to_owned(), value)).collect())
}
