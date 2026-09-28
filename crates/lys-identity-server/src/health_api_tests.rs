#![cfg(test)]

use std::collections::BTreeSet;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use identity_contract::harness::{GRANT_MODEL, Service};
use serde_json::json;

use super::SERVICE;

type Outcome = Result<(), Box<dyn Error>>;

const PAGE: &str = "<html>the screens</html>";

/// A screens directory holding the page.
fn screens() -> Result<tempfile::TempDir, Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    std::fs::write(dir.path().join("index.html"), PAGE)?;
    Ok(dir)
}

/// A listener standing where `SpiceDB` would be, which counts each connection
/// it accepts, closes it at once, and says on `each` that it did.
struct Counting {
    address: String,
    accepted: Arc<AtomicUsize>,
    each: tokio::sync::mpsc::UnboundedReceiver<()>,
}

async fn counting() -> Result<Counting, Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?.to_string();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&accepted);
    let (said, each) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            counted.fetch_add(1, Ordering::SeqCst);
            drop(stream);
            if said.send(()).is_err() {
                break;
            }
        }
    });
    Ok(Counting {
        address,
        accepted,
        each,
    })
}

#[tokio::test]
async fn the_health_route_names_the_service_and_its_build_and_nothing_else() -> Outcome {
    let surface = screens()?;
    let served = surface.path().to_path_buf();
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        move |config| config.surface_dir = Some(served),
        |_| Ok(()),
    )
    .await?;

    let (status, body) = service.get("/api/health", None).await?;
    assert_eq!(status, 200);
    assert_eq!(body["service"], SERVICE);
    assert_eq!(body["service"], "lys-identity-server");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    let members = body
        .as_object()
        .ok_or("the health answer is not a JSON object")?
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(members, BTreeSet::from(["service", "version"]));
    Ok(())
}

#[tokio::test]
async fn the_health_route_asks_no_other_service() -> Outcome {
    let surface = screens()?;
    let served = surface.path().to_path_buf();
    let mut engine = counting().await?;
    let key_file = surface.path().join("spicedb.env");
    std::fs::write(&key_file, "SPICEDB_GRPC_PRESHARED_KEY=test-only-key\n")?;
    let spicedb = serde_json::from_value(json!({
        "endpoint": engine.address,
        "key_file": key_file,
    }))?;
    let (service, configured) = Service::start_adjusted(
        GRANT_MODEL,
        Some(spicedb),
        None,
        None,
        move |config| config.surface_dir = Some(served),
        |config| Ok(config.spicedb.clone()),
    )
    .await?;
    let configured = configured.map(|settings| settings.endpoint);
    assert_eq!(configured, Some(engine.address.clone()));

    let (status, body) = service.get("/api/health", None).await?;
    assert_eq!(status, 200);
    assert_eq!(body["service"], SERVICE);
    assert_eq!(engine.accepted.load(Ordering::SeqCst), 0);

    // The control: the listener counts a connection that is made to it.
    let made = tokio::net::TcpStream::connect(&engine.address).await?;
    engine.each.recv().await.ok_or("listener closed")?;
    drop(made);
    assert_eq!(engine.accepted.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn without_the_screens_the_health_route_is_at_the_root() -> Outcome {
    let service = Service::start().await?;
    let (status, body) = service.get("/health", None).await?;
    assert_eq!(status, 200);
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    Ok(())
}
