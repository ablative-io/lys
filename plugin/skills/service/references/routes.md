# service routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/authority` | The authority this service speaks for | not declared; inspect operation |
| GET | `/callback` | Finish a sign-in and begin a session | not declared; inspect operation |
| GET | `/canvas` | The caller's canvas: their arrangement and saved layouts | not declared; inspect operation |
| PUT | `/canvas` | Keep the caller's arrangement | CanvasKeepBody |
| POST | `/canvas/layouts` | Save an arrangement as a layout of the caller's, by name | CanvasSaveBody |
| POST | `/canvas/layouts/remove` | Remove a layout of the caller's, by name | CanvasRemoveBody |
| GET | `/changes` | Wait for the next change signal | not declared; inspect operation |
| GET | `/configuration` | The service's configuration | not declared; inspect operation |
| PUT | `/configuration` | Set the organisation zone at its current version | ZoneBody |
| GET | `/connections` | What the service is connected to | not declared; inspect operation |
| GET | `/health` | That this service is serving, with its name and build | not declared; inspect operation |
| POST | `/identity/estate-apply` | Apply grants through the installed loader | ImportDocument |
| GET | `/identity/estate-plan` | Read the installed estate plan | not declared; inspect operation |
| POST | `/identity/import` | Import named entries as an independently authorised service account | ImportDocument |
| GET | `/login` | Redirect to the sign-in screen | not declared; inspect operation |
| GET | `/mcp` | MCP server stream availability | not declared; inspect operation |
| POST | `/mcp` | MCP calls through the admitted HTTP router | Envelope |
| GET | `/openapi.json` | This document | not declared; inspect operation |
| GET | `/receipts/{index}` | A directory receipt, publicly | not declared; inspect operation |
| GET | `/security-events` | Recovery events visible to the administrator | not declared; inspect operation |
| GET | `/service-key` | The service's public key | not declared; inspect operation |
| POST | `/setup` | Finish the administrator's first-run setup | SetupRequest |
| POST | `/setup/administrator` | Register the first administrator | NewAdministrator |
| POST | `/setup/open` | Open first-run setup with its code | Opened |
| POST | `/setup/password` | Set the administrator's password without changing other sign-in factors | NewPassword |
| POST | `/sign-in` | Sign in with a password | SignInBody |
| GET | `/sign-in-providers` | The sign-in providers | not declared; inspect operation |
| POST | `/sign-in-providers` | Set a sign-in provider | SignInProviderSetBody |
| GET | `/sign-in/help` | The configured administrator's sign-in contact | not declared; inspect operation |
| GET | `/sign-in/providers` | The providers the sign-in page offers | not declared; inspect operation |
| GET | `/sign-in/providers/{id}` | Begin sign-in through a provider | not declared; inspect operation |
| GET | `/surface-contract` | The surface registration and computer admission contract | not declared; inspect operation |
