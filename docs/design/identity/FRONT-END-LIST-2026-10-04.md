# Lys front end: everything I will do (4 Oct 2026)
Basis: all 185 files under surface/identity/src read in full; 51 pages and sub-pages looked at in the installed build 8e8ad98d. Evidence: FINDINGS.md beside this file.

A. One agent page, nothing hidden
1. Delete the "Details" fold on an agent's file. An agent gets the same head and always-visible tabs a person has.
2. Tabs become: Overview | Settings | Access | Limits and goals | Sessions | Credentials | Record. Role folds into Overview; Policy folds into Settings (one Rules table); Certificate folds into Credentials; Memory folds into Sessions.
3. Overview offers Settings, Start/Stop/Restart, Limits, Access for a running or a stopped agent alike. Team, computer and model are controls, not text.
4. Name is edited in the head; status (turn on, pause, retire) is a control in the head. Emergency stop stays there.

B. One settings form, laid out as a form
5. One settings component, used on the Settings tab and inside Add agent. The copy under the You-page row goes; the row links to the Settings tab.
6. A real form grid (two and three columns, labels aligned, full-width inputs). Remove `max-width:480px` on inputs and the second `.field` definition in overlays.css.
7. Rules: ONE table (what, on what, without asking / asks / refused, who enforces it, remove) with an add row. Replaces the permissions wizard, the bullet lists and the separate Policy tab.
8. Connected tools (MCP), skills, extra folders: tables with an add row. No wizard, no fold.
9. Computer is a field on the form; the "Where can it run" ticks become a column in it.

C. Limits and goals as tables
10. Limits: one editable table (measure, amount, period, when reached, used now, standing, remove) with an add row. Replaces the table + "Change the limits" fieldsets + "Add a limit" form, and the person's-limits articles. One vocabulary for the words.
11. Goals: one table (kind, words, deadline, evidence, standing) with an add row. Replaces the textarea and "Add a deadline".
12. Remove the 64-limit cap and the 500-character cap in the page unless you say keep them.

D. Nothing scrolls; the page is used
13. Remove `.screen {overflow:auto}` and the scrolling panes. Every page fits 1440x900 and reflows to the window.
14. Lists page by the height they have (previous / next, page n of m) in place of "Show 25 more" and scrolling.
15. Tables wrap or drop to a second line rather than being cut by a side pane; the side pane that only repeats the selected row is deleted on every page (People, Access, Resources, Requests, Reviews, Secrets, Computers, Service accounts).
16. Every `<details>` fold (44 in 29 files) is removed: its content shown in place or deleted. Errors use the one-sentence-one-button pattern already in CannotStart.

E. One home for each thing
17. Terminals: the canvas is the one place. Running = the list of what is running down the left of the canvas page. Remove the corner box and its four-pane screen, the terminal inside the People side pane, the terminal pane on Running, and the team screen under You. /runtime/canvas and /team go.
18. Access: one page with tabs Grants | Ask | Requests | Reviews | Resources | Graph | Model; "Issue" and "Give" are acts on it, in the page, not a drawer and not a separate page. Rail goes from Resources, Access, Graph, Requests, Reviews, Model to one item with counts.
19. A grant is drawn one way (the table row) everywhere: Access, the file's Access tab, Reviews, Requests.
20. People and agents: tabs People | Agents | Teams | Service accounts | Found, each with its own address. /directory/manage is deleted: its two register forms duplicate Add person / Add agent; Edit name and Change status move to the file's head; Connect sign-in and the Lys account (email, password, sign-in on/off) move to the person's Security tab. /account/:id goes. /sessions ("Sign-ins") goes: it is the same list as the Security tab and the You page.
21. Configuration: one page with tabs Sign-in providers | Apps | Services | Storage and keys, replacing /settings (seven near-empty sections), /connections and /apps as separate places. The app schema is shown as the relation-by-action table, not sentences; the schema builder gets the whole page.
22. Secrets: Entries is the page; visibility and who-may-receive are controls on the row; revocation state is a column in Activity. The "Advanced" and "Check revoked access" tabs go. /vault goes.
23. Roles: role text on the left, holders as a table on the right, templates as a table; no nested folds.
24. Teams: members as a table with remove per row and an add row (replaces one button per member).
25. Add agent: two columns (who and access on the left, settings on the right); access as a table with a tick column.
26. Computers: the add form and a computer's detail use the page, not a 360px strip.

F. Names, not identifiers
27. One resource namer and one identity namer used everywhere; no raw ids as labels (Access "On" column, secrets Who / Granted by, certificates, stop history, record lines).
28. One date formatter (four exist, two behaviours). Durations in words ("8 hours", not "28800 seconds").
29. Typed-by-hand identifiers become pickers: resource kind and id (Issue, Ask, role templates), person id (secret visibility), handle id.

G. Dead and duplicated code removed
30. Dead links: #/teams (two places), #/model/try. Dead files and code: NowSummary, the assistant dock mode and its styles, old start shapes in generated/index.ts, the second (older) sign-in screen in signin/Gate.
31. Four migrations of saved browser requests removed (add-and-run versions 1 and 2, legacy add-agent, legacy pending computer, legacy secret change). You said no migrations.
32. Nine copies of the keep-and-retry logic become one; three pending-change renderers become one; two refusal-word tables become one; two copies of the request function become one.
33. Silent caps fixed: `ask()` stops at 1000 pages and answers No; `chainOf` stops at 64; estate plan refuses over 250 resources or 100 agents; "N more agents" cut at 3; four-terminal limit (goes with the corner box).
34. Placeholder sentences removed ("…is coming", "does not yet list…", "Keep decisions are unavailable here").
35. Styles: one definition each for .field, .pill, .btn, inputs, the graph; the undefined `--text` token fixed.

H. Canvas
36. The canvas at 690bcb6a goes in with this piece (written and read; never built).

Order of writing: D13-16 and B6 first (they change every page), then A, B, C, E, F, G. All of it written and read by Archie and Vesper before anything is built; then one battery, one install, then I use every page with a real mouse and look at the pictures before I tell you anything works.

Yours to decide
a. Service accounts: a list of names with nothing attached. Keep as a tab of People, or delete?
b. There is no screen to add a secret or to register a model account (your long-lived Claude Code tokens). I have not read the server for whether routes exist; I will read it and add the screen to this piece if they do, and tell you if they do not.
c. No screen adds a second, remote computer or sets a team's lead or parent. Same answer as b.
d. `plainReason` rewrites the service's refusal sentences by pattern. I would delete it and show the service's own words.

I. Added after reading the server's route tables
37. Teams: set the lead and the parent team in the Teams table (the route exists, no screen uses it).
38. Teams: limits and goals for a team, the same two tables as an agent's (routes exist, no screen).
39. Computers: assign a computer to a team (route exists, no screen).
NOT in this piece because the server has no route (needs your word, it is back-end work): adding a secret in the browser; registering a model account (long-lived token); adding a remote computer with a dialled runner from the browser.
