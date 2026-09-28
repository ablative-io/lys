# Lys-Core — Checklist

## The underscore-binding rule

- [ ] **C75** — rules/ast-grep/no-underscore-binding.yml holds one rule, id no-underscore-binding, at severity error with vendor/** ignored, whose message and note say it refuses because a leading underscore silences the unused warning instead of fixing its cause.
- [ ] **C76** — On an uncommitted scratch file with one underscore-prefixed binding in each of the 19 binding positions, the scan reports exactly 19 no-underscore-binding hits, and none on a bare `_`, a struct field declaration, a field initialiser or a field access.
- [ ] **C77** — `ast-grep scan --config sgconfig.yml` reports zero hits at the repository root of the landed tree.

## Bindings cleared by what they do

- [ ] **C78** — Every guard whose directory or state the test still uses after the binding (a TempDir holder or an EnvCleanup guard) has a name without a leading underscore and is ended by drop(name) after its last use, with a tail result bound, the guard dropped and the binding returned.
- [ ] **C84** — Each of the eight test fixture structs that holds a TempDir guard names the field temp_dir and has a close method returning TempDir::close's error, and every test that builds one calls its close after the value's last use.
- [ ] **C79** — Every discarded error in a closure or match arm is left unbound, map_err is rewritten as .ok().ok_or(…) when the error is already built and .ok().ok_or_else(…) when building it calls a function or a macro, and each site returns the same error as before.
- [ ] **C80** — Every value with no use is not bound: a bare `_` inside its pattern, including a TempDir a fixture returns beside a value it has already loaded into memory, an expression statement for a whole let, and no `let _ =` statement introduced.
- [ ] **C81** — Every trait implementation that ignores a parameter takes it as a bare `_`, and no trait declaration changes.
- [ ] **C82** — The two cfg(not(unix)) stubs, fsync_dir and warn_if_loose_permissions, take their parameter as a bare `_`.

## Behaviour held

- [ ] **C83** — cargo test --workspace --all-features exits 0 at the commit the build starts from and at its final commit, with the same number of tests passed at both.
