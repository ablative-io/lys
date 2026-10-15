# teams routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/teams` | Every team | not declared; inspect operation |
| POST | `/teams` | Create a team | TeamCreateBody |
| GET | `/teams/{id}` | One team | not declared; inspect operation |
| GET | `/teams/{id}/budget` | A team's limits and measured usage | not declared; inspect operation |
| PUT | `/teams/{id}/budget` | Replace a team's limit collection | BudgetBody |
| GET | `/teams/{id}/goals` | A team's goals | not declared; inspect operation |
| POST | `/teams/{id}/goals` | Set a goal on a team | GoalSetBody |
| POST | `/teams/{id}/members` | Add a team member | TeamMemberBody |
| POST | `/teams/{id}/members/{member}/confirm` | Confirm a held membership | TeamRetireBody |
| POST | `/teams/{id}/members/{member}/remove` | Remove a member | TeamRetireBody |
| POST | `/teams/{id}/nesting` | Replace a team parent and lead | TeamNestingBody |
| POST | `/teams/{id}/retire` | Retire a team | TeamRetireBody |
| GET | `/tree` | The caller's owned and led teams and descendants | not declared; inspect operation |
