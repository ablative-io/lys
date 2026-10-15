//! Package the native prompt implementation inside the CLI.

use std::path::Path;

pub fn compile(manifest: &Path) -> Result<(), String> {
    let source = manifest.join("native/owner-presence.swift");
    println!("cargo:rerun-if-changed={}", source.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return Ok(());
    }
    let out = std::env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?;
    let executable = Path::new(&out).join("lys-owner-presence");
    let output = std::process::Command::new("/usr/bin/xcrun")
        .args(["swiftc", "-O"])
        .arg(&source)
        .args(["-framework", "LocalAuthentication", "-o"])
        .arg(executable)
        .output()
        .map_err(|error| format!("presence_helper_compile_failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "presence_helper_compile_failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}
