# runtime routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/runtime/found` | The sessions found | not declared; inspect operation |
| POST | `/runtime/found/{session}/reports` | Report a found session | ReportBody |
| GET | `/runtime/live` | The sessions still running | not declared; inspect operation |
| GET | `/runtime/message-edges` | Caller-visible message addresses from the message service, mapped to Lys identities | not declared; inspect operation |
| GET | `/runtime/sessions` | Every runtime session | not declared; inspect operation |
| POST | `/runtime/sessions/{id}/compact` | Compact a session | Empty |
| POST | `/runtime/sessions/{id}/end` | End a session | Empty |
| POST | `/runtime/sessions/{id}/input` | Type into a session | InputBody |
| POST | `/runtime/sessions/{id}/input-bytes` | Write exact bytes to a session | InputBytesBody |
| POST | `/runtime/sessions/{id}/keys` | Send a session keys | KeysBody |
| POST | `/runtime/sessions/{id}/read` | Read a session's screen | ReadBody |
| POST | `/runtime/sessions/{id}/read-bytes` | Read a session's exact output bytes | ReadBytesBody |
| POST | `/runtime/sessions/{id}/resize` | Resize a session | ResizeBody |
| GET | `/runtime/sessions/{id}/variables` | A session's variables | not declared; inspect operation |
| POST | `/runtime/sessions/{id}/variables` | Patch a session's variables with the revision read | VariablesPatchBody |
| POST | `/runtime/sessions/{id}/wait` | Wait for a session to show a pattern | WaitBody |
| POST | `/runtime/sessions/{id}/words/{slot}` | Set a slot at a session's layer | WordsSaveBody |
| GET | `/runtime/sessions/{session}/control-receipts` | A bounded page of session control evidence | ControlPageQuery |
| POST | `/runtime/sessions/{session}/control-receipts/{operation}/reconcile` | The responsible person's decision about uncertain delivery | ControlDecisionBody |
| GET | `/runtime/sessions/{session}/controls` | A session's current control identifiers | not declared; inspect operation |
| GET | `/runtime/stop-everything` | Whether everything is stopped, by whom, when and why | not declared; inspect operation |
| POST | `/runtime/stop-everything` | Stop every session on every computer and refuse every start | StopEverythingBody |
| POST | `/runtime/stop-everything/console` | Stop every session under the service's own console signature | ConsoleStopBody |
| POST | `/runtime/stop-everything/release` | Let agents start again | StopEverythingReleaseBody |
| GET | `/seats` | Every seat with its state from its runner's own knowledge, and whether the runners were read | not declared; inspect operation |
| POST | `/seats` | Add a seat: a name an agent runs under on one machine from one reviewed profile version | SeatAddBody |
| GET | `/seats/owned` | Every seat owner each runner started, live and unreachable as the runner proved them at the kernel | not declared; inspect operation |
| GET | `/seats/unregistered` | The sessions runners hold for no seat | not declared; inspect operation |
| GET | `/seats/{name}` | One seat with its state | not declared; inspect operation |
| POST | `/seats/{name}/attach` | Read a seat's live session as lines, following it | SeatAttachBody |
| GET | `/seats/{name}/import` | A seat's imports, its selected import and its latest dry run | not declared; inspect operation |
| POST | `/seats/{name}/import/confirm` | Confirm one seat's exact dry-run plan and apply it, resumably, under one operation | SeatImportConfirmBody |
| POST | `/seats/{name}/import/dry-run` | Read one seat's declared sources at one instant and show the whole import plan; nothing is sent, started or changed | SeatImportDryRunBody |
| POST | `/seats/{name}/restart` | Stop a seat and restart its reviewed profile's selected transport under a new session | SeatStopBody |
| POST | `/seats/{name}/retire` | Retire a stopped seat and release its name without discarding its history | SeatOperationBody |
| POST | `/seats/{name}/send` | Send a seat a message, delivered as a user turn | SeatSendBody |
| POST | `/seats/{name}/start` | Start a seat through its reviewed profile's selected transport | SeatStartBody |
| POST | `/seats/{name}/stop` | Stop a seat between turns, or mid-turn with force | SeatStopBody |
| POST | `/seats/{name}/type` | A line typed in an attach, delivered as a user turn | SeatSendBody |
| POST | `/seats/{name}/version` | Select a reviewed profile version for a stopped seat | SeatVersionBody |
