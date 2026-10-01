#![cfg(test)]
//! A prepared service retains private state and measures its actual startup writes.

use identity_contract::harness::Service;

#[tokio::test]
async fn prepared_service_startup_flushes() -> Result<(), Box<dyn std::error::Error>> {
    let service = Service::start().await?;
    eprintln!(
        "prepared service startup log-store flushes: {}",
        service.startup_flushes
    );
    assert_eq!(
        service.startup_flushes, 21,
        "17 leaf-directory opens and four migration snapshot flushes"
    );
    Ok(())
}
