# words routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/words` | The words a seat is sent: every layer and template as held | not declared; inspect operation |
| POST | `/words/preview` | Render a slot for a session or an agent; nothing is sent | WordsPreviewBody |
| POST | `/words/templates/{name}` | Save a named template | WordsTemplateBody |
| POST | `/words/{slot}` | Set a slot at the workspace layer | WordsSaveBody |
