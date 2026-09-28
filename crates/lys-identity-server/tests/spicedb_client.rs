#![cfg(test)]
//! R1: the one `SpiceDB` client against a disposable `SpiceDB`. A fresh
//! store answers a schema read `NOT_FOUND` and the client reads exactly
//! `SchemaRead::NoSchema`; two tests' stores are their own; an unreachable
//! engine is named with its address and never with the preshared key.

mod spicedb_support;

use lys_identity::grants::ObjectRef;
use lys_identity_server::spicedb::schema::{Started, ensure};
use lys_identity_server::spicedb::wire::authzed::api::v1::{ReadSchemaRequest, ReadSchemaResponse};
use lys_identity_server::spicedb::{
    ClientConfig, SchemaRead, SpiceDbApi, SpiceDbClient, SpiceDbError,
};
use spicedb_support::server::{SpiceDb, unique};
use spicedb_support::{TestResult, stored, write_one};
use tonic::codegen::http::uri::PathAndQuery;

/// The preshared key the redaction tests present.
const FIXTURE_KEY: &str = "spicedb-test-key-fixture";

#[tokio::test]
async fn a_fresh_store_answers_not_found_and_the_client_reads_no_schema() -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let key = unique("spicedb-test-key");
    let channel = tonic::transport::Endpoint::from_shared(spicedb.address.clone())?
        .connect()
        .await?;
    let mut grpc = tonic::client::Grpc::new(channel);
    grpc.ready().await?;
    let mut request = tonic::Request::new(ReadSchemaRequest {});
    request
        .metadata_mut()
        .insert("authorization", format!("Bearer {key}").parse()?);
    let answered = grpc
        .unary(
            request,
            PathAndQuery::from_static("/authzed.api.v1.SchemaService/ReadSchema"),
            tonic_prost::ProstCodec::<ReadSchemaRequest, ReadSchemaResponse>::default(),
        )
        .await;
    let status = answered
        .err()
        .ok_or("a fresh store answered a schema read")?;
    assert_eq!(status.code(), tonic::Code::NotFound, "{status:?}");

    let client = SpiceDbClient::connect(&ClientConfig::new(&spicedb.address, key))?;
    assert_eq!(client.read_schema()?, SchemaRead::NoSchema);
    Ok(())
}

/// Write one relationship naming `name` to a store of its own and read back
/// exactly that one.
fn own_store_holds_only_its_own(name: &str) -> TestResult {
    let spicedb = SpiceDb::start(None)?;
    let client = spicedb.client()?;
    assert_eq!(ensure(client.as_ref())?, Started::Written);
    let resource = ObjectRef {
        kind: "resource".to_owned(),
        id: format!("project/{name}/read"),
    };
    let grant = ObjectRef {
        kind: "grant".to_owned(),
        id: format!("grant-{name}"),
    };
    write_one(client.as_ref(), (&resource, "granted", &grant))?;
    assert_eq!(
        stored(client.as_ref())?,
        vec![format!(
            "resource:project/{name}/read#granted@grant:grant-{name}"
        )]
    );
    Ok(())
}

#[test]
fn the_first_of_two_parallel_tests_reads_back_only_its_own_relationship() -> TestResult {
    own_store_holds_only_its_own("first")
}

#[test]
fn the_second_of_two_parallel_tests_reads_back_only_its_own_relationship() -> TestResult {
    own_store_holds_only_its_own("second")
}

#[test]
fn an_unreachable_engine_is_named_by_address_and_never_by_its_key() -> TestResult {
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    let address = format!("http://127.0.0.1:{port}");
    let client = SpiceDbClient::connect(&ClientConfig::new(&address, FIXTURE_KEY))?;
    let refused = client.read_schema();
    let Err(error) = refused else {
        return Err(format!("a closed port answered a schema read: {refused:?}").into());
    };
    assert!(
        matches!(error, SpiceDbError::Unreachable { .. }),
        "{error:?}"
    );
    let words = error.to_string();
    assert!(words.contains(&address), "{words}");
    assert!(!words.contains(FIXTURE_KEY), "{words}");
    assert!(!format!("{error:?}").contains(FIXTURE_KEY));
    Ok(())
}

#[test]
fn the_preshared_key_is_not_in_debug_output() -> TestResult {
    let config = ClientConfig::new("http://127.0.0.1:50051", FIXTURE_KEY);
    let shown = format!("{config:?}");
    assert!(!shown.contains(FIXTURE_KEY), "{shown}");
    assert!(shown.contains("http://127.0.0.1:50051"), "{shown}");
    let client = SpiceDbClient::connect(&config)?;
    assert!(!format!("{client:?}").contains(FIXTURE_KEY));
    Ok(())
}
