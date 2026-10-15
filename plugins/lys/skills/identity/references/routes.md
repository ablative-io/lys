# identity routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/dashboard` | The caller's own agents, how each is going, and what waits on the caller | not declared; inspect operation |
| GET | `/directory/agents/{id}` | Any agent, for the administrator | not declared; inspect operation |
| GET | `/directory/people` | Every person, for the administrator | not declared; inspect operation |
| GET | `/directory/people/{id}/account` | A person's sign-in account | not declared; inspect operation |
| POST | `/directory/people/{id}/account/email` | Change a person's email | NewEmail |
| POST | `/directory/people/{id}/account/enabled` | Enable or disable a person's sign-in | Enabled |
| POST | `/directory/people/{id}/account/password` | Set a person's password | Reset |
| GET | `/directory/people/{id}/sessions` | A person's sessions | not declared; inspect operation |
| POST | `/directory/people/{id}/sessions/{session}/end` | End a session | not declared; inspect operation |
| GET | `/identities` | Every identity the directory holds | not declared; inspect operation |
| GET | `/identities/{id}` | One identity | not declared; inspect operation |
| POST | `/identities/{id}/profile` | Change an identity's profile | NamedBody |
| POST | `/identities/{id}/transitions` | Move an identity's state; a person's app pass only activates an own registered agent | TransitionBody |
| GET | `/me` | The signed-in caller | not declared; inspect operation |
| GET | `/me/account` | The caller's sign-in account | not declared; inspect operation |
| POST | `/me/account/email` | Change the caller's email | OwnEmail |
| POST | `/me/account/password` | Change the caller's password | OwnPassword |
| GET | `/me/session/variables` | The asking run's session variables, by its run pass | not declared; inspect operation |
| POST | `/me/session/variables` | Patch the asking run's session variables, by its run pass | VariablesPatchBody |
| GET | `/me/variables` | The asking run's agent variables, by its run pass | not declared; inspect operation |
| POST | `/me/variables` | Patch the asking run's agent variables, by its run pass | VariablesPatchBody |
| GET | `/people` | The people the caller may see | not declared; inspect operation |
| POST | `/people` | Register a person | NamedBody |
| POST | `/people/admit` | Admit a person in one act: register, issuer account by email, bind, activate, and a first root grant only when one is asked for | PeopleAdmitBody |
| POST | `/people/{id}/logins` | Bind a login to a person | BindLoginBody |
| GET | `/service-accounts` | The service accounts | not declared; inspect operation |
| POST | `/service-accounts` | Create a service account | ServiceAccountCreateBody |
| POST | `/service-accounts/{id}/retire` | Retire a service account | ServiceAccountRetireBody |
| GET | `/sessions` | The caller's sessions | not declared; inspect operation |
| POST | `/sessions/{id}/end` | End one's own session | not declared; inspect operation |
