# Historical identity receipts

`operator-f8c4cb92.json` is the signed leaf produced through the HTTP service by
`matching_token_records_operator_provenance_and_exact_retry_keeps_one_leaf`
at main `f8c4cb929d2bd94643d329c36dd22e43f8cc862a` on 29 September 2026.
Only the test was changed to export its existing leaf and public service key;
production encoders, admission and storage were unchanged. The export contains
no private signing key or operator token. The candidate does not regenerate it.
The JSON file SHA-256 is
`46f7a8faf093ac51ec9f655bb5d8af6b40544ffd737ea0551b5181d5cf495973`.

The retained capture evidence is `chippy-main-operator-capture-2/test.log` and
`capture-only.patch` under the delivery evidence directory. `bearer-1b568cd9.json` is the signed agent registration produced through
`POST /identity/import` by the real HTTP test
`old_install_gains_loader_and_reimport_after_restart_writes_nothing` at
`1b568cd90578f5ed5d7d438e628b23724eef7f12`. Its public JSON SHA-256 is
`bbf3cc965e8c4d3c8ba1001ef838c5cea9341ab89c370a0aefe2d3cf8d6704fd`.
Only a test export was added; the historical production code was unchanged.
Evidence is in `chippy-old-bearer-capture/test.log` and `capture-only.patch`.

Both captures passed their original HTTP test. These frozen reader tests do
not replace the separate real-install upgrade and rollback proof, which also
captures a receipt from its own old install and checks its unchanged bytes.
