# Lys-Anchor — User Stories

## Operator — Runs the workspace's gate on a shared, loaded build host

**S1.** As an operator running the gate on a loaded host, I want the expired-certificate test to finish without waiting on the clock, so that the suite stays fast and its outcome does not depend on load.

## Reviewer — Checks that the certificate gate's tests can tell a correct gate from a broken one

**S2.** As a reviewer, I want the expiry refusal and its positive control judged at explicit instants, so that the refusal still proves the chain verification consults the validity window.

## Developer — Embeds RecognisedCertificate in code that must judge admission at a known instant

**S3.** As a developer, I want to ask RecognisedCertificate whether a submission is admitted at an instant I name, given the same submitter context admit takes, so that I can judge admission without reading the clock.

## Operator — Serves a live anchor

**S4.** As an operator of a live anchor, I want each live submission still judged at the present, so that adding admit_at changes nothing a live request sees.
