# What an agent may do: the truth today, and what has to change

Waffles, 3 October 2026, rewritten 20:31 by the clock; amended 20:32 from Vesper's read against her table (six sentences that said more than their rows) and Archie's observation of the tools a real run was offered; amended again from Archie's read of the Codex part against his table (four findings). The first version of this page (068c7c51) is
withdrawn: it had claims from memory that the sources contradict. Nothing is built from this page until
Tom has read it.

Every line under "What is true today" names its rows in the three fact tables, where each claim sits
beside the program's own documentation, Lys's code at a path and line, and what was seen for real:
W = `docs/harness/reference/FACTS-waffles.md`, V = `docs/harness/reference/claude-code/FACTS.md` (Vesper),
A = `docs/harness/reference/codex/FACTS.md` (Archie). **Agreed** means all three legs say the same.
**Open** means one is missing, and nothing may be built on it until it is closed. Everything under
"What I propose" is a proposal, not a fact.

## What Tom asked for

"This is meant to be a serious permission system for agents." "They need to be granular. If you want to
have a thing like that, that needs to be a permissions profile." "Not based on a single assumption."
"This had better work for both of them." The test: the agent is running in a children's intensive care
unit.

## What is true today

**The screen said the opposite of what the run got.**
- The settings form showed "Reads freely and asks before most changes and commands". Those are the
  catalogue's words for Claude Code's `default` mode. (W2, agreed)
- The file tonight's run of pancake was handed refused Read, Write, Edit, Glob, Grep and WebFetch by name,
  and was written to refuse Read anywhere on the disk (`Read(//)`, `Read(///**)`); allowed nothing; no
  sandbox; no hooks. (W3, V3, agreed as to what the file holds) Whether Claude Code reads the second of
  those two path rules as a rule or silently skips it was not seen. (V11, open)
- Those refusals are pancake's own policy, version 1, seven hard rules. Nobody chose it: Lys wrote it for
  every agent added between 1 October 17:39 and 2 October 12:33 and has never taken it back. Ten of the
  fifteen policies on the installed Lys are that seed; the five newest are empty. (W16, V3, V4, agreed)
- A tool refused by bare name is taken away from the model entirely: in that run's first real call Edit,
  Read, WebFetch and Write were not among the 103 tools offered. Bash was. (W4, V9, agreed; seen by Archie
  in the proxy's kept request, 20:31)

**Lys has two permission systems and the screens show neither truthfully.**
- Per agent, in its settings: three lists (without asking, ask first, refused), extra folders, one mode.
  The settings form shows only the mode; the add form saves only the mode. (W1, agreed)
- Per agent, a policy: refusal rules only (a whole tool, a path and everything under it, a web host),
  each either hard or liftable by a named grant. It has its own tab on the agent's file, with an add form
  that is free text throughout. (W17; seen in code, not yet in a browser: open)

**What actually enforces anything, for Claude Code.**
- The three lists and the hard rules are written into a settings file and enforced by Claude Code itself.
  (W1, W16, agreed as to what is written; that Claude Code then refuses a call has not been watched: V8, open)
- Lys's own judge, which would check each tool call against the policy, is never installed on a start
  made from the screen. A rule a grant may lift is therefore enforced by nothing. (W11, V1 agreed; V2 open
  on the real leg, since no installed policy has such a rule)
- Rules about shell commands match the command's text and, in the documentation's words, "aren't a
  security boundary". Rules about reading files do not stop a script the agent runs. Only the sandbox is
  enforced by the operating system, and only for shell commands. (W6, W7, W8: documentation only, open)
- Lys turns the sandbox on in one mode only: its own `workspace-only`, with no network and no way out. No
  run in that mode has been observed, and this computer's own settings may set the sandbox either way.
  (W10, V13, V6, open)
- By one page of the documentation, an installed mod can approve a call a refusal rule refuses, on a
  computer with no managed settings and no Team or Enterprise sign-in; another page says a refusal holds
  in every mode with no exception. The pages disagree, and nobody has looked at whether Tom's Mac has a
  mod. So even "refused" is not yet known to hold. (V15, W13, open)
- The person's own Claude Code setup reaches a Lys run. A plugin's hook ran inside pancake's 19:19 run;
  hooks run with the person's full access whatever the file says. (V5, V14: seen and in code; the
  documentation is silent on which sources' hooks run when a settings file is passed) And 70 of the 103 tools
  that run was offered came from Tom's own setup, not from Lys: 62 from his Argus plugin, 8 from his
  claude.ai connector. They include tools that type into and start other sessions on the computer. No Lys
  record names them. (W12, agreed; whether a call to one would have been allowed was not seen) Whether the
  person's own allow rules also widen what the agent may do is documentation and code only. (V6, open)
- A path rule typed with one leading slash would mean "beside the settings file", not the top of the
  disk. Lys writes its hard rules in the documentation's absolute form; nothing checks a rule a person
  types. (W9; V11 open as above)
- This arrangement is a choice made on 3 October, not how it has to be. Until some time that day a run
  had its own config folder, apart from Tom's: the 08:09 run's folder holds Claude Code's own state and no
  later run's does. The code's comment names Tom's ruling. (V7: seen; the earlier code is history not yet
  read, open)

**For Codex.**
- No Codex agent has ever been started by the installed Lys, so every row about a Lys Codex run is open
  on the real leg. (A, O4)
- A Codex agent cannot carry any rule about a tool, a file or a host: Lys refuses the start. A policy like
  pancake's makes a Codex start refuse. (A14, A15, W-C1: code only, open)
- With no mode chosen, Lys sets no sandbox and never sets when Codex asks; the run would use the login's
  own Codex config, which Lys neither sets nor reads. (A2, A3, A4, open)
- "Read-only" stops commands writing and using the network while they stay in the sandbox. It does not
  stop reading: every mode reads the whole disk. Seen with the installed Codex under the settings Lys
  writes. (A7, A8, agreed for the sandbox itself) Web search is not turned off under read-only; Lys turns
  it off only under workspace-write. (A11, open) And Codex itself tells the model to ask to rerun a blocked
  command outside the sandbox; whether that is granted is decided by the login's own approval setting,
  which Lys never sets. (A41, A3, open)
- By its documentation Codex hands commands the login's environment with names containing KEY, SECRET or
  TOKEN left in. (A34, documentation only, open)
- Lys's workspace-write settings do close `/tmp`, the temp folder and the network. Seen. (A9, agreed)
- The confinement Lys has written for Codex is called only by a test, and as written would not run a
  command and would not hold the network. (A16 agreed; A17, A18 seen)
- No judge is installed for Codex, and Codex's own pages say such a hook is not a wall. (A23, A26)
- Run with its Codex home pointed at a scratch folder holding only a marker instruction file, the
  installed Codex still listed 32 of the login's own skills to the model; and that marker file and the
  working folder's own instruction file both reached the model, even in Lys's "replace" mode. (A37 to A40,
  seen; no Lys run) So for Codex, giving a run its own config folder does not by itself make it start clean.
- The one place Codex offers a setting the login cannot undo is an administrator's requirements file.
  Documentation only; no Lys code writes it; not observed. (A33, open)

**In one sentence:** today Lys can refuse a Claude Code agent whole tools and paths through the program's
own settings file, and can box what a Codex agent's commands write and reach on the network while they
stay in the sandbox; everything else the screens imply is
either not enforced by anything, or enforced by something Lys does not control.

## What I propose (for Tom to mark; none of it is built)

**1. First, the screens stop saying anything untrue. Small, and safe.**
- Remove the mode sentence. In its place, for the chosen program, show "What Lys hands the program" at
  the next start: every refused, ask-first and without-asking rule by name, the mode by its real name with
  the documentation's own words for it, the extra folders, sandbox on or off. The server already renders
  this for a start; the screen shows that rendering, not a description of it.
- Under it, one plain block that claims no more than the rows support. For Claude Code: "This computer's
  own Claude Code settings, plugins and hooks also apply. Lys does not read them and does not check each
  action." For Codex with no mode chosen: "Lys has set nothing; this computer's own Codex settings
  decide." (A2, open) For Codex with a mode: "Lys sets where its commands may write and whether they may
  use the network, while they stay in the sandbox. It can read every file this login can read. Web search and leaving the
  sandbox are decided by this computer's own Codex settings." Nothing on the screen says a rule is enforced until a run has been watched
  being refused (V8).
- Show the policy on the settings form and the front page pane, not only on a tab of the file, with who
  set each rule. A seeded rule says "written by Lys on <date>, chosen by nobody".
- On each run, show what that run was handed, from the kept file.

**2. Then the permissions are made real, as their own piece with its own design page.** The order I
recommend, each to be proved by a watched run before the next:
- Install the judge on every Claude Code start made from the screen, and refuse the start if it cannot be
  installed. Its code exists; only the server's start path leaves it out.
- Decide what a Lys run inherits from the person's own setup. Tom ruled on 3 October that a run uses the
  machine's own Claude Code and Codex setup. That ruling and "a serious permission system" pull against
  each other: a plugin hook with full access ran inside tonight's agent. This is Tom's call (question 1).
- Turn the sandbox on by default for Claude Code agents, as `workspace-only` already does.
- For Codex: a choice between two things that by the documentation do not combine (A28, A29: seen for a mode given on the command line, where the profile was
  ignored and the denied read went through; the `--sandbox` flag itself and an agent run not seen): a sandbox mode, as today, or a permission profile, which is the only thing that can confine
  reads (A8) and which Lys's unused code gets wrong (A17, A18). Either way set the approval policy, but
  only once it has been seen what `never` does to a request to leave the sandbox (A41); and observe a
  requirements file holding against a looser login before leaning on it (A33).

**3. Granular rules and named profiles come on top of 2, not before it.** A permissions profile is a
named, saved set of rules, shown in full wherever it is chosen. Rules are chosen from what the program
accepts (tools by name from its own list, folders from the folder chooser, hosts), never typed as free
text. For Codex the honest granular offer today is small (mode, extra folders); the screen says so.
Building a rich editor before enforcement is real would be the same mistake again: a screen that
promises what nothing holds.

## Questions for Tom

1. Does a Lys-started agent keep running the person's own plugins, hooks and rules, or does a Lys run
   start clean? Clean is the only answer I can defend for a ward; it reverses part of the 3 October ruling
   and means Lys must hand the run everything it needs.
2. The seeded policy on ten agents: remove it from all of them, or keep it and show it? I recommend
   removing it, as one recorded act, because nobody chose it.
3. Do proposals 1 and 2 go in that order, with the start-from-the-screen piece (already written, not
   built) landing first so the product can be used at all?

## Before anything in 2 or 3 is built, these are observed, not assumed

- A Claude Code run watched refusing a refused call, and asking on an ask-first call. (V8, V12)
- Whether a call to one of the person's own tools is allowed in a Lys run. (W12)
- Whether the person's own allow rules merge into a Lys run. (V6)
- Whether this Mac has a mod, and whether a mod can lift a refusal. (V15)
- Whether `Read(///**)` is read as a rule. (V11)
- A run in `workspace-only`, with the sandbox seen to hold. (W10)
- A Codex agent started by Lys at all, in each mode. (A2 to A15)
- A requirements file refusing a looser login setting. (A33)
- What Codex's approval setting `never` does to a request to leave the sandbox. (A41)
- Whether a permission profile is ignored under the `--sandbox` flag itself, in an agent run. (A28, A29; seen for `-c sandbox_mode=`)

## The worst credible failure

If this were used in a hospital as it stands, the worst credible failure is that a person reads "asks
before most changes" or "read-only", trusts it, and the agent, or a plugin running inside it with the
person's full access, reads or changes records it was never meant to reach; and it could harm every
patient whose records that login can read.
