//! `lys identity prepare`: validate the configuration and materialise the
//! private files the three services read, each owner-only, from credentials
//! generated once and reused thereafter.

use std::path::{Path, PathBuf};

use base64::Engine as _;

use super::config::DeployConfig;
use super::credentials::{API_KEY_NAME, CREDENTIALS_FILE, Credentials, ENCRYPTION_KEY_ID};
use super::error::IdentityResult;
use super::private_files::{Outcome, ensure_private_dir, mode_of, write_private};

/// The private files `prepare` declares, relative to the venue, in the order
/// they are written. `credentials.json` is written first, by
/// [`Credentials::load_or_generate`].
pub const DECLARED_FILES: [&str; 6] = [
    CREDENTIALS_FILE,
    "compose.env",
    "postgres.env",
    "spicedb.env",
    "rauthy/config.toml",
    "rauthy/secrets.toml",
];

/// One written file, for the report.
#[derive(Debug, Clone)]
pub struct Written {
    /// The file, relative to the venue.
    pub name: &'static str,
    /// Its full path.
    pub path: PathBuf,
    /// What the write found and did.
    pub outcome: Outcome,
    /// Its mode bits after the write.
    pub mode: u32,
}

/// What `prepare` did.
#[derive(Debug, Clone)]
pub struct Prepared {
    /// The venue directory.
    pub venue: PathBuf,
    /// Whether the credentials were generated now or reused from the venue.
    pub credentials_generated: bool,
    /// Every declared file, in order.
    pub files: Vec<Written>,
}

/// Validate `config` and write the venue.
pub fn run(config: &DeployConfig) -> IdentityResult<Prepared> {
    ensure_private_dir(&config.venue)?;
    ensure_private_dir(&config.venue.join("rauthy"))?;
    let (credentials, generated) = Credentials::load_or_generate(&config.venue)?;
    let mut files = Vec::with_capacity(DECLARED_FILES.len());
    let credentials_path = config.venue.join(CREDENTIALS_FILE);
    files.push(Written {
        name: CREDENTIALS_FILE,
        path: credentials_path.clone(),
        outcome: if generated {
            Outcome::Created
        } else {
            Outcome::Unchanged
        },
        mode: mode_of(&credentials_path)?,
    });
    let renders: [(&'static str, String); 5] = [
        ("compose.env", compose_env(config)),
        ("postgres.env", postgres_env(config, &credentials)),
        ("spicedb.env", spicedb_env(config, &credentials)),
        ("rauthy/config.toml", rauthy_config(config, &credentials)),
        ("rauthy/secrets.toml", rauthy_secrets(&credentials)),
    ];
    for (name, text) in renders {
        let path = config.venue.join(name);
        let outcome = write_private(&path, text.as_bytes())?;
        let mode = mode_of(&path)?;
        files.push(Written {
            name,
            path,
            outcome,
            mode,
        });
    }
    Ok(Prepared {
        venue: config.venue.clone(),
        credentials_generated: generated,
        files,
    })
}

fn compose_env(config: &DeployConfig) -> String {
    format!(
        "LYS_IDENTITY_VENUE={}\nLYS_IDENTITY_PG_PORT={}\nLYS_IDENTITY_RAUTHY_PORT={}\nLYS_IDENTITY_SPICEDB_GRPC_PORT={}\nLYS_IDENTITY_SPICEDB_HTTP_PORT={}\n",
        absolute(&config.venue).display(),
        config.ports.postgres,
        config.ports.rauthy,
        config.spicedb.grpc_port,
        config.spicedb.http_port,
    )
}

fn postgres_env(config: &DeployConfig, credentials: &Credentials) -> String {
    format!(
        "POSTGRES_DB={}\nPOSTGRES_USER={}\nPOSTGRES_PASSWORD={}\nRAUTHY_PG_PASSWORD={}\nSPICEDB_PG_PASSWORD={}\n",
        config.database.name,
        config.database.admin_role,
        credentials.postgres_admin_password.expose(),
        credentials.rauthy_db_password.expose(),
        credentials.spicedb_db_password.expose(),
    )
}

fn spicedb_env(config: &DeployConfig, credentials: &Credentials) -> String {
    format!(
        "SPICEDB_DATASTORE_ENGINE=postgres\nSPICEDB_DATASTORE_CONN_URI=postgres://spicedb:{}@{}:{}/{}?sslmode=disable\nSPICEDB_GRPC_PRESHARED_KEY={}\n",
        credentials.spicedb_db_password.expose(),
        config.database.host,
        config.database.port,
        config.database.name,
        credentials.spicedb_preshared_key.expose(),
    )
}

/// The API key declaration Rauthy's bootstrap reads: clients read, create
/// and update only, expiring at the end of 2099.
fn api_key_declaration() -> String {
    let json = format!(
        "{{\"name\":\"{API_KEY_NAME}\",\"exp\":4102444800,\"access\":[{{\"group\":\"Clients\",\"access_rights\":[\"read\",\"create\",\"update\"]}}]}}"
    );
    base64::engine::general_purpose::STANDARD.encode(json)
}

fn rauthy_config(config: &DeployConfig, credentials: &Credentials) -> String {
    format!(
        "[database]\nhiqlite = false\npg_host = '{host}'\npg_port = {port}\npg_user = '$SECRETS'\npg_password = '$SECRETS'\npg_db_name = '{db}'\npg_tls_no_verify = true\n\n[server]\nlisten_address = '0.0.0.0'\nport_http = 8080\nscheme = '{scheme}'\npub_url = '{pub_url}'\n\n[cluster]\nnode_id = 1\nnodes = [\"1 localhost:8100 localhost:8200\"]\nsecret_raft = '$SECRETS'\nsecret_api = '$SECRETS'\npassword_dashboard = '$SECRETS'\n\n[encryption]\nkeys = ['$SECRETS']\nkey_active = '$SECRETS'\n\n[webauthn]\nrp_id = '{rp_id}'\nrp_origin = '{rp_origin}'\n\n[bootstrap]\nadmin_email = '{admin}'\npassword_plain = '{admin_password}'\napi_key = '{api_key}'\napi_key_secret = '{api_key_secret}'\n",
        host = config.database.host,
        port = config.database.port,
        db = config.database.name,
        scheme = config.rauthy.scheme,
        pub_url = config.rauthy.public_url,
        rp_id = config.rauthy.rp_id,
        rp_origin = config.rauthy.rp_origin,
        admin = config.rauthy.admin_email,
        admin_password = credentials.rauthy_admin_password.expose(),
        api_key = api_key_declaration(),
        api_key_secret = credentials.rauthy_api_key_secret.expose(),
    )
}

fn rauthy_secrets(credentials: &Credentials) -> String {
    format!(
        "[database]\npg_user = 'rauthy'\npg_password = '{db}'\n\n[cluster]\nsecret_raft = '{raft}'\nsecret_api = '{api}'\npassword_dashboard = '{dashboard}'\n\n[encryption]\nkeys = ['{key_id}/{key}']\nkey_active = '{key_id}'\n",
        db = credentials.rauthy_db_password.expose(),
        raft = credentials.rauthy_raft_secret.expose(),
        api = credentials.rauthy_api_secret.expose(),
        dashboard = base64::engine::general_purpose::STANDARD
            .encode(credentials.rauthy_dashboard_password.expose()),
        key_id = ENCRYPTION_KEY_ID,
        key = credentials.rauthy_encryption_key.expose(),
    )
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
}
