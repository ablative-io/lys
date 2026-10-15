//! Records build provenance through the shared `build/provenance.rs`, byte-identical in every stamped crate, after
//! compiling the device-owner presence helper this crate alone carries.

#[path = "build/presence.rs"]
mod presence;
#[path = "build/provenance.rs"]
mod provenance;

fn main() -> Result<(), String> {
    let dir =
        std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| ".".into(), std::path::PathBuf::from);
    presence::compile(&dir)?;
    provenance::main()
}
