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

## Item 2: requests counted two ways side by side

"Waiting for you" counts requests the person may decide from the server's first page of 50; the Requests tile below counts every waiting request on the one page `/requests` answered, including ones only someone else may decide. Drafts are counted from page one; product drafts are counted by the server and dropped.

| Option | What Cherie sees | Mistake it avoids | Serves |
|---|---|---|---|
| A. One "Waiting on you" list at the top of the side column: each thing she may decide on its own line in words ("Sam asks to edit project Logo", "Helper drafted: …"), each with Decide; then one line each for app drafts and reviews (server counts, every page, with their links); then one line "2 more wait on someone else". Its count is the length of the list, read over every page. The Requests and Drafts tiles leave the dashboard. | One list, one number that matches it, and only what is hers to do. | Two numbers measuring different sets (06 #2); a list said twice (06 #3); raw relation and action ids (06 "Words"). | Purpose: "could anyone have stopped it"; the obvious law (WHAT-I-THINK 11:0x). |
| B. Keep the counts table, drop the tiles. | Three numbers, none of them right past 50 requests. | Repetition only. | Half. |
| C. Keep both, ask the server to count every page. | Still two lists of the same thing. | Wrong counts, once the server changes. | Half, later. |

Recommendation: A. The list is its own second party for the count: the number is how many lines are shown, so the two can never disagree. The server's request and draft counts are no longer shown; its reviews and app-draft counts are, as single lines, because the dashboard lists neither. The tiles stay available to the canvas, which places them by hand.

Second go. Rendered in jsdom against fixtures. The first go said "2 app drafts to approve" and "1 grant to review": both words are on Cherie's never-read list ("grant"), and "app draft" names a mechanism. They now say "2 app actions wait for your approval" and "Check the access you gave: 1 is due". A relation the page has no plain word for first read "asks to auditor project logo"; it now reads "asks for auditor access to project logo". Still wrong: the resource is said by its kind and id ("project logo"), because the dashboard does not read the names the Requests page reads; and each Decide goes to the Requests list rather than to that request, because the Requests page takes no request in its address (domain 03's page). Approve and Decline in the row itself would be the satisfying version; it is domain 03's decision form and I have not copied it.

## Item 3: Running, Budget and Goals each shown twice

The agents table has Running, Budget and Goals columns; beside and under it the Running now panel, the Budget tile and the Goals tile say the same things again with the same helpers.

| Option | What Cherie sees | Mistake it avoids | Serves |
|---|---|---|---|
| A. Keep the table's columns; remove Running now, the Budget tile and the Goals tile. In their place, only when there is something, a "Needs a look" panel of exceptions in sentences: "Scribe has used 90% of its US$500 a week", "Scribe reached its limit of 1,000 tokens a day", "Helper missed a goal: send the invoice". Stop all moves to the Agents heading and is shown while anything runs or while what runs cannot be read. | One list of her agents, and below the waiting list only what is wrong, in words. Nothing at all when nothing is wrong. | Three lists of agents (WHAT-I-THINK "who is it for"); Stop All with nothing running (STAKEHOLDERS, Dashboard row). | Problem: "could anyone have stopped it"; value: the obvious next thing to look at. |
| B. Drop the Budget and Goals columns, keep the tiles. | A thinner table and two tiles to read across. | Doubling, but splits one agent across three places. | Less. |
| C. Fold the tiles away under "More". | The same doubling, one click away. | Clutter only. | Nothing new. |

Recommendation: A. "Near" is the budget's own `warn_at` when it has one; when it has none, 80% (the review's figure, and early enough to act before the limit's act happens). The same threshold drives the bar's warn colour in item 4, from one helper.

Second go. Rendered in jsdom. What I changed on reflection: a retired agent that still has a live session used to be reachable only from Running now; with that panel gone it would have lost its Stop, so the table now keeps a retired agent while it has a live session. Stop all first sat in the Agents heading always; the stakeholders sheet says "not Stop All with nothing running", so it shows only while something runs or while what runs cannot be read (the case the old comment guarded: the off switch is needed most when the page cannot say what runs). Still wrong: the near line says "$500 a week" with the old money words until item 8; the dashboard no longer lists upcoming goal deadlines anywhere but the table's "next due" line, which I think is right for Cherie but loses the one place a manager saw every goal at once (the canvas still has it).

## Item 4: the budget bar has two states and ignores warn_at

The bar is the accent below the limit and red at it; `warn_at` is never drawn, so "near" is never seen. On the canvas a tinted widget repaints every bar in its tint, so a bar at its limit can look like one far below it. Since item 3 the dashboard draws no bar at all: the table's Budget cell is words only.

| Option | What Cherie sees | Mistake it avoids | Serves |
|---|---|---|---|
| A. One limit bar, drawn as the HTML `meter` element, under the words of the Budget cell in the agents table and in the canvas Budget widget: accent below where the warning begins, the warn colour from there, red at the limit. Context limits draw no bar (compacting is the limit working). The threshold is item 3's `warnFraction`. | A thin bar under "$212 of $500 this week" that turns amber at 80% (or her warn_at) and red at the limit. | Two-state bar (06 #4); a level said only in numbers; and, by using `meter`, no inline width style. | Problem: "could anyone have stopped it" (seen before it acts). |
| B. Colour the words instead of drawing a bar. | Amber or red text. | Two-state, but colour on text alone is weak for anyone colour-blind and reads as an error. | Less. |
| C. Keep the old span bar and add a third class. | The same as A on the canvas; nothing on the dashboard. | Two-state only. | Less. |

Recommendation: A. The canvas tint rule is in `features/runtime` (not mine): asked, so a tint never repaints a near or full bar.

Second go. Looked at in headless Chrome with the app's own tokens: the three bands draw as accent, amber and red, the red at exactly the limit. Changed on reflection: the first go removed the old `.dash-bar` rules, which the canvas's usage widget (not mine) still draws, so they are back, marked as the canvas's; and the first go also put the bar's words in a `title`, a tooltip a phone never shows (06 #10), so the words are only the meter's accessible name and the cell's own text. Still wrong: Firefox not looked at; the canvas's tint rule still repaints its own span bars (asked).

Item 1, a third look, after seeing it drawn in headless Chrome (item 4's way of looking): with one agent that never ran, the total read "At least US$4.95", which to Cherie sounds like a bill that may be bigger than she is told. It now reads "US$4.95 since 16 Oct" with a line under it, "from 2 of 3 agents in full", and the agent's own line still says "Nothing reported". Nothing missing is counted as zero, and nothing is said in a frightening way.

## Item 5: usage totals across agents are added up in the browser

The canvas Usage widget sums dollars and running time and takes the highest context and plan percentages across agents (`features/runtime/canvas-widgets.ts:106-125`, not one of my folders). The server keeps period totals (`budgets_totals.rs`) but serves none across agents. Since item 1, the dashboard also adds its agents' week figures, and says how many it covers.

| Option | What Cherie sees | Mistake it avoids | Serves |
|---|---|---|---|
| A. Ask for the server summary in an exact shape, and keep the dashboard's one in-page sum (`weekTotal` in `Spend.tsx`) as the single place to replace when it is served; change nothing else. | What she sees today: one total, with how many agents it covers. | A figure computed two ways in two places (06 #5), and inventing a client for a route that does not exist. | Value: a total a rung-2 owner and a rung-5 sheet can trust. |
| B. Write the client reader and types for `/usage/summary` now, unused. | Nothing. | none: code for a route nobody has built, which "agrees with itself" until it meets the real answer. | Nothing. |
| C. Move the canvas's summing into my folder. | Nothing new. | A second copy, but it edits a file that is not mine. | Nothing. |

Recommendation: A. The ask now carries the exact answer shape so the route and the page can be written against one contract. Nothing in my folders changes for this item; the commit is this page and the ask.

Second go. Is A a dodge? The defect is that a total is computed in the page, and the only cure is the server; what I could do in my folders I did in item 1 (one sum, labelled by coverage, never a missing figure counted as zero). The honest cost: until the route exists, a rung-2 owner still cannot see other people's agents' spend, and the canvas widget still adds its own total a different way (it says "(n of m reporting)"; the dashboard says "from n of m agents in full"). I would rather the two wordings matched; the canvas is not mine, so it is in the ask.
