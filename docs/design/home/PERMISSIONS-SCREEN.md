# What an agent may do: the truth today, and what has to change

Waffles, 3 October 2026, rewritten 20:31 by the clock. The first version of this page (068c7c51) is
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
  and Read anywhere on the disk; allowed nothing; no sandbox; no hooks. (W3, V3, agreed)
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
- The sandbox is off in every mode but one. Lys's own `workspace-only` mode turns it on, with no network
  and no way out; no run in that mode has been observed. (W10, V13, open)
- The person's own Claude Code setup reaches a Lys run. A plugin's hook ran inside pancake's 19:19 run;
  hooks run with the person's full access whatever the file says. (V5, V14, agreed) And 70 of the 103 tools
  that run was offered came from Tom's own setup, not from Lys: 62 from his Argus plugin, 8 from his
  claude.ai connector. They include tools that type into and start other sessions on the computer. No Lys
  record names them. (W12, agreed; whether a call to one would have been allowed was not seen) Whether the
  person's own allow rules also widen what the agent may do is documentation and code only. (V6, open)
- A path rule typed with one leading slash would mean "beside the settings file", not the top of the
  disk. Lys's own hard rules come out right; nothing checks a rule a person types. (W9, V11)

**For Codex.**
- No Codex agent has ever been started by the installed Lys, so every row about a Lys Codex run is open
  on the real leg. (A, O4)
- A Codex agent cannot carry any rule about a tool, a file or a host: Lys refuses the start. A policy like
  pancake's makes a Codex start refuse. (A14, A15, W-C1: code only, open)
- With no mode chosen, Lys sets no sandbox and never sets when Codex asks; the run would use the login's
  own Codex config, which Lys neither sets nor reads. (A2, A3, A4, open)
- "Read-only" stops writes and the network. It does not stop reading: every mode reads the whole disk.
  Seen with the installed Codex under the settings Lys writes. (A7, A8, agreed for the sandbox itself)
- Lys's workspace-write settings do close `/tmp`, the temp folder and the network. Seen. (A9, agreed)
- The confinement Lys has written for Codex is called only by a test, and as written would not run a
  command and would not hold the network. (A16 agreed; A17, A18 seen)
- No judge is installed for Codex, and Codex's own pages say such a hook is not a wall. (A23, A26)
- Run with an empty config folder, the installed Codex still listed 32 of the login's own skills to the
  model, and both instruction files reached it even in Lys's "replace" mode. (A37 to A40, seen; no Lys run)
- The one place Codex offers a setting the login cannot undo is an administrator's requirements file.
  Documentation only; no Lys code writes it; not observed. (A33, open)

**In one sentence:** today Lys can refuse a Claude Code agent whole tools and paths through the program's
own settings file, and can box a Codex agent's writes and network; everything else the screens imply is
either not enforced by anything, or enforced by something Lys does not control.

## What I propose (for Tom to mark; none of it is built)

**1. First, the screens stop saying anything untrue. Small, and safe.**
- Remove the mode sentence. In its place, for the chosen program, show exactly what the next start will
  be handed: every refused, ask-first and without-asking rule by name, the mode by its real name with the
  documentation's own words for it, the extra folders, sandbox on or off. The server already renders this
  for a start; the screen shows that rendering, not a description of it.
- Under it, one plain block: "What enforces this". For Claude Code: the program enforces these rules;
  Lys does not check each action; your own plugins and hooks on this computer also run. For Codex: only
  the write and network box is enforced; it reads every file this login can read.
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
- For Codex: always set the mode and the approval policy; fix or delete the unused confinement; observe a
  requirements file holding against a looser login before leaning on it.

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
- A run in `workspace-only`, with the sandbox seen to hold. (W10)
- A Codex agent started by Lys at all, in each mode. (A2 to A15)
- A requirements file refusing a looser login setting. (A33)

## The worst credible failure

If this were used in a hospital as it stands, the worst credible failure is that a person reads "asks
before most changes" or "read-only", trusts it, and the agent, or a plugin running inside it with the
person's full access, reads or changes records it was never meant to reach; and it could harm every
patient whose records that login can read.
