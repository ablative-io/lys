#![cfg(test)]
//! A prepared service retains private state and measures its actual startup writes.

use identity_contract::harness::Service;

#[tokio::test]
async fn prepared_service_startup_flushes() -> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let service = Service::start().await?;
    eprintln!(
        "complete service setup, including cache build or lookup: {:?}",
        started.elapsed()
    );
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

#[tokio::test]
async fn preparation_keeps_its_signed_directory() -> Result<(), Box<dyn std::error::Error>> {
    use identity_contract::fixtures::{administrator, op, shown};
    use lys_identity_server::routes::open_directory;
    use lys_log_store::{FileLeafStore, LeafStore};

    let (service, leaf) = Service::start_with(|config| {
        let mut directory = open_directory(config)?;
        directory.register_person(administrator()?, op(1), shown("Prepared"), 1)?;
        let store = FileLeafStore::open(&config.log_dir)?;
        Ok(store.leaf(0)?.ok_or("prepared leaf missing")?)
    })
    .await?;
    let store = FileLeafStore::open(&service.dir.path().join("log"))?;
    assert_eq!(store.extent(), 1);
    assert_eq!(store.leaf(0)?.ok_or("served leaf missing")?, leaf);
    Ok(())
}
