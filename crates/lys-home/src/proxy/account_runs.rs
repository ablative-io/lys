//! Signed run bindings hold proxy draws outside every child's launch.
use super::{
    account::{Broker, Drawn, placeholder},
    error::ProxyError,
    forward::{Base, ProxyBody, Upstream, refusal},
};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Incoming};
use lys_core::Ed25519Identity;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Draw {
    pub source: String,
    secret: String,
    id: String,
    token: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub run: String,
    broker: String,
    pub provider: String,
    pub draws: Vec<Draw>,
}
pub(super) struct Accounts {
    directory: PathBuf,
    key: Arc<Ed25519Identity>,
}
impl std::fmt::Debug for Accounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Accounts")
            .field("directory", &self.directory)
            .finish_non_exhaustive()
    }
}
fn failed(reason: &str) -> ProxyError {
    ProxyError::ModelAccount {
        account: "run".to_owned(),
        reason: reason.to_owned(),
    }
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    text
}
fn loopback(text: &str) -> Result<Base, ProxyError> {
    let uri: hyper::Uri = text
        .parse()
        .map_err(|error| failed(&format!("account broker URL invalid: {error}")))?;
    if uri.scheme_str() != Some("http")
        || !matches!(uri.host(), Some("127.0.0.1" | "localhost" | "[::1]"))
        || uri.path_and_query().is_some_and(|p| p.as_str() != "/")
    {
        return Err(failed("account broker must be an HTTP loopback origin"));
    }
    Base::parse(text)
}
impl Accounts {
    pub fn open(state: &Path) -> Result<Self, ProxyError> {
        let directory = state.join("accounts");
        std::fs::create_dir_all(&directory)
            .map_err(|e| ProxyError::io("creating run accounts", &directory, e))?;
        let key = Ed25519Identity::load_or_generate(&directory.join("proxy.key"))
            .map_err(|error| failed(&format!("account proxy key unavailable: {error}")))?;
        Ok(Self {
            directory,
            key: Arc::new(key),
        })
    }
    pub fn public_key(&self) -> Response<ProxyBody> {
        refusal(StatusCode::OK, &hex(&self.key.public_key_bytes()))
    }
    pub fn binding(&self, run: Option<&str>) -> Result<Option<Binding>, ProxyError> {
        let Some(run) = run else { return Ok(None) };
        let path = self.directory.join(format!("{run}.json"));
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ProxyError::io("reading run account", &path, e)),
        };
        let binding: Binding = serde_json::from_slice(&bytes).map_err(|error| {
            failed(&format!(
                "stored run account binding invalid at line {}, column {}",
                error.line(),
                error.column()
            ))
        })?;
        if binding.run != run {
            return Err(failed("stored account run differs"));
        }
        Ok(Some(binding))
    }
    pub async fn bind(
        &self,
        upstream: &Upstream,
        request: Request<Incoming>,
    ) -> Result<Response<ProxyBody>, ProxyError> {
        let (parts, body) = request.into_parts();
        let body = body
            .collect()
            .await
            .map_err(|error| failed(&format!("account binding body unreadable: {error}")))?
            .to_bytes();
        let asked: serde_json::Value = serde_json::from_slice(&body).map_err(|error| {
            failed(&format!(
                "account binding malformed at line {}, column {}",
                error.line(),
                error.column()
            ))
        })?;
        let text = |name: &str| {
            asked
                .get(name)
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| failed("account binding field absent"))
        };
        if text("proxy_key")? != hex(&self.key.public_key_bytes())
            || !super::usage::is_run_key(text("run")?)
        {
            return Err(failed("account binding key or run differs"));
        }
        let base = loopback(text("broker")?)?;
        let body = Full::new(body)
            .map_err(|never: std::convert::Infallible| match never {})
            .boxed_unsync();
        let response = upstream
            .send(
                &base,
                "/_lys/account/bind",
                Request::from_parts(parts, body),
            )
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Ok(response.map(BodyExt::boxed_unsync));
        }
        let bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|error| failed(&format!("broker account binding unreadable: {error}")))?
            .to_bytes();
        let binding: Binding = serde_json::from_slice(&bytes).map_err(|error| {
            failed(&format!(
                "broker account binding malformed at line {}, column {}",
                error.line(),
                error.column()
            ))
        })?;
        if binding.run != text("run")?
            || binding.broker != text("broker")?
            || binding.provider != text("provider")?
        {
            return Err(failed("broker account binding differs"));
        }
        let path = self.directory.join(format!("{}.json", binding.run));
        let temporary = path.with_extension("pending");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|e| ProxyError::io("creating run account binding", &temporary, e))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| ProxyError::io("writing run account binding", &temporary, e))?;
        std::fs::rename(&temporary, &path)
            .map_err(|e| ProxyError::io("placing run account binding", &path, e))?;
        std::fs::File::open(&self.directory)
            .and_then(|d| d.sync_all())
            .map_err(|e| ProxyError::io("syncing run accounts", &self.directory, e))?;
        Ok(refusal(StatusCode::OK, &binding.run))
    }
    pub fn drawn(
        &self,
        binding: &Binding,
        headers: &hyper::HeaderMap,
    ) -> Result<(Broker, Drawn, String), ProxyError> {
        let selector = headers
            .get("authorization")
            .or_else(|| headers.get("x-api-key"))
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| failed("account selector absent"))?;
        let source = selector
            .strip_prefix("Bearer ")
            .unwrap_or(selector)
            .strip_prefix("lys-account.")
            .ok_or_else(|| failed("account selector invalid"))?;
        let draw = binding
            .draws
            .iter()
            .find(|d| d.source == source)
            .ok_or_else(|| failed("account selector unheld by run"))?;
        let mut carried = hyper::HeaderMap::new();
        let value = hyper::header::HeaderValue::from_str(&format!(
            "Bearer {}",
            placeholder(&draw.secret, &draw.id, &draw.token)
        ))
        .map_err(|error| failed(&format!("account draw invalid: {error}")))?;
        carried.insert("authorization", value);
        let drawn = Drawn::carried(&carried)?.ok_or_else(|| failed("account draw absent"))?;
        Ok((
            Broker {
                base: loopback(&binding.broker)?,
                key: Arc::clone(&self.key),
            },
            drawn,
            source.to_owned(),
        ))
    }
}
