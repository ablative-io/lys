# network routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/clients` | The client platforms and commits held by this install | not declared; inspect operation |
| GET | `/clients/{platform}/{name}` | Download a held client executable with its commit and SHA-256 | not declared; inspect operation |
| GET | `/network` | The machines | not declared; inspect operation |
| GET | `/network/machine-identities` | The computers that joined, as machine identities, and the grants each holds | not declared; inspect operation |
| POST | `/network/machines` | Name an owned computer, or administer a machine | NameBody |
| POST | `/network/machines/{id}/agents` | Allow an own agent on an own computer, or administer its agents | AgentBody |
| POST | `/network/machines/{id}/choose-folder` | The folder picked in a computer's own folder dialog | ChooseFolderBody |
| POST | `/network/machines/{id}/folders` | The folders inside one folder of a computer | FoldersBody |
| POST | `/network/machines/{id}/installer/{kind}` | Download an installer with a fresh connection for an own computer, or administer its installation | RunnerInstallerBody |
| POST | `/network/machines/{id}/join-code` | Give a one-time connection code for an own computer, or administer a join | JoinCodeBody |
| POST | `/network/machines/{id}/retire` | Retire a machine | not declared; inspect operation |
| GET | `/network/machines/{id}/runner` | A machine's runner | not declared; inspect operation |
| POST | `/network/machines/{id}/runner` | Name a machine's runner | RunnerBody |
| POST | `/network/machines/{id}/team` | Assign or clear a computer's owning team | TeamBody |
| GET | `/runner-receipts/{index}` | A runner act's receipt | not declared; inspect operation |
| POST | `/runner/dial/{machine}/next` | A dialled runner's next request | not declared; inspect operation |
| POST | `/runner/dial/{machine}/pass` | A joined machine's pass to an app, asked with its join's key | PassAsked |
| POST | `/runner/dial/{machine}/replies/{ticket}` | A dialled runner's reply | not declared; inspect operation |
| POST | `/runner/join` | Join another computer's runner with its connection code | RunnerJoinBody |
| GET | `/runner/protocol` | The runner protocol | not declared; inspect operation |
