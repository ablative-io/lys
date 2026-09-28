# Lys-Anchor — Checklist

## Dependencies

- [ ] **C1** — crates/lys-anchor/Cargo.toml lists chrono.workspace = true under [dependencies], and no other dependency is added.

## Admission at a named instant

- [ ] **C2** — RecognisedCertificate has a public, documented admit_at taking the submission, the SubmitterContext and a DateTime<Utc>, which verifies the chain and validity window at that instant through lys_core::ca::verify_certificate_chain_at.
- [ ] **C3** — RecognisedCertificate's AdmissionPolicy::admit keeps its signature and calls admit_at with Utc::now().
- [ ] **C4** — certificate.rs's docs say a certificate is judged at the instant its caller names, and none says 'right now' or 'currently'.

## The expiry test

- [ ] **C5** — an_expired_certificate_is_refused contains no sleep and asserts Ok(()) from admit_at at an instant inside the one-second window and Err(NotAdmitted) at issuance plus 2 seconds.
- [ ] **C6** — grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-anchor prints nothing.
- [ ] **C7** — certificate_tests.rs's module doc describes the expiry case as judged at named instants, not as waiting for expiry.
- [ ] **C8** — The other 18 admission tests are byte-unchanged and pass.
