//! Stamps the commit this crate is built from into `LYS_BUILD`, through the
//! one build stamp every Lys binary shares (`crates/lys-build-stamp`).

fn main() {
    lys_build_stamp::emit();
}
