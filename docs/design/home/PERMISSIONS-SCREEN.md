# What an agent may do: the permissions screen

Written by Waffles, 3 October 2026, 20:25, for Tom to read before anything is built. Nothing on this
page is built. It replaces item 5 of SCREENS-PLAN.md, which was wrong: it relabelled three vague choices.

## What Tom said

"How is somebody supposed to know what an agent can do with those things, like 'most things'. If I said,
is this secure? And you said mostly. And I said, can you tell me any more? And you said, nah. You're
fired. This is meant to be a serious permission system for agents." (20:11)

"Those things are not the actual permissions. They need to be granular. If you want to have a thing like
that, that needs to be a permissions profile." (20:09)

## What is true in the code today

Read from the code, not from memory.

1. Lys stores real rules for each agent, in its settings (a profile version): three lists, `allow`
   (done without asking), `ask` (a person is asked first), `deny` (refused), plus `additional_directories`
   (folders beyond its working folder) and one `default_mode`. (`lys-identity-server/src/launch_permissions.rs`)
2. A rule names a tool and, for some tools, a target: `Read`, `Edit(/srv/site/**)`, `Bash(git push:*)`,
   `WebFetch(domain:example.com)`. Which forms a program accepts is in its description (`rule_forms`), and
   which modes it accepts (`modes`). (`lys-home/src/harness/description.rs`)
3. A policy adds hard rules that the agent's own settings cannot loosen: a whole tool, a path under Read or
   Edit, or a website under WebFetch. They are written as extra `deny` rules at start.
   (`lys-home/src/harness/rendering_permissions.rs`)
4. At start, Lys writes these lists into the program's own settings file. **The program (Claude Code or
   Codex) is what enforces them. Lys does not check each tool call today.**
5. The screen shows none of this. The settings form offers only the mode, as a dropdown with words like
   "reads freely" and "asks before most changes". The add-an-agent form saves only the mode. The three
   lists can be stored and are carried through a save, but a person can neither see nor change them.

So the defect is not wording. The store is granular; the screen hides it.

## What the screen becomes

One section in an agent's settings, named **What this agent may do**. No dropdown of moods.

**Every rule is a row.** A row says the thing and the answer, in words, with the program's own spelling of
the rule small beside it for whoever debugs:

| The thing | The answer |
|---|---|
| Read any file in its working folder | Without asking |
| Change files under /srv/site | Asks a person first |
| Run `git push` | Asks a person first |
| Run `rm` | Refused |
| Reach example.com | Without asking |

Rows are grouped by what a person would ask: files it reads, files it changes, commands it runs, websites
it reaches, connected tools it uses, folders beyond its working folder. Each group has Add. Adding a row
is a choice of thing (from what the chosen program accepts), a target where the thing takes one (a folder
from the folder chooser, a website, a command), and one of the three answers. No free text where Lys
knows the choices; no JSON.

**Anything not listed has one stated answer.** The mode is shown as exactly that sentence, per program,
for example "Anything not listed here: asks a person first." If a program's mode means something broader
(Claude Code's `bypassPermissions` means nothing is asked or refused except the Refused rows), the
sentence says that, in those words, in red. No mode is ever shown as an adjective.

**Rules a policy forces are shown and locked.** Each has the name of the policy that forces it. They
cannot be removed here, and the row says so.

**A permissions profile is a named, saved set of rows.** It is a record in Lys with a name, a version,
who made it and when. Choosing one fills the rows, and the rows stay on the screen: a profile is never a
label that hides its contents. Changing a row afterwards shows "Builder, with 2 changes" and lists the
two. Profiles are made from an agent's rows ("Save these as a profile") or on their own page. Lys ships
none that I have invented; which ones ship is Tom's call (question 1).

**The same rows on the add-an-agent form.** A new agent starts from a profile or from no rows, and the
form shows the rows before it is saved.

**What a run was actually handed.** Each start already keeps its launch record. The screen for a run
shows "This run started with these rules", the lists exactly as written into the program's settings file,
hard rules included. The settings screen and the run cannot disagree without it being visible.

**Changing a row is a new version of the agent's settings** and needs approval before the next start,
as any settings change does today.

## What this does not fix, said plainly

Enforcement stays with the program. If Claude Code or Codex has a hole, or a mode that ignores the lists,
Lys does not stop the call. Lys itself enforces today only: which computer may run the agent, which
secrets it is handed, which models and accounts it draws on, and the policy's forced refusals as written
into the settings file. A permission system that is serious in Tom's sense checks the call itself. The
proxy design (PROXY.md) already has the place for it: a gate holds a model call, sees the tool use the
model asked for, and can refuse it against these same rows. That is not written yet (question 2).

## Questions for Tom

1. Which permissions profiles ship with Lys, if any, and their exact rows. I will not invent them.
2. Enforcement at the proxy gate against these rows: part of this piece, or the piece straight after the
   screens are usable? My recommendation: straight after, as its own piece with its own design page, so
   starting an agent from the screen is not held up by it; and until it lands, the screen carries one
   sentence saying the program enforces these rules, not Lys.
3. Codex takes a sandbox mode and fewer rule forms than Claude Code. The screen shows only what the chosen
   program accepts and says what it cannot express. Say if that is wrong.

## How it gets built, once Tom has read this

1. Server: a route that reads and saves the three lists, extra folders and the mode as rows, checked
   against the program's `rule_forms` and `modes`; permissions profiles as records (make, read, version).
2. Screen: the rows, the groups, Add with the folder chooser, the not-listed sentence, locked policy rows,
   profiles, on both the settings form and the add-an-agent form.
3. Run screen: the rules a run was handed, from its launch record.
4. All written and read, one battery, one install, then a walk in a browser with screenshots before any
   claim that it works.
