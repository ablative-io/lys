# Configuration preflight

`lys-identity-server --check-config config.json` checks a configuration using
the incoming server's strict `Config` schema and structural validation. Use
`--check-config -` to read the bytes from standard input. Success prints one
JSON receipt with `format: "lys-config-check/1"`, the server's `build`, and
`config_sha256`, the SHA-256 of the exact input bytes. Failure exits nonzero
with a fixed diagnostic; it never prints configuration values.

This command does not start a listener, open or create the configured stores,
read the configured secret or signing-key files, or contact an issuer or app.
It checks structure, not readiness: success does not prove that credentials,
permissions, upstream services or the installation will work at runtime.

For a new `lys identity upgrade`, the incoming server checks the installed
configuration before rendering and checks the rendered replacement before
adoption, writing an upgrade intent or stopping a service. The upgrade verifies
each receipt's format, build and input digest. An incoming server without the
checker, unsupported or invalid configuration, an absent or duplicate rendered
service configuration, or a mismatched receipt refuses the new upgrade.

The original configuration is read back after candidate validation; a change
since its check refuses. This readback is not a lock against subsequent edits.
The operator must retain exclusive ownership of the installation throughout
the upgrade. Do not edit its configuration or run another upgrade concurrently.

An unsupported original field cannot disappear silently through rendering.
This slice does not implement a migration override: reconcile the configuration
through an explicitly reviewed migration before trying the upgrade again.
Existing adapter settings retain their existing meaning and validation.

Recovery of an already recorded, interrupted upgrade happens first, using its
existing intent and recovery rules. That recovery may stop or restart services;
the preflight guarantee concerns the subsequent **new** upgrade transaction.
