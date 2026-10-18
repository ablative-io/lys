# Domain 06 options: dashboard, usage, budgets

Each item from the top ten of `lys-review/06-dashboard-usage-budgets.md`, in order: the options, what Cherie (rung 1, first MacBook this weekend, a Claude subscription) would see, which review mistake each avoids, which purpose it serves (what purpose / what problem / what value), the recommendation, and then a second go.

## Item 1: no cost anywhere on the dashboard

The service already answers each agent's dollars this week in `usage.figures` on `GET /dashboard` (`budgets_figures.rs`, `SHOWN`), with the week's start in `since_ms`. Nothing on the page shows it. Per model and per account are not in that answer.

| Option | What Cherie sees | Mistake it avoids | Serves |
|---|---|---|---|
| A. A "This week" panel beside Waiting: a total in US$ with two decimals, then one line per agent, dearest first, every agent listed (with or without a limit), and in plain words how many reported nothing. One visible line says it is the price at API rates, and, when an agent is on a plan account, that the plan is what she pays. | "This week, since 13 Oct: US$4.20. Helper US$4.20." and "On a plan you pay the plan's price; this is what the same use would cost at API prices." | Dollars only as "used of limit" for limited agents (06 #1); a figure with no unit (06 #8); a plan price shown as money owed (06 "Units"). | Value: "what did it cost"; the problem "costing what". |
| B. A cost column in the agents table. | A fourth figure in a row already holding Running, Budget, Goals. | none: it repeats the table-doubling the review condemns (06 #3) and gives no total. | Weak: no total, no place for the plan sentence. |
| C. Wait for a server `/usage/summary` with person, model and account, show nothing until then. | Nothing, still. | Adding up in the browser (06 #5). | Nothing today. |

Recommendation: A. The per-agent figures are already the server's; the page only adds a handful of them for the total and says when one is missing ("at least"), never counting a missing figure as zero. The per-model and per-account split and an organisation total are written as a server ask, so C's concern is carried without leaving the page empty.

Second go. Looked at in jsdom with fixtures and at the dev server's compiled module (I may not sign in, so not against the installed service's data). Two things were wrong and are changed: when no agent had given a figure the total said "US$0.00", which reads as nothing spent, so it now says "Nothing reported yet"; and a part-priced week said "3 records gave no price", an engineer's count Cherie cannot use, so it now says "some of its use gave no price". Still wrong and not mine to fix in the page: a rung-2 owner sees only her own agents, not each person's, and there is no split by model or account; both need the server's summary (asked). The week's start is the service's own ("since 12 Oct"), but in the reader's time zone, which on a phone abroad could print the day before.
