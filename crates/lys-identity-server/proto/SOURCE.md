# Where these protocol files come from

The identity server's `SpiceDB` client is compiled from the files under
`crates/lys-identity-server/proto/authzed/` by `crates/lys-identity-server/build.rs`,
with protox, so no protoc binary is needed. Nothing here is edited by hand.

## The authzed v1 API

- Release: **authzed/api `v1.53.0`**, commit `d5fc38fe34ec0a74f782e4328d978a3cbac633b4`
  (https://github.com/authzed/api/tree/v1.53.0).
- Why this release: `deploy/identity/versions.json` pins SpiceDB `v1.56.2`. SpiceDB
  `v1.56.2`'s `go.mod` requires `github.com/authzed/authzed-go v1.10.0`, whose
  `magefiles/gen.go` generates its API from `buf.build/authzed/api` at buf commit
  `55aa23d533a34fa8bf8c7cf1744d8bf7`. authzed-go `v1.10.0` is the merge of its pull
  request 418, "add-zedtoken-to-download-api", on 2026-05-21, and authzed/api `v1.53.0`
  is the merge of its own pull request 169 of the same name, the same day: the API
  release SpiceDB `v1.56.2` builds against.
- Files, copied unchanged from `authzed/api/v1/` at that commit into
  `proto/authzed/api/v1/`: `core.proto`, `debug.proto`, `permission_service.proto`,
  `schema_service.proto`. The licence is `proto/authzed/LICENSE`, copied from the same
  commit.

| File | SHA-256 |
| --- | --- |
| `authzed/api/v1/core.proto` | `de6878f083dee28e82923aa79455da91eea11866d09ced86e4c2933846a4b265` |
| `authzed/api/v1/debug.proto` | `2cd357e2305d1b5ea933b147f35c959203fd6b193a4d94979f5adec7118a234a` |
| `authzed/api/v1/permission_service.proto` | `0e7dd93537c4dc6fc37ab94152f76064daaed983c7905767d75315a135a79898` |
| `authzed/api/v1/schema_service.proto` | `390ecd5819ee9a91b30b43b9ee98523950f5922fa4d12846e80d063bd1e8d2c1` |

## The files the API imports

The API's files import validation and gateway annotations. They are kept beside it
under `proto/authzed/third_party/`, each at the exact buf commit authzed/api
`v1.53.0`'s `buf.lock` names, fetched from `https://buf.build/<module>/archive/<commit>.zip`.
They define options only: the build parses them because the API's files name them,
generates their packages beside the API's, and includes none of them, since no option
changes a message the client sends or reads. Each module's licence is copied beside it.

| Module (buf commit) | File | SHA-256 |
| --- | --- | --- |
| `buf.build/bufbuild/protovalidate` (`0409229c37804d6187ee0806eb4eebce`) | `buf/validate/validate.proto` | `9f2fb1c6893adbc8b34bbcba655b6ac5a0325aef6ea5c111c68a189cff0646f7` |
| `buf.build/envoyproxy/protoc-gen-validate` (`daf171c6cdb54629b5f51e345a79e4dd`) | `validate/validate.proto` | `68c9625ebe0668605a37670db1759ceb03864bc07c52eee919b049347cbc018c` |
| `buf.build/googleapis/googleapis` (`61b203b9a9164be9a834f58c37be6f62`) | `google/api/annotations.proto` | `e79ea741cb605a65e78ca322174764a4af9fde1962c1631e12b84c4934ba9a6c` |
| `buf.build/googleapis/googleapis` (`61b203b9a9164be9a834f58c37be6f62`) | `google/api/http.proto` | `4a4d9be6a5c7f1989c93c25c71b48ff1b401645790b8b978ad34d579e29c4a2a` |
| `buf.build/googleapis/googleapis` (`61b203b9a9164be9a834f58c37be6f62`) | `google/rpc/status.proto` | `3b5c712455570ac4342dd3c521c4c11011652ae9a0fbca75ba22fcc45c6e1991` |
| `buf.build/grpc-ecosystem/grpc-gateway` (`4c5ba75caaf84e928b7137ae5c18c26a`) | `protoc-gen-openapiv2/options/annotations.proto` | `1f8e23ce506c358349d5ce0e54c336e3b2f3bf17c5d9e7d965e24a6e3429c2c9` |
| `buf.build/grpc-ecosystem/grpc-gateway` (`4c5ba75caaf84e928b7137ae5c18c26a`) | `protoc-gen-openapiv2/options/openapiv2.proto` | `ca1833903416822ad9540529f727b0010d77ecaf6987ae8b15f3b6acb005b6e8` |

The well-known `google/protobuf/*.proto` files come from protox itself, and their
messages from `prost-types`.

## Changing the release

When `deploy/identity/versions.json` pins another SpiceDB, find the authzed/api release
that SpiceDB's authzed-go builds against as above, replace the four API files and the
imported files at the commits its `buf.lock` names, and update this file and its
digests in the same change. The disposable SpiceDB the tests start pins its release
archives' digests in `crates/lys-identity-server/tests/spicedb_support/server.rs`,
which is updated in the same change.
