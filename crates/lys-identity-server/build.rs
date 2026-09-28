//! Compiles the vendored authzed v1 API protocol files into the identity
//! server's `SpiceDB` message types with protox, so no protoc binary is
//! needed.
//!
//! Only the messages the permission and schema services use are compiled,
//! by prost: the client calls the services itself, through tonic, so no
//! generated client is needed. The protocol files' own comments stay in the
//! protocol files, where they are read; each generated item carries one line
//! naming where its meaning is written. The packages imported only for
//! validation and gateway options are generated beside the others and
//! included nowhere, because those options change no message the client
//! sends or reads.

use std::error::Error;
use std::path::PathBuf;

/// The files the messages are generated from.
const SERVICES: [&str; 2] = [
    "authzed/api/v1/permission_service.proto",
    "authzed/api/v1/schema_service.proto",
];

/// Where the imports are found: the authzed files, and beside them the files
/// they import.
const INCLUDES: [&str; 2] = ["proto", "proto/authzed/third_party"];

/// The line every generated item carries.
const DOC: &str = "#[doc = \"Generated from the authzed v1 API; its meaning is written in the protocol files under crates/lys-identity-server/proto/authzed/.\"]";

/// The generated files the server includes.
const INCLUDED: [&str; 2] = ["authzed.api.v1.rs", "google.rpc.rs"];

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=proto");
    let files = protox::compile(SERVICES, INCLUDES)?;
    prost_build::Config::new()
        .disable_comments(["."])
        .type_attribute(".", DOC)
        .field_attribute(".", DOC)
        // A bulk answer's item is many times the size of its error.
        .boxed(".authzed.api.v1.CheckBulkPermissionsPair.response.item")
        .compile_fds(files)?;
    let out = PathBuf::from(std::env::var("OUT_DIR")?);
    for name in INCLUDED {
        let path = out.join(name);
        // prost names the protocol format in the words of the enum helpers
        // it writes; the name is code, so it is written as code.
        let text = std::fs::read_to_string(&path)?.replace("ProtoBuf", "`ProtoBuf`");
        std::fs::write(&path, text)?;
    }
    Ok(())
}
