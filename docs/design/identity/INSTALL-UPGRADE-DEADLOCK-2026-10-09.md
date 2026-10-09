# Install and upgrade each sent the operator to the other (9 October 2026)

Written by Gaia on Waffles' order of 18:3x. The fix landed as 01e36e7b. This note records the defect, the outage it led to, and what is still owed.

## The defect

70b7bd9c (9 October 11:18, "Refuse upgrades with undeclared live issuer key authority") made an upgrade read back two live API keys in the issuer before it touched any installed unit. The two keys are `lys_configure`, the install's own key, and `lys_directory`, the service's narrow key.

On an install made before `lys_directory` existed, the two commands blocked each other:

- `lys identity upgrade` (6138ac62) refused with `read_back_mismatch: verify live API key rights lys_directory: missing rights: …` and told the operator to run `lys identity install` first.
- `lys identity install` (6138ac62) refused with `install_build_differs: … the placed lys-secrets is ba9ddcfd… and this install's is 6138ac62…` and told the operator to run the upgrade.

Neither path could complete with the new binary alone. Only the placed (older) binary's own install could make the key, because it does not hit `install_build_differs`.

## What it led to

Running the placed ba9ddcfd install to make the key re-rendered `state/compose.env` from a `deployment.toml` whose `public_origin` had been edited at 10:32 that day. Rauthy 0.36.2 refuses an `RP_ORIGIN` without an explicit port, so the sign-in service crash-looped from 18:15. The way it was restored also wrote two issuer-move leaves to the directory log, out to `:443` and back, which stay as the record. Lys answered again at 18:29:45. The renderer fix is c88ae6d4.

## The fix (01e36e7b)

`directory_key::verify`, the upgrade's read-back, now makes a missing `lys_directory` first, through `directory_key::provide`, exactly as the install makes it. It then reads the key back.

- A key that exists is never changed there.
- The install's `install_build_differs` refusal is unchanged.

Test: `an_upgrade_over_an_install_without_the_directory_key_makes_it_then_reads_it_back`.

## Still owed

- An install or upgrade must not re-render a running service's configuration from an edited `deployment.toml` without saying so. The 10:32 edit sat unapplied for eight hours and then took sign-in down on an unrelated run. The re-render should name every value it changes from the running files before it changes them.
- `identity.json`'s `redirect_url` and the issuer client's registered redirects must agree by construction. This was fixed at d4539fae, after a hand edit at 19:29.
