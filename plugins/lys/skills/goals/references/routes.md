# goals routes

Generated from the live Lys 0.3.1 OpenAPI document.
The current service schema and admission take precedence over this index.
All tool paths omit the HTTP endpoint's `/api` prefix.

| Method | Path | Operation | Declared JSON body schema |
| --- | --- | --- | --- |
| GET | `/alarms` | The caller's alarms, open first | not declared; inspect operation |
| GET | `/alarms/{id}` | One of the caller's alarms | not declared; inspect operation |
| POST | `/alarms/{id}/acknowledge` | A person acknowledges an alarm | AlarmAcknowledgement |
| POST | `/goals/{goal}/active` | Suspend or resume a standing aim | GoalActiveBody |
| POST | `/goals/{goal}/mark` | Judge a goal | GoalMarkBody |
| POST | `/goals/{goal}/resend` | A distinct authorised occurrence naming possible prior delivery | GoalResendBody |
| GET | `/goals/{goal}/resends/{prior}` | The kept resend of one uncertain goal delivery | not declared; inspect operation |
| POST | `/goals/{goal}/words` | Reword a standing aim | GoalWordsBody |
| GET | `/schedules` | The schedules the caller may see | not declared; inspect operation |
| POST | `/schedules` | Set a schedule | ScheduleSetBody |
| GET | `/schedules/{id}` | A schedule with every occurrence and delivery | not declared; inspect operation |
| POST | `/schedules/{id}/change` | Pause, resume or edit a schedule, keeping its count | ScheduleChangeBody |
| POST | `/schedules/{id}/stop` | Stop a schedule | ScheduleStopBody |
