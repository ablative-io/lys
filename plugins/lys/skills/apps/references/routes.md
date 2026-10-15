# apps routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/.well-known/oauth-authorization-server` | How a connected app is authorized | not declared; inspect operation |
| GET | `/.well-known/oauth-protected-resource` | Where the MCP door's authorization is found | not declared; inspect operation |
| GET | `/.well-known/oauth-protected-resource/api/mcp` | Where the MCP door's authorization is found, by its screens address | not declared; inspect operation |
| GET | `/.well-known/oauth-protected-resource/mcp` | Where the MCP door's authorization is found, by its address | not declared; inspect operation |
| GET | `/.well-known/openid-configuration` | The issuer's discovery document | not declared; inspect operation |
| GET | `/apps` | Every app the caller may see | not declared; inspect operation |
| POST | `/apps` | Register an app: pending until the administrator approves it | RegisterBody |
| POST | `/apps/bench` | Open a test bench on a draft schema | OpenBody |
| POST | `/apps/bench/{id}/ask` | Ask the bench: may X do Y to Z | AskBody |
| POST | `/apps/bench/{id}/close` | Close a bench | not declared; inspect operation |
| POST | `/apps/connect` | Connect an app from its manifest: register, approve and issue in one act, answering where the browser goes with a one-time code | ConnectBody |
| POST | `/apps/connect/exchange` | Exchange a one-time connect code for the app's credentials, once | ExchangeBody |
| GET | `/apps/me` | The app a credential signs in as | not declared; inspect operation |
| GET | `/apps/me/agents` | The access pass holder's own agents | ProductAgentsQuery |
| POST | `/apps/registrars` | Make a service account a registrar | RegistrarBody |
| GET | `/apps/{app}` | One app the caller may see | not declared; inspect operation |
| POST | `/apps/{app}/approve` | Approve an app with its sign-in settings, after configured broker credential custody | AppApprovalBody |
| POST | `/apps/{app}/bearer/issue` | Issue or rotate an approved app's bearer, answered once | ClientCredentialIssueBody |
| POST | `/apps/{app}/connector` | Give an app approved before connectors its connector, answering to the administrator acting | ConnectorBody |
| POST | `/apps/{app}/credentials/issue` | Issue an approved app a client credential, answered once | ClientCredentialIssueBody |
| POST | `/apps/{app}/credentials/{credential}/revoke` | Revoke one of an app's client credentials | ClientCredentialRevokeBody |
| POST | `/apps/{app}/decline` | Decline an app | DecideBody |
| POST | `/apps/{app}/placements` | Place a resource in its parent | PlaceBody |
| POST | `/apps/{app}/retire` | Retire an app | DecideBody |
| GET | `/apps/{app}/schema` | A schema version, the current one unless `version` names one | not declared; inspect operation |
| PUT | `/apps/{app}/schema` | Change a schema, replacing the version named | ChangeBody |
| POST | `/apps/{app}/schema/approve` | Approve the schema change waiting | ChangeDecision |
| POST | `/apps/{app}/schema/check` | What a schema change would do, writing nothing | CheckBody |
| POST | `/apps/{app}/schema/decline` | Decline the schema change waiting | ChangeDecision |
| POST | `/apps/{app}/sign_in` | Set an approved app's return addresses and whether it is given the person's name | AppSignInBody |
| GET | `/oauth/authorize` | Begin an authorization | not declared; inspect operation |
| GET | `/oauth/jwks` | The issuer's signing keys | not declared; inspect operation |
| POST | `/oauth/jwks/rotate` | Rotate the key passes are signed with; the old key stays published one lifetime | RotateBody |
| GET | `/oauth/mcp/authorize` | Ask the person to connect an app | not declared; inspect operation |
| POST | `/oauth/mcp/consent` | The person connects an app or refuses it | not declared; inspect operation |
| POST | `/oauth/mcp/register` | An app registers to connect to the MCP door | not declared; inspect operation |
| POST | `/oauth/mcp/token` | Exchange a connected app's code or refresh token for tokens | not declared; inspect operation |
| POST | `/oauth/token` | Exchange a code or a refresh token for a pass | not declared; inspect operation |
| GET | `/oauth/userinfo` | The signed-in subject's claims | not declared; inspect operation |
