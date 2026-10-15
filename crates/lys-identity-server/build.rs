//! Records build provenance through the shared `build/provenance.rs`, byte-identical in every stamped crate.

#[path = "build/provenance.rs"]
mod provenance;

fn main() -> Result<(), String> {
    provenance::main()
}
