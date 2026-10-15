# access routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/agent/grants` | The signed agent's own live grants and their chain admission | not declared; inspect operation |
| GET | `/grants` | The grants the caller may see | not declared; inspect operation |
| POST | `/grants` | Pass on part of a grant | DelegateBody |
| POST | `/grants/agent-roots` | Record the administrator's roots for giving people and agents access | not declared; inspect operation |
| POST | `/grants/bindings` | A Lys-judged grant binding for a pass a registry issued | GrantBindingRequest |
| GET | `/grants/cannot-give` | What the caller cannot pass on | not declared; inspect operation |
| POST | `/grants/changes` | The grant changes after a consumer's cursor, with watermarks, readiness and resets | GrantChangesRequest |
| POST | `/grants/check` | Check, and record, an exercise | ActionBody |
| POST | `/grants/check/batch` | Any number of checks, answered in order at one revision | BatchBody |
| POST | `/grants/checkpoint/reset` | Discard the refused grant checkpoint the administrator names and rebuild the grants from their log | ResetBody |
| POST | `/grants/membership` | Whether a subject may read or post in a placed channel, at one revision | MembershipRequest |
| POST | `/grants/membership/admission` | Whether a subject holds a current grant within a workspace | MembershipAdmissionRequest |
| GET | `/grants/membership/counts` | The membership capability's work counts and retained entries | not declared; inspect operation |
| POST | `/grants/membership/recipients` | The subjects who may read or post in a channel, a bounded page at a time | MembershipRecipientPageRequest |
| POST | `/grants/membership/resources` | The resources a subject may read or post in, a bounded page at a time | MembershipResourcePageRequest |
| GET | `/grants/model` | Lys's own permission model | not declared; inspect operation |
| POST | `/grants/reach` | Who may act on each of many resources | ReachBody |
| POST | `/grants/roots` | Issue a root grant, outright or held by draft or by two | RootBody |
| POST | `/grants/which` | The ids of a kind a subject may act on, by page | WhichBody |
| POST | `/grants/who` | Who may act on a resource | WhoBody |
| POST | `/grants/why` | Why the caller may act | ActionBody |
| GET | `/grants/{id}` | One grant the caller may see | not declared; inspect operation |
| POST | `/grants/{id}/revoke` | Revoke a grant and all it derives | RevokeBody |
| POST | `/grants/{id}/tokens` | Issue a credential for one grant | GrantTokenIssueBody |
| POST | `/grants/{id}/tokens/{token_id}/revoke` | Revoke one grant credential | not declared; inspect operation |
| POST | `/link-audit` | Deliver a link-audit record | LinkAuditDelivery |
| POST | `/link-audit/person` | Look up a link-audit holder | LinkAuditAsked |
| GET | `/requests` | The access requests | not declared; inspect operation |
| POST | `/requests` | Ask for access | RequestAskBody |
| POST | `/requests/{id}/approve` | Approve an access request | ApproveBody |
| POST | `/requests/{id}/decline` | Decline an access request | DeclineBody |
| POST | `/requests/{id}/reconcile` | Settle an approval | not declared; inspect operation |
| GET | `/resources` | The resources grants are on | not declared; inspect operation |
| GET | `/reviews` | The grants due for review | not declared; inspect operation |
| POST | `/reviews/{grant}/keep` | Keep a grant under review | KeepBody |
| GET | `/roles` | Every role | not declared; inspect operation |
| POST | `/roles` | Make a role | MakeBody |
| GET | `/roles/{id}` | One role | not declared; inspect operation |
| POST | `/roles/{id}/holders` | Assign a role | AssignBody |
| POST | `/roles/{id}/holders/{holder}/end` | End a holding | EndBody |
| POST | `/roles/{id}/holders/{holder}/move` | Move a holding | MoveBody |
| POST | `/roles/{id}/versions` | Revise a role | VersionBody |
