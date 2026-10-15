# draft-workflow routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/drafts` | The drafts the signed-in person may decide | not declared; inspect operation |
| POST | `/drafts` | Record an agent's immutable draft | DraftBody |
| POST | `/drafts/{id}/approve` | Record approval without applying the action | DraftApproveBody |
| POST | `/drafts/{id}/correct` | Refuse and save the responsible person's correction | DraftCorrectBody |
| POST | `/drafts/{id}/refuse` | Refuse one immutable draft | DraftRefuseBody |
| GET | `/product-drafts` | The drafts a person may see; an app's approved drafts, for its connector | ApprovedQuery |
| POST | `/product-drafts` | Record a product's held act as a draft | ProductDraftBody |
| POST | `/product-drafts/{id}/approve` | Approve a product draft | ProductApproveBody |
| POST | `/product-drafts/{id}/executed` | Close an approved draft as executed | ExecutedBody |
| POST | `/product-drafts/{id}/refuse` | Refuse a product draft | ProductRefuseBody |
| POST | `/product-drafts/{id}/refused-on-execution` | Close an approved draft as refused on execution | RefusedOnExecutionBody |
