//! The identity server's one `SpiceDB` client: `SpiceDB`'s v1 gRPC API over
//! tonic and prost, with TLS by rustls on the pure-Rust `RustCrypto`
//! provider for an `https://` address.
//!
//! The client carries requests and answers; it builds no request that writes.
//! The schema write is built in `schema`, the relationship writes in
//! `projector`, the checks in `check` and the lookups in `lookup`; each hands
//! its request to the one operation here that sends it. Every call presents
//! the configured preshared key, which never appears in `Debug` output, a log
//! line or an error. A call `SpiceDB` did not answer is a [`SpiceDbError`]
//! naming the operation and the address, never an answer the client made up;
//! a fresh store's `NOT_FOUND` answer to a schema read is the typed
//! [`SchemaRead::NoSchema`], neither an error nor an empty schema.
//!
//! The server's decisions are synchronous, so the client runs its channel on
//! a runtime of its own and each operation waits for its answer. Nothing
//! waits on a clock: a call ends when `SpiceDB` answers or the transport
//! fails.

use std::future::Future;
use std::sync::Arc;

use tonic::codegen::http::uri::PathAndQuery;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::{Channel, Endpoint, Uri};
use tonic::{Code, Request, Status};
use zeroize::Zeroizing;

use super::check::{CheckAnswer, CheckQuestion};
use super::error::SpiceDbError;
use super::gateway::SpiceDbSettings;
use super::lookup::{LookupAnswer, LookupQuestion};
use super::projector::{RelationshipsWrite, RelationshipsWritten};
use super::schema::{SchemaWrite, SchemaWritten};
use super::wire::authzed::api::v1::{
    ReadRelationshipsRequest, ReadRelationshipsResponse, ReadSchemaRequest, ReadSchemaResponse,
};

/// The line of an environment file that holds the preshared key.
const KEY_LINE: &str = "SPICEDB_GRPC_PRESHARED_KEY=";

const READ_SCHEMA: &str = "/authzed.api.v1.SchemaService/ReadSchema";
const WRITE_SCHEMA: &str = "/authzed.api.v1.SchemaService/WriteSchema";
const WRITE_RELATIONSHIPS: &str = "/authzed.api.v1.PermissionsService/WriteRelationships";
const READ_RELATIONSHIPS: &str = "/authzed.api.v1.PermissionsService/ReadRelationships";
const CHECK: &str = "/authzed.api.v1.PermissionsService/CheckPermission";
const LOOKUP: &str = "/authzed.api.v1.PermissionsService/LookupSubjects";

/// Where `SpiceDB` is and the preshared key it is reached with.
pub struct ClientConfig {
    address: String,
    preshared_key: Zeroizing<String>,
}

impl std::fmt::Debug for ClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientConfig")
            .field("address", &self.address)
            .field("preshared_key", &"<redacted>")
            .finish()
    }
}

impl ClientConfig {
    /// `SpiceDB` at `address`, `http://host:port` or `https://host:port`,
    /// reached with `preshared_key`.
    pub fn new(address: impl Into<String>, preshared_key: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            preshared_key: Zeroizing::new(preshared_key.into()),
        }
    }

    /// The client the server's configuration names: the gRPC address, and
    /// the preshared key from its key file, alone or as the
    /// `SPICEDB_GRPC_PRESHARED_KEY=` line of an environment file.
    pub fn from_settings(settings: &SpiceDbSettings) -> Result<Self, SpiceDbError> {
        let address = settings
            .grpc
            .clone()
            .ok_or_else(|| SpiceDbError::ConfigInvalid {
                reason: "the configuration names no gRPC address for SpiceDB".to_owned(),
            })?;
        let text = Zeroizing::new(std::fs::read_to_string(&settings.key_file).map_err(
            |error| SpiceDbError::ConfigInvalid {
                reason: format!(
                    "SpiceDB's key file {} could not be read: {}",
                    settings.key_file.display(),
                    error.kind()
                ),
            },
        )?);
        let key = text
            .lines()
            .find_map(|line| line.strip_prefix(KEY_LINE))
            .unwrap_or(&text)
            .trim();
        if key.is_empty() {
            return Err(SpiceDbError::ConfigInvalid {
                reason: format!(
                    "SpiceDB's key file {} is empty",
                    settings.key_file.display()
                ),
            });
        }
        Ok(Self::new(address, key))
    }

    /// The configured address.
    pub fn address(&self) -> &str {
        &self.address
    }
}

/// What a schema read found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaRead {
    /// The store holds no schema: `SpiceDB` answered `NOT_FOUND`.
    NoSchema,
    /// The schema text the store holds.
    Schema(String),
}

/// The operations the identity server asks of `SpiceDB`. The client
/// implements them; a test wraps them to count and inspect every call.
pub trait SpiceDbApi: Send + Sync {
    /// The configured address.
    fn address(&self) -> &str;

    /// Read the stored schema.
    fn read_schema(&self) -> Result<SchemaRead, SpiceDbError>;

    /// Send a schema write built by the schema module.
    fn write_schema(&self, request: SchemaWrite) -> Result<SchemaWritten, SpiceDbError>;

    /// Send a relationship write built by the projector.
    fn write_relationships(
        &self,
        request: RelationshipsWrite,
    ) -> Result<RelationshipsWritten, SpiceDbError>;

    /// Read the relationships a filter names, every one of them.
    fn read_relationships(
        &self,
        request: ReadRelationshipsRequest,
    ) -> Result<Vec<ReadRelationshipsResponse>, SpiceDbError>;

    /// Send a permission check built by the one evaluator.
    fn check(&self, request: CheckQuestion) -> Result<CheckAnswer, SpiceDbError>;

    /// Send a subject lookup built by the lookup module, and read every answer.
    fn lookup(&self, request: LookupQuestion) -> Result<Vec<LookupAnswer>, SpiceDbError>;
}

/// The client: one channel to `SpiceDB` on a runtime of its own.
pub struct SpiceDbClient {
    address: String,
    authorization: MetadataValue<Ascii>,
    channel: Channel,
    runtime: Option<tokio::runtime::Runtime>,
}

impl std::fmt::Debug for SpiceDbClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiceDbClient")
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

impl Drop for SpiceDbClient {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

fn invalid(reason: String) -> SpiceDbError {
    SpiceDbError::ConfigInvalid { reason }
}

/// The TLS connection to an `https://` address, on the `RustCrypto` provider
/// and the web PKI's roots, speaking HTTP/2.
fn tls_channel(endpoint: &Endpoint) -> Result<Channel, SpiceDbError> {
    let provider = Arc::new(rustls_rustcrypto::provider());
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let mut config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| invalid(format!("TLS to SpiceDB cannot be set up: {error}")))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec()];
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let connect = tower::service_fn(move |uri: Uri| {
        let connector = connector.clone();
        async move {
            let host = uri
                .host()
                .ok_or_else(|| std::io::Error::other("the address names no host"))?
                .to_owned();
            let port = uri.port_u16().unwrap_or(443);
            let tcp = tokio::net::TcpStream::connect((host.as_str(), port)).await?;
            let name = rustls::pki_types::ServerName::try_from(host)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let tls = connector.connect(name, tcp).await?;
            Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(tls))
        }
    });
    Ok(endpoint.connect_with_connector_lazy(connect))
}

impl SpiceDbClient {
    /// A client for `config`. Nothing is sent until the first operation.
    pub fn connect(config: &ClientConfig) -> Result<Self, SpiceDbError> {
        let mut authorization: MetadataValue<Ascii> =
            format!("Bearer {}", config.preshared_key.as_str())
                .parse()
                .map_err(|error| {
                    invalid(format!(
                        "the preshared key is not a gRPC header value: {error}"
                    ))
                })?;
        authorization.set_sensitive(true);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("lys-spicedb")
            .enable_all()
            .build()
            .map_err(|error| invalid(format!("the SpiceDB client's runtime: {error}")))?;
        let endpoint = Endpoint::from_shared(config.address.clone()).map_err(|error| {
            invalid(format!(
                "{} is not a SpiceDB address: {error}",
                config.address
            ))
        })?;
        let entered = runtime.enter();
        let channel = match endpoint.uri().scheme_str() {
            Some("https") => tls_channel(&endpoint),
            Some("http") => Ok(endpoint.connect_lazy()),
            _ => Err(invalid(format!(
                "{} is not an http:// or https:// address",
                config.address
            ))),
        };
        drop(entered);
        Ok(Self {
            address: config.address.clone(),
            authorization,
            channel: channel?,
            runtime: Some(runtime),
        })
    }

    fn request<M>(&self, message: M) -> Request<M> {
        let mut request = Request::new(message);
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request
    }

    fn failed(&self, operation: &'static str, status: &Status) -> SpiceDbError {
        if status.code() == Code::Unavailable {
            SpiceDbError::Unreachable {
                operation,
                address: self.address.clone(),
                reason: status.message().to_owned(),
            }
        } else {
            SpiceDbError::Refused {
                operation,
                address: self.address.clone(),
                code: format!("{:?}", status.code()),
                message: status.message().to_owned(),
            }
        }
    }

    /// Run `call` on the client's runtime and wait for its answer.
    fn wait<T: Send + 'static>(
        &self,
        operation: &'static str,
        call: impl Future<Output = Result<T, Status>> + Send + 'static,
    ) -> Result<T, SpiceDbError> {
        let unreachable = |reason: String| SpiceDbError::Unreachable {
            operation,
            address: self.address.clone(),
            reason,
        };
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| unreachable("the client's runtime has stopped".to_owned()))?;
        let (send, receive) = std::sync::mpsc::channel();
        runtime.spawn(async move {
            if send.send(call.await).is_err() {
                tracing::warn!(
                    operation,
                    "a SpiceDB answer arrived after its caller stopped waiting"
                );
            }
        });
        let answer = receive
            .recv()
            .map_err(|error| unreachable(format!("the call ended without an answer: {error}")))?;
        answer.map_err(|status| self.failed(operation, &status))
    }

    /// The unary call of `path` with `message`, to be run on the runtime.
    fn call<Q, A>(
        &self,
        path: &'static str,
        message: Q,
    ) -> impl Future<Output = Result<A, Status>> + Send + 'static
    where
        Q: prost::Message + Send + Sync + 'static,
        A: prost::Message + Default + Send + Sync + 'static,
    {
        let mut grpc = tonic::client::Grpc::new(self.channel.clone());
        let request = self.request(message);
        async move {
            grpc.ready()
                .await
                .map_err(|error| Status::unavailable(error.to_string()))?;
            let codec = tonic_prost::ProstCodec::<Q, A>::default();
            let answer = grpc
                .unary(request, PathAndQuery::from_static(path), codec)
                .await?;
            Ok(answer.into_inner())
        }
    }

    fn unary<Q, A>(
        &self,
        operation: &'static str,
        path: &'static str,
        message: Q,
    ) -> Result<A, SpiceDbError>
    where
        Q: prost::Message + Send + Sync + 'static,
        A: prost::Message + Default + Send + Sync + 'static,
    {
        self.wait(operation, self.call(path, message))
    }

    fn streaming<Q, A>(
        &self,
        operation: &'static str,
        path: &'static str,
        message: Q,
    ) -> Result<Vec<A>, SpiceDbError>
    where
        Q: prost::Message + Send + Sync + 'static,
        A: prost::Message + Default + Send + Sync + 'static,
    {
        let mut grpc = tonic::client::Grpc::new(self.channel.clone());
        let request = self.request(message);
        self.wait(operation, async move {
            grpc.ready()
                .await
                .map_err(|error| Status::unavailable(error.to_string()))?;
            let codec = tonic_prost::ProstCodec::<Q, A>::default();
            let mut answers = grpc
                .server_streaming(request, PathAndQuery::from_static(path), codec)
                .await?
                .into_inner();
            let mut every = Vec::new();
            while let Some(answer) = answers.message().await? {
                every.push(answer);
            }
            Ok(every)
        })
    }
}

impl SpiceDbApi for SpiceDbClient {
    fn address(&self) -> &str {
        &self.address
    }

    fn read_schema(&self) -> Result<SchemaRead, SpiceDbError> {
        let read =
            self.call::<ReadSchemaRequest, ReadSchemaResponse>(READ_SCHEMA, ReadSchemaRequest {});
        self.wait("ReadSchema", async move {
            match read.await {
                Ok(answer) => Ok(SchemaRead::Schema(answer.schema_text)),
                Err(status) if status.code() == Code::NotFound => Ok(SchemaRead::NoSchema),
                Err(status) => Err(status),
            }
        })
    }

    fn write_schema(&self, request: SchemaWrite) -> Result<SchemaWritten, SpiceDbError> {
        self.unary("WriteSchema", WRITE_SCHEMA, request)
    }

    fn write_relationships(
        &self,
        request: RelationshipsWrite,
    ) -> Result<RelationshipsWritten, SpiceDbError> {
        self.unary("WriteRelationships", WRITE_RELATIONSHIPS, request)
    }

    fn read_relationships(
        &self,
        request: ReadRelationshipsRequest,
    ) -> Result<Vec<ReadRelationshipsResponse>, SpiceDbError> {
        self.streaming("ReadRelationships", READ_RELATIONSHIPS, request)
    }

    fn check(&self, request: CheckQuestion) -> Result<CheckAnswer, SpiceDbError> {
        self.unary("CheckPermission", CHECK, request)
    }

    fn lookup(&self, request: LookupQuestion) -> Result<Vec<LookupAnswer>, SpiceDbError> {
        self.streaming("LookupSubjects", LOOKUP, request)
    }
}
