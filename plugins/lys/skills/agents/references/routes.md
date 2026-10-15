# agents routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| POST | `/agents` | Register an own agent with a person's app pass or sign-in; administration requires sign-in | AgentRegistrationBody |
| GET | `/agents/{id}` | An agent the caller answers for | not declared; inspect operation |
| GET | `/agents/{id}/calls` | An agent's model calls, newest first | not declared; inspect operation |
| GET | `/agents/{id}/calls/{call}` | One model call whole: its record and both bodies | not declared; inspect operation |
| GET | `/agents/{id}/certificates` | An agent's certificates | not declared; inspect operation |
| POST | `/agents/{id}/certificates` | Issue an own agent's certificate with a person's app pass or sign-in; administration requires sign-in | IssueBody |
| POST | `/agents/{id}/certificates/{serial}/withdrawal` | Withdraw one | WithdrawBody |
| GET | `/agents/{id}/control-sessions` | An agent's control session identities, stopped sessions included | ControlPageQuery |
| GET | `/agents/{id}/goals` | An agent's goals | not declared; inspect operation |
| POST | `/agents/{id}/goals` | Set a goal on an agent | GoalSetBody |
| GET | `/agents/{id}/mcp-requests` | An agent's MCP requests | not declared; inspect operation |
| POST | `/agents/{id}/mcp-requests` | Ask for a declared MCP server | McpAskBody |
| POST | `/agents/{id}/mcp-requests/{request}/approve` | Approve an MCP request within remit | McpApproveBody |
| GET | `/agents/{id}/memory` | An agent's memory | not declared; inspect operation |
| POST | `/agents/{id}/pass` | An agent's pass to an approved app, carrying its own grants, asked with its run pass or a grant credential for a grant it holds | AppPassAsked |
| POST | `/agents/{id}/pass/revoke` | Revoke an agent's retained audience passes with its own run pass and seat proof; offline copies remain valid until expiry | AppPassAsked |
| GET | `/agents/{id}/policy` | An agent's tool-boundary policy | not declared; inspect operation |
| POST | `/agents/{id}/policy` | Set an agent's tool-boundary policy, from its next launch | PolicyBody |
| GET | `/agents/{id}/provisioning` | An agent's profile | not declared; inspect operation |
| POST | `/agents/{id}/provisioning` | Set an own agent's profile | ProvisioningSetBody |
| POST | `/agents/{id}/provisioning/{version}/review` | Review a profile | ReviewBody |
| GET | `/agents/{id}/refusals` | An agent's refused tool calls | not declared; inspect operation |
| POST | `/agents/{id}/reports-to` | Change an agent's reporting edge | ReportsToBody |
| POST | `/agents/{id}/restart` | Restart an agent on its latest reviewed profile | RestartBody |
| GET | `/agents/{id}/runtime/sessions` | An agent's runtime sessions | not declared; inspect operation |
| POST | `/agents/{id}/runtime/sessions/{session}/reports` | A runtime report | ReportBody |
| POST | `/agents/{id}/start` | Start an agent | not declared; inspect operation |
| GET | `/agents/{id}/start-command` | Preview the previous conversation's working folder | PreviewQuery |
| POST | `/agents/{id}/start-command` | An agent's start command | LaunchBody |
| POST | `/agents/{id}/stop` | Suspend an agent's access, withdraw certificates and end its sessions; ordinary Stop uses the runtime session end route | StopBody |
| GET | `/agents/{id}/stops` | An agent's stops | not declared; inspect operation |
| GET | `/agents/{id}/usage` | An agent's budget crossings and what came of each | not declared; inspect operation |
| POST | `/agents/{id}/usage` | Report a use an agent made | UsageBody |
| GET | `/agents/{id}/variables` | An agent's variables | not declared; inspect operation |
| POST | `/agents/{id}/variables` | Patch an agent's variables with the revision read | VariablesPatchBody |
| POST | `/agents/{id}/wake` | Wake an agent's live session | WakeBody |
| GET | `/agents/{id}/words` | An agent's slots resolved, and its own layer | not declared; inspect operation |
| POST | `/agents/{id}/words/{slot}` | Set a slot at an agent's layer | WordsSaveBody |
| POST | `/budgets/person/{id}/confirm` | Confirm a legacy personal budget | ConfirmBody |
| GET | `/budgets/{kind}/{id}` | A holder's limits and measured usage | not declared; inspect operation |
| PUT | `/budgets/{kind}/{id}` | Replace a holder's limit collection | BudgetBody |
| GET | `/harnesses` | The programmes Lys describes and their reviewed builds | not declared; inspect operation |
| GET | `/harnesses/plugins` | The Claude Code plugins installed in one config folder of a computer, as its runner reads them | PluginsQuery |
| POST | `/launch-records/{id}/start-again` | Start a launch again | not declared; inspect operation |
| GET | `/launch-records/{id}/state` | A launch's state | not declared; inspect operation |
| POST | `/launch-records/{id}/withdraw` | Withdraw a launch | not declared; inspect operation |
| GET | `/skills` | The skills Lys keeps | not declared; inspect operation |
| POST | `/skills` | Keep a skill's text | SkillBody |
