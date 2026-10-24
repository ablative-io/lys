/**
 * The page help's content, after liminal's console (apps/liminal-console/src/shell/HelpOverlay.tsx): the pages
 * carry no explanation; this layer does. Each page names its regions by selector, with a numbered callout placed
 * beside each one that is on screen, and the keys that work there. A region whose element is not on the page is
 * skipped, so a page that has not loaded, or has nothing in a region, shows only what is there.
 *
 * Every sentence here is what the screen does today; when a screen changes, its entry changes with it.
 */

/** Where a callout sits beside its region (liminal's placements). */
export type At = 'below' | 'above' | 'left' | 'right' | 'in-tr' | 'in-bl' | 'c';

export interface Region {
  readonly sel: string;
  readonly title: string;
  readonly text: string;
  readonly at: At;
  /** Moves the callout down by this much, so two regions close together do not cover each other. */
  readonly dy?: number;
}

export interface PageHelp {
  readonly title: string;
  readonly regions: readonly Region[];
  /** The keys that work on this page, beyond the shell's own. */
  readonly keys: readonly (readonly [string, string])[];
  /** The full help's sheet for this page, opened by "Full help". */
  readonly topic: string;
}

/** The rail is on every page, so its region joins every page's. */
export const RAIL_REGION: Region = {
  sel: '#rail',
  title: 'The rail',
  text: 'Every screen, one key away: g then the letter beside its name. Jobs and the access map are found with Search. Help or ? opens this layer; Full help, below the notes, lists every topic. [ or Menu labels shows or hides the words.',
  at: 'right',
};

const ROWS: readonly (readonly [string, string])[] = [
  ['j / k', 'move down / up the list'],
  ['Enter', 'open the row'],
];

/** The fields every agent's settings share, on Add an agent, the Settings tab and the Overview's model and computer. */
const PROFILE_FIELDS: readonly Region[] = [
  { sel: 'label:has(select[name="program"])', title: 'Program', text: 'The AI program this agent is run with, Claude Code or Codex. Only programs the chosen computer’s runner found are listed. Changing it resets the model, the mode and the prompt choice to that program’s.', at: 'right' },
  { sel: '.first-run-copy', title: 'The copy Lys found', text: 'Lys found this program on the computer and will start exactly this command. To use a different copy, open More settings and choose it under Installed copy.', at: 'right' },
  { sel: 'label:has(select[name="build"])', title: 'Installed copy', text: 'Which installed copy of the program is started, by the command that starts it, grouped by computer. Choosing one from another computer’s group moves the agent to that computer. Another program lets you type a command or full path when Lys did not find it.', at: 'right' },
  { sel: 'label:has(input[name="program-path"])', title: 'Command or path', text: 'The command you would type in Terminal to start the program, such as claude, or its full path. The agent is started with exactly this.', at: 'right' },
  { sel: 'label:has(select[name="model"])', title: 'Model', text: 'The model the program is started with. Only models the program lists are offered. It is fixed at start and holds for the whole session.', at: 'right' },
  { sel: 'label:has(select[name="runs_on"])', title: 'Computer', text: 'The computer whose runner starts this agent; folders, installed copies and plugins are read from it. Settled when it starts leaves the choice to the moment you press Start.', at: 'right' },
  { sel: 'table[aria-label="How it works"]', title: 'Mode', text: 'How much the program may do without asking, one chosen row: kept to its folder, asks before it acts, reads only, plans only, or no limits (marked ⚠ because it can change anything on the computer). What that means, beside each, says exactly what the program does in that mode.', at: 'right' },
  { sel: 'table[aria-label="Rules"]', title: 'Rules', text: 'One row per rule: what it covers (a tool, a command, a file, a web address), the exact rule, and What happens when the agent tries it: allowed without asking, asked about, or refused. Enforced by says whether the program itself or Lys’s policy applies it; policy rules take effect at the next start after Save policy.', at: 'right' },
  { sel: 'tr[aria-label="Add a rule"]', title: 'Add a rule', text: 'Choose what the rule is about, type the exact thing (the command, path or address), choose what happens when it is tried, and press Add. It is kept with the rest of the settings when you Save.', at: 'right' },
  { sel: 'table[aria-label="Extra folders"]', title: 'Extra folders', text: 'Folders beyond its working folder that the agent may also read and change.', at: 'right' },
  { sel: '.mode-words', title: 'In plain words', text: 'What the chosen mode and rules add up to for this program, as it will behave when it runs.', at: 'right' },
  { sel: 'section[aria-label="Tools named in these settings"]', title: 'Tools', text: 'Tools the program may use without asking, as its saved settings name them. Remove takes one off; nothing is added here.', at: 'right' },
  { sel: 'section[aria-label="Plugins named in these settings"]', title: 'Plugins', text: 'Claude Code plugins turned on for this agent’s runs. Each must already be installed on its computer; Lys reads whether it is, and a start is refused while one is missing.', at: 'right' },
  { sel: 'section[aria-label="Connected tools"]', title: 'Connected tools', text: 'MCP servers the agent is started with: each by name and how it is reached, a web address or a program on its computer. Add one in the last row; Remove takes one off. A server marked as waking the agent wakes it when a message arrives.', at: 'right' },
  { sel: 'section[aria-label="Skills"]', title: 'Skills', text: 'Skills Lys keeps and can give this agent: tick each to give it. Codex also uses the skills set up on its own computer.', at: 'right' },
  { sel: 'label:has(select[name="prompt"])', title: 'System prompt', text: 'Whether the agent runs with the program’s own prompt, with your words added to it, or with your words in its place. Choose Add or Replace and the box for the words appears.', at: 'right' },
  { sel: 'label:has(textarea[name="instructions"])', title: 'The words', text: 'What is added to, or used instead of, the program’s prompt. Optional when adding; required when replacing.', at: 'right' },
  { sel: '.works-in', title: 'Working folder', text: 'The folder the agent works in on its computer: what it reads and changes in the kept-to-its-folder mode. Choose opens that computer’s folders; pick one or type a path and press Open.', at: 'right' },
  { sel: 'label.resume-field', title: 'Continue a conversation', text: 'Leave empty to start fresh. Paste a Claude Code session id or a Codex thread id and the agent carries that conversation on, in the folder above, when it next starts.', at: 'right' },
  { sel: 'form.save-settings', title: 'Save', text: 'Keeps every setting on this page; the agent uses them the next time it starts, and a running session is not changed. Starts are refused until the settings are saved. When Save is greyed, the reason is written beside it.', at: 'right' },
];

/** The sections of a person's or agent's page: the same seven, each with its own address. */
const SECTIONS_REGION: Region = { sel: 'nav.tabs[aria-label="Agent sections"], nav.tabs[aria-label="Sections"]', title: 'Sections', text: 'Overview (who it is, Start), Settings (how it runs), Access (what it may reach), Limits and goals (spending, context, goals, messages), Sessions (what has run), Credentials (secrets it may use), Record (every change, signed). Keys 1 to 7; a person has no Settings.', at: 'below', dy: 30 };

const HEAD_REGION: Region = { sel: '.file .head', title: 'Name and state', text: 'Its access state: active, not turned on yet, paused or retired. That says what it may do in Lys, not whether it is running. The ⋯ menu suspends, restores or retires it, and asks for a reason each time; retiring is not undone.', at: 'below' };

export const PAGES: Readonly<Record<string, PageHelp>> = {
  dashboard: {
    title: 'Dashboard',
    topic: 'people.dashboard',
    regions: [
      { sel: 'section.getting-started', title: 'Getting started', text: 'What this Lys still needs, in order: where agents run, Google sign-in (optional), an app to connect, and your first agent. A step leaves the list once it is done; Not this one counts as an answer.', at: 'below' },
      { sel: 'form[aria-label="Run agents on the Lys computer"]', title: 'Run agents on the Lys computer?', text: 'Yes, run agents here adds the computer Lys runs on, under the name typed, so agents can be started on it. Not this one records that it is not used; you can add another under Computers.', at: 'right' },
      { sel: 'section.dash-agents', title: 'Agents', text: 'Your agents, grouped by team. The dot says whether each is running; Budget is how near it is to its nearest limit; Goals are what it is working towards. Start, Watch and Stop sit at the end of each row; pressing the row opens its page.', at: 'in-bl' },
      { sel: '[data-act="stop-everything"]', title: 'Stop all', text: 'Ends every running agent on every computer at once, after asking why. Agents stay stopped until an administrator presses Start again.', at: 'left' },
      { sel: 'section[aria-label="Waiting for you"]', title: 'Waiting for you', text: 'Access requests to decide, drafts your agents want approved, and grants due a review; each row opens its list. A ? means that count could not be read; press the words beside it to see why.', at: 'left' },
      { sel: 'section[aria-label="Running now"]', title: 'Running now', text: 'Every live session, with the computer it is on. Watch shows it in a small picture here; the picture’s title opens it full size in Running agents. Stop ends that one session after asking why.', at: 'left' },
      { sel: '[aria-label="Alarms"]', title: 'Alarms', text: 'What Lys could not do or saw go wrong: a start refused, a runner not answering, a limit hit. Acknowledge marks one as seen; Show all lists older ones.', at: 'above' },
      { sel: '.dash-widgets', title: 'Readings', text: 'Alarms, the requests and drafts waiting for you, how near each agent is to its limits (the bar), and every goal with its deadline. Open goes to the whole list; an agent’s name opens its Limits and goals.', at: 'above' },
    ],
    keys: [],
  },
  people: {
    title: 'People and agents',
    topic: 'people.kind',
    regions: [
      { sel: '.page-header .head-actions', title: 'Add', text: 'Add agent asks a name, who it answers to, the program and the folder, and can start it at once. Add person asks a name and an email and writes to them with where to sign in.', at: 'below' },
      { sel: 'nav[aria-label="People and agents views"]', title: 'Views', text: 'All; People (who signs in); Agents (what they run); Teams; Service accounts (apps and tools that act with their own credential); Found (agents a runner reported that Lys has no record of). Each view has its own address.', at: 'below', dy: 30 },
      { sel: '.stat-strip', title: 'At a glance', text: 'How many people and agents are active, and how many agents have no active person answering for them: those cannot have their access renewed until someone takes them on.', at: 'below', dy: 60 },
      { sel: '.listing', title: 'The list', text: 'A person’s row carries their agents folded under it; an agent’s row names who it answers to and its state. Press a row, or j, k and Enter, to open its page. Retired records are hidden until Show retired.', at: 'c' },
    ],
    keys: ROWS,
  },
  'add-agent': {
    title: 'Add an agent',
    topic: 'start.agent',
    regions: [
      { sel: 'label:has(input[name="display_name"])', title: 'Name', text: 'What the agent is called everywhere in Lys and in the apps it reaches. Two agents of one person cannot share a name. The name starts empty; type one.', at: 'right' },
      { sel: 'label:has(select[name="computer"])', title: 'Computer', text: 'The computer this agent runs on. Only computers with a connected runner are offered; its folders and installed programs are read from there.', at: 'right' },
      { sel: 'label:has(select[name="answers_to"])', title: 'Who it answers to', text: 'The person responsible for this agent: the only one who can give it access, and who is told when it hits a limit. Show retired and suspended lists people who no longer can.', at: 'right' },
      { sel: 'label:has(select[name="team"])', title: 'Team', text: 'The team the agent joins, which decides where it is grouped on the dashboard and which computers a team owns it may use.', at: 'right' },
      { sel: 'fieldset.agent-grants-scope', title: 'Its access in Lys', text: 'Tick what the agent may do, grouped by what it is on. Only access you hold and may pass to agents is offered; a greyed row says why it cannot be given. An app’s channels appear once the app is approved on Apps. An agent with nothing ticked starts with no access.', at: 'right' },
      { sel: '.first-run-mode', title: 'Careful mode', text: 'This program has no mode that keeps it to its folder with the internet off, so it starts in its most careful mode, named here. Change it under More settings.', at: 'right' },
      { sel: 'details.more-settings > summary', title: 'More settings', text: 'Everything a first run does not ask: installed copy, model, mode and rules, tools, plugins, connected tools, skills, prompt. Open it to change any of them; what you leave keeps its default.', at: 'right' },
      ...PROFILE_FIELDS,
      { sel: 'form.add-agent > p:last-of-type', title: 'Add', text: 'Add and run it adds the agent and starts it at once on the computer you chose, once it has a folder to work in. Add agent alone adds it; you start it from its page. If this browser kept an earlier unanswered request, Lys checks that first so the agent is not added twice.', at: 'right' },
    ],
    keys: [],
  },
  'add-person': {
    title: 'Add a person',
    topic: 'start.person',
    regions: [
      { sel: 'label:has(input[name="given_name"]), label:has(input[name="display_name"])', title: 'Name', text: 'First and last name, as they are shown everywhere in Lys.', at: 'right' },
      { sel: 'label:has(input[name="email"])', title: 'Email', text: 'Their sign-in name. If they will sign in with Google, this must be their Google email, or Lys cannot match them.', at: 'right' },
      { sel: 'label:has(select[name="kind"])', title: 'Give them access to', text: 'Optional first grant: what kind of thing, which one, and as what (their level on it). They can start with none and ask later.', at: 'right' },
      { sel: 'form[aria-label="Add a person"] .btn, form[aria-label="Add a person"] button[type="submit"]', title: 'Add', text: 'Adds them and writes to them at their email with where to sign in; the receipt says who wrote.', at: 'right' },
      { sel: 'section[aria-label="Receipt"]', title: 'Letter', text: 'The person is added and the letter to them has gone, from Lys or from the sign-in service when it made their account. What Lys recorded lists the signed entries.', at: 'right' },
    ],
    keys: [],
  },
  file: {
    title: 'Overview',
    topic: 'people.agent-file',
    regions: [
      HEAD_REGION,
      SECTIONS_REGION,
      { sel: 'section[aria-label="Run"]', title: 'Start, Restart, Stop', text: 'Whether the agent is running now. Start begins a session on its computer in its folder with its saved settings; if it has no program yet, Choose opens Settings. Restart ends the session and starts a new one; Stop ends it. Both ask first.', at: 'in-tr' },
      { sel: '.agent-facts', title: 'About this agent', text: 'Name (Edit changes it, the record keeps its history), when it was added, who it answers to, its team, and the model and computer it is set to use; those two can be changed and saved here.', at: 'right' },
      { sel: 'section.computer-admission', title: 'Where it can run', text: 'Tick each computer this agent may be started on; an administrator changes these. Allowed through role means a role it holds already lets it run there.', at: 'right' },
      { sel: '[aria-label="Personal budgets"]', title: 'Limits across their agents', text: 'For a person: the limits that apply across every agent answering to them, and where each stands.', at: 'right' },
      { sel: 'section[aria-label="Read problems"]', title: 'What could not be read', text: 'Parts of this page Lys could not read, each with the refusal and reason. What is shown is still true; what is missing is named here.', at: 'left' },
    ],
    keys: [['1 to 7', 'switch sections']],
  },
  'file:access': {
    title: 'Access',
    topic: 'access.give',
    regions: [
      SECTIONS_REGION,
      { sel: '.file-section-head', title: 'Give', text: 'Passes on access you hold and may pass to agents. Only the agent’s responsible person can give it access; if you hold nothing passable, Lys says so.', at: 'below' },
      { sel: '.grant-table', title: 'Grants it holds', text: 'Every grant held by this agent: what it may do, on what, from whom, until when, and whether it stands now. Give passes part of one on; Take back withdraws one after asking why.', at: 'right' },
      { sel: '[data-standing-choice="all"]', title: 'Set up and run agents', text: 'One choice, given once: lets this agent register, set up and review your agents and run them on your computer. For an administrator who holds nothing to give yet, Give records the administrator’s roots first (the agents and apps directories, their own person record and the computer Lys runs on), then gives the five grants, the fifth letting it issue your agents’ certificates. It gives no access to an app’s channels; those need a separate grant you hold and may pass to agents.', at: 'right' },
      { sel: '[data-organisation-standing]', title: 'Standing in the organisation', text: 'Shown to whoever holds organisation.grant there: gives this agent standing in the organisation itself, administrator among them, the same give as Give access on the Access page. Every act under it is on the record as the agent’s.', at: 'right' },
      { sel: '.access-halves', title: 'What it can reach, and a check', text: 'Left: everything this agent can reach, summed up from its grants. Right: ask whether it can do one thing; the answer shows the chain of grants back to a person, or says why not.', at: 'above' },
    ],
    keys: [['c', 'ask the check']],
  },
  'file:budgets': {
    title: 'Limits and goals',
    topic: 'budgets.limit',
    regions: [
      SECTIONS_REGION,
      { sel: 'input[name="spending_amount"]', title: 'Spending limit', text: 'The most this agent may spend, in dollars, in the period beside it (a 5-hour window, a day, a week or a month). When the spend reaches it, what happens is the act chosen beside it. It applies from the moment you press Save. Clear the box and Save to remove it.', at: 'right' },
      { sel: 'form[aria-label="Edit budget limits"] table', title: 'Limits in force', text: 'One row per limit. Limit is the amount and what it counts: dollars, tokens, % of your plan, minutes running, or % of its context (how full the conversation window is). Used shows how far along it is now. Change an amount or what happens, then Save.', at: 'right' },
      { sel: 'form[aria-label="Edit budget limits"] select[name^="act-"]', title: 'When it’s hit', text: 'What Lys does the moment a limit is reached: Type a notice into the session warns the agent and lets it carry on; Compact the session squeezes its conversation so it can continue; End the session stops it; Tell the responsible person messages whoever answers for it.', at: 'right' },
      { sel: 'input[name="warn_at"]', title: 'Warn me at', text: 'A percentage of each limit at which Lys tells you it is getting close, before anything is done to the session. Leave it blank for no early warning.', at: 'right' },
      { sel: 'form[aria-label="Set a budget"], input[aria-label="Limit"]', title: 'Add a limit', text: 'Type an amount, choose what it counts and over what period, choose what happens when it is hit, and press Add. Context limits have no period: they apply at any moment.', at: 'right' },
      { sel: '[aria-label="Context default"]', title: 'Context levels for every agent', text: 'When this agent has no context limits of its own it follows the default: a warning typed into the session at 20% and 38% of its context window, and a compaction at 40%, at the first turn boundary past that level. Change for this agent copies these levels into the table above as this agent’s own. Change the default changes them for every agent that has none; only an administrator can.', at: 'right' },
      { sel: 'section.usage-goals', title: 'Goals', text: 'What this agent is working towards, in your words; each is sent to it as a reminder. A goal with a deadline shows how long is left. Untick Active to pause one without losing it; Reword changes its words.', at: 'right' },
      { sel: 'form[aria-label="Set a goal"], section.usage-goals tfoot', title: 'Set a goal', text: 'Type the goal in the last row, pick a deadline if it has one, and press Set or Enter.', at: 'right' },
      { sel: 'section.usage-words', title: 'Messages it is sent', text: 'The exact words Lys types into this agent’s session at each moment: the context warning, the preparation before compaction, the compaction command itself, the wake-up after compaction, and a scheduled reminder. Each row shows what it is set to now. Preview shows the words as they would be sent, filled in, without sending them.', at: 'right' },
      { sel: 'section.usage-words select', title: 'Whose words', text: 'The template every agent gets, another named template, or this agent’s own words you type below. Save keeps the choice for this agent alone.', at: 'right' },
      { sel: 'section.usage-variables', title: 'Variables', text: 'Named values Lys fills into its messages to this agent, such as a ward or a project name. Give a name, a value and, if it should lapse, a date to keep it until; empty keeps it until you remove it.', at: 'right' },
      { sel: '[aria-label="Schedules"]', title: 'Schedules', text: 'Messages Lys sends this agent on a timer. In the last row: the message, the first time it is sent, how many minutes between sends (empty sends it once), and at most how many times (empty for no limit). Stop ends a schedule for good after asking once.', at: 'right' },
      { sel: '[aria-label="Managed controls"]', title: 'Managed controls', text: 'A record of every message Lys sent into a running session: notices, compactions, reminders. Each says whether the agent’s program confirmed receiving it. When that is uncertain, Seen and Unseen let you record whether you saw it arrive in the session, and Resend sends it again. None of this says the agent acted on it.', at: 'left' },
    ],
    keys: [],
  },
  'file:provisioning': {
    title: 'Settings',
    topic: 'people.settings',
    regions: [SECTIONS_REGION, ...PROFILE_FIELDS, { sel: '.pane .head:has(h2)', title: 'Refused tool calls', text: 'What this agent’s policy stopped, newest first: the tool, what it tried and which rule refused it. Reading it is how you see whether a rule is too tight.', at: 'right' }],
    keys: [],
  },
  'file:sessions': {
    title: 'Sessions',
    topic: 'people.agent-file',
    regions: [SECTIONS_REGION, { sel: '.pane table', title: 'Sessions', text: 'Every session of this agent its runners reported: the computer, its state (running, ended, not answering) and the last report. A session that started and stopped straight away shows why here.', at: 'right' }, { sel: '.pane .head', title: 'Memory and home', text: 'Where this agent’s memories came from and the last context it was given at a start: the session, the starting point and what was sent.', at: 'right' }, { sel: 'section[aria-label="Your signed-in sessions"], section.card.you-sessions', title: 'Signed-in sessions', text: 'For a person: each browser they are signed in from. End signs that browser out; it does not stop any agent.', at: 'right' }],
    keys: [],
  },
  'file:credentials': {
    title: 'Credentials',
    topic: 'access.sealed',
    regions: [
      SECTIONS_REGION,
      { sel: '.pane section.card', title: 'Credential handles', text: 'Secrets this agent may use through Lys. It is given a handle, never the value: Uses counts how often, Spending what it cost, Works until when it lapses. End withdraws a handle and every handle passed on from it, after asking.', at: 'right' },
      { sel: 'section.certificates', title: 'Capability certificates', text: 'A certificate records what the agent held when it was issued, for systems that check a certificate instead of asking Lys. Issue takes a signing request the agent made with its own key; Withdraw ends one after asking why. Current permissions are always checked in Access, not here.', at: 'right' },
      { sel: 'form[aria-label="Issue capability certificate"]', title: 'Issue a certificate', text: 'Paste the PKCS#10 request the agent made with its own key. Its common name must be the agent’s id. Issue signs it with what the agent holds now.', at: 'right' },
      { sel: 'section[aria-label="Sign-in accounts"], .pane section.card:has(h2)', title: 'Sign-in accounts', text: 'For a person: the accounts that sign them in (a provider and an account id); passwords are never shown. Connect a sign-in account links a Google or other account to this person.', at: 'right' },
    ],
    keys: [],
  },
  'file:record': {
    title: 'Record',
    topic: 'people.agent-file',
    regions: [SECTIONS_REGION, { sel: '.grid2 > div:first-child .card', title: 'What changed', text: 'Every change to this record, oldest first: what changed, to what, by whom and when, with the reason given. Nothing here can be edited or removed.', at: 'right' }, { sel: '.grid2 > div:last-child .card', title: 'Proof of each change', text: 'Lys signs every change and keeps it for good. Download gives the signed entry for one change, to hand to anyone who asks for proof.', at: 'left' }, { sel: 'section[aria-label="Emergency-stop history"]', title: 'Emergency-stop history', text: 'Earlier stops of this agent: who pulled the cord, why, and what each stop did. Outcome not confirmed means Lys asked but never heard back. This is history, not the agent’s state now.', at: 'right' }],
    keys: [],
  },
  start: {
    title: 'Start',
    topic: 'start.agent',
    regions: [{ sel: 'section[aria-label="Run"]', title: 'Start', text: 'Starts the agent on its computer, in its folder, with its saved settings. If more than one computer is allowed, choose which. If it has no program chosen yet, Choose opens its Settings. Started and stopped straight away means the runner refused it; the link shows what it reported.', at: 'right' }, { sel: 'section[aria-label="Settings of this agent"]', title: 'Set-up before the first start', text: 'The folder and program a first run must know. Save keeps them, then Start works.', at: 'right' }, ...PROFILE_FIELDS],
    keys: [],
  },
  network: {
    title: 'Computers',
    topic: 'computers.network',
    regions: [
      { sel: '.page-header .head-actions', title: 'Add a computer', text: 'Puts the keyboard in the add row: name the computer, say whether it is this one or another, and Add. Another computer is given a one-time connection code to type there.', at: 'below' },
      { sel: '.stat-strip', title: 'At a glance', text: 'Computers whose runner is up now, computers that need attention (not answering or never connected), and how many agents are running across them.', at: 'below', dy: 30 },
      { sel: '.body.halves .listing', title: 'Computers', text: 'Each computer with its state, what is running on it and who may start there. Not answering filters to the ones Lys cannot hear. Press a row to open it.', at: 'in-bl' },
      { sel: '[aria-label="Running now"]', title: 'Running now', text: 'The agents running on the chosen computer; Watch opens each in Running agents.', at: 'left' },
      { sel: '[aria-label="May start here"]', title: 'May start here', text: 'Who may start an agent on this computer and through what: a direct permission or a role. Below it, allow one of your own agents here.', at: 'left' },
      { sel: 'form[aria-label^="Team of"]', title: 'Team', text: 'The team that owns this computer; its agents may run here. Save keeps the choice.', at: 'left' },
      { sel: '.join-code', title: 'Connect this computer', text: 'Choose what kind of computer it is, then download its installer and open that file on the computer. The installer carries a one-time connection for this computer alone, connects it, and keeps its runner starting at every login and updating itself. Each download replaces any connection given before. People who prefer a terminal can ask for a code instead.', at: 'right' },
      { sel: '.machine-retire', title: 'Retire', text: 'No agent can be started on this computer afterwards; agents already running on it keep running. Retire asks once.', at: 'left' },
      { sel: '[data-machine]', title: 'Machine identity', text: 'The runner’s own signing key and the grants held by the machine itself. Give a grant passes part of one of yours to the machine.', at: 'left' },
    ],
    keys: ROWS,
  },
  apps: {
    title: 'Apps',
    topic: 'apps.apps',
    regions: [
      { sel: 'nav[aria-label="Apps"]', title: 'Which app', text: 'Every app Lys knows, by state: approved, waiting for approval, declined or retired. Choose one to see it.', at: 'below', dy: 30 },
      { sel: 'section[aria-label="Connect an app"]', title: 'Connect an app', text: 'The apps Lys knows how to connect, then Another app: any app that connects to Lys, by its web address. Connect asks once for the app’s web address, sends you to the app, and brings you back to approve what it asks for.', at: 'below', dy: 30 },
      { sel: '.app-card', title: 'The app', text: 'Its state, the permission template it asked for, its sign-in settings and its client credentials. Nothing it registers takes effect until approved here. Decline takes an optional reason in Why.', at: 'left' },
      { sel: '.app-waiting', title: 'A change waiting', text: 'The app asked to change its permission template. Approve applies it; Decline refuses it with your reason. Until then the earlier template stands.', at: 'left' },
      { sel: '.app-signin', title: 'Sign-in', text: 'The return addresses the app’s sign-in may send a person back to, one to a line, and whether the app is given the person’s name. An https address may name every machine of a domain with *. as its first label (https://*.example.com/signin/return); http addresses are admitted only on this machine. Save replaces the list with what is in the box. An app may declare its own return address with its credential, and such an address is shown here with who declared it and when. The list saved here is the whole list: saving it without a declared address removes it.', at: 'left' },
      { sel: '.app-credentials', title: 'Client credentials', text: 'What the app presents to Lys to act as itself. Issue makes a new one and shows its secret once; copy it into the app and press Done, it is never shown again. Take back ends one after asking why.', at: 'left' },
      { sel: '.app-retire', title: 'Retire', text: 'Retire for good ends every credential and grant of the app; it cannot be undone. Cancel keeps it.', at: 'left' },
      { sel: '[aria-label="Add your own app (for developers)"]', title: 'Register an app', text: 'For an app of your own: its name, id and the return addresses its sign-in uses; an https address with *. as its first label names every machine of a domain, and http addresses are admitted only on this machine. It then appears above, waiting for approval like any other.', at: 'above' },
      { sel: '.sb', title: 'Permission template', text: 'The kinds of thing the app keeps permissions on, the relations on each kind and what each relation may do. Check asks Lys what the change would do before it is sent; the test bench asks a draft question against example holdings.', at: 'left' },
    ],
    keys: [],
  },
  'apps-connect': {
    title: 'Connect an app',
    topic: 'apps.apps',
    regions: [
      { sel: 'section[aria-label="Connect an app"] h3', title: 'The app', text: 'The app that sent you here, by the name in its manifest: the description it publishes of itself and what it asks for. Lys read it from the app’s own address.', at: 'right' },
      { sel: '.app-addresses', title: 'Where it sends people', text: 'Its address, and the return address it sends a person back to after they sign in, and whether it is given their name.', at: 'below' },
      { sel: 'section[aria-label="Connect an app"] table', title: 'The permissions it asks for', text: 'Each kind of thing the app keeps permissions on, and what can be done to it. Approving approves all of them; none takes effect before.', at: 'right' },
      { sel: 'section[aria-label="Connect an app"] .app-actions', title: 'Approve or Decline', text: 'Approve connects the app in one act and sends you back to it. Decline sends you back to the app, which is told no; nothing it asked for takes effect.', at: 'below' },
    ],
    keys: [],
  },
  access: {
    title: 'Access',
    topic: 'access.grant',
    regions: [
      { sel: 'nav[aria-label="Access views"]', title: 'Views', text: 'Grants (every grant you may see); Ask (can someone do something?); Requests (access people asked for); Drafts (what agents want approved); Reviews (grants due a look); Resources (everything grants are on); Model (the permission rules themselves).', at: 'below' },
      { sel: '.grant-table', title: 'Grants', text: 'Who holds what, on what, from whom, until when, and whether it stands now. Last used is an exercise seen where access is enforced; not seen never means never used. Give passes part of a grant to an agent; Take back withdraws one after asking why.', at: 'c' },
      { sel: '.seg', title: 'Which question', text: 'Can: can this person or agent do this on that? Reach: everything one person or agent can reach. Who: everyone who can reach one thing. Give: give access from scratch, as an administrator.', at: 'below' },
      { sel: '[aria-label="Question"]', title: 'Ask', text: 'Choose who, what and on what, then Check (or c). Every yes shows the chain of grants back to a person; every no says why.', at: 'below' },
      { sel: 'select[aria-label="Resource"]', title: 'Which thing', text: 'The thing to ask about; the list below shows everyone who can reach it and how.', at: 'below' },
      { sel: 'form[aria-label="Give access"], .recorded-form', title: 'Give access', text: 'Person, what kind of thing, which one, what they may do (or a role that bundles it), whether they may pass it on to their agents, and when it expires. Mode, under More, is how it is enforced. Give access records it signed.', at: 'right' },
      { sel: '[aria-label="Restricted places"]', title: 'Restricted places', text: 'Places in connected apps that are held apart from the rest: access to them is given one by one and never implied by access to the whole.', at: 'left' },
    ],
    keys: [['c', 'ask the check']],
  },
  resources: {
    title: 'Resources',
    topic: 'access.grant',
    regions: [
      { sel: 'nav[aria-label="Access views"]', title: 'Views', text: 'Resources sits beside Grants, Ask, Requests, Drafts, Reviews and Model.', at: 'below' },
      { sel: '#screen .pane, #screen .listing', title: 'Resources', text: 'Everything a grant can be on, by kind and app: Lys’s own kinds and those of every approved app. Open one to see who reaches it.', at: 'c' },
    ],
    keys: ROWS,
  },
  model: {
    title: 'Model',
    topic: 'access.grant',
    regions: [
      { sel: 'nav[aria-label="Access views"]', title: 'Views', text: 'Model sits beside Grants, Ask, Requests, Drafts, Reviews and Resources.', at: 'below' },
      { sel: '#screen .pane, .schema-tables', title: 'The permission model', text: 'The rules every answer comes from: for each kind of thing, its relations (owner, editor, viewer and so on) and what each relation may do, with a tick where it may. Roles are listed with what each carries. This is read here; it is changed by each app’s template on Apps.', at: 'c' },
    ],
    keys: [],
  },
  drafts: {
    title: 'Drafts',
    topic: 'access.drafts',
    regions: [
      { sel: 'nav[aria-label="Access views"]', title: 'Views', text: 'Drafts sits beside Grants, Ask, Requests, Reviews, Resources and Model.', at: 'below' },
      { sel: 'table.drafts', title: 'Drafts', text: 'What your agents prepared for you to decide: which agent, what it wants to do, its note, and the request exactly as it would be sent (open Prepared request). Nothing is done until you Approve; Decline asks why, and the agent is told.', at: 'c' },
      { sel: '[aria-label="Held for products"]', title: 'Held for products', text: 'Requests a connected app held for a person to approve: the product, what it wants, who holds it, and who approved it. Approve lets it run; Decline stops it with your reason.', at: 'above' },
    ],
    keys: [],
  },
  requests: {
    title: 'Requests',
    topic: 'access.requests',
    regions: [
      { sel: 'nav[aria-label="Access views"]', title: 'Views', text: 'Requests sits beside Grants, Ask, Drafts, Reviews, Resources and Model. The rail’s Access count includes requests waiting.', at: 'below' },
      { sel: '.page-header .head-actions, [name="Ask for access"]', title: 'Ask', text: 'Ask for access yourself: what you need, why, and until when. Someone who can grant it reviews the request.', at: 'below', dy: 30 },
      { sel: 'form[aria-label="Ask for access"]', title: 'Ask for access', text: 'Choose the thing and the access needed (or tick What I need is not in the list and name it), say why, and give an end date or tick No expiry requested. Request sends it to whoever can grant it.', at: 'right' },
      { sel: 'article[aria-label="Request"]', title: 'A request', text: 'Who asked, what access on what, why, and until when. Approve chooses which grant of yours it comes from and asks for a reason; Decline asks for a reason. The asker is told either way.', at: 'right' },
    ],
    keys: [],
  },
  reviews: {
    title: 'Reviews',
    topic: 'access.reviews',
    regions: [
      { sel: 'nav[aria-label="Access views"]', title: 'Views', text: 'Reviews sits beside Grants, Ask, Requests, Drafts, Resources and Model.', at: 'below' },
      { sel: '[aria-label="Agents needing an active owner"]', title: 'Agents needing an owner', text: 'Agents whose responsible person is suspended or retired. Their access cannot be renewed until someone active takes them on; open each to change who it answers to.', at: 'right' },
      { sel: '.listing', title: 'Grants due a review', text: 'Grants whose review date has come, or that have not been looked at for a long time. Choose one to see it whole.', at: 'in-bl' },
      { sel: '[aria-label="Grant to review"]', title: 'The grant', text: 'Who holds it, on what, from whom, when it was last used. Approve records that it was reviewed and stands, with an optional note; Take back withdraws it after asking why.', at: 'left' },
    ],
    keys: ROWS,
  },
  roles: {
    title: 'Jobs',
    topic: 'access.grant',
    regions: [
      { sel: '#screen .page-header', title: 'Jobs', text: 'A job describes work: its responsibilities, goals, practice and starting instructions for an agent, plus the access it usually needs. Giving someone a job hands them those words; it gives no access by itself.', at: 'below' },
      { sel: '.listing', title: 'The jobs', text: 'Every job with its latest version. Choose one to read it, give it to someone or publish a new version.', at: 'in-bl' },
      { sel: '[aria-label="Job description"], [aria-label="Role words"]', title: 'The job’s words', text: 'The version shown: responsibilities, goals, practice and the starting instructions an agent is given. If a newer version exists, it says so.', at: 'left' },
      { sel: '[aria-label="Access templates"]', title: 'Access this job usually needs', text: 'The grants normally given with the job: the kind of thing, which one, the access level, and how many days it usually lasts. A template is a guide for whoever grants; it does not grant.', at: 'left' },
      { sel: '[aria-label="Version history"]', title: 'Version history', text: 'Every version, who made it and why. Read shows an older one above.', at: 'left' },
      { sel: '[aria-label="Holders"]', title: 'Doing this job', text: 'Each person or agent with this job and the version they hold. They keep that version until an administrator applies a newer one; Apply shows what changed first. Remove ends the assignment after asking.', at: 'left' },
      { sel: 'form[aria-label="Assign job"], form[aria-label="Assign role"]', title: 'Give this job to someone', text: 'Find a person or agent, give an end date or none, and Assign. They hold the latest version from then.', at: 'left' },
      { sel: 'form[aria-label="Create job"], form[aria-label="Publish job version"], form[aria-label="Create role"], form[aria-label="Publish role version"]', title: 'Create or save a version', text: 'The job’s name (new jobs only), its four parts, the access templates, and the reason for this version, which is kept in its history. Save publishes a new version; holders keep theirs until it is applied.', at: 'left' },
    ],
    keys: ROWS,
  },
  secrets: {
    title: 'Secrets',
    topic: 'access.sealed',
    regions: [
      { sel: 'nav[aria-label="Secrets views"]', title: 'Views', text: 'Entries (every secret), Access (who may see and hand on each) and Activity (every use, by handle).', at: 'below' },
      { sel: 'form[aria-label="Add a secret"]', title: 'Add a secret', text: 'A name, its kind (a credential or a key), where it is used (the address it is sent to), the header it travels in and any text before it, and its value, typed once and never shown again. Add seals it.', at: 'right' },
      { sel: '#screen table', title: 'Secrets', text: 'Each secret by name: who owns it, who may see it, and its state. A value is never shown, and seeing a secret here does not give permission to use it. Manage opens who can see it, who it can be handed to, its value, and retiring it.', at: 'c' },
      { sel: '.secret-controls', title: 'One secret', text: 'Who can see this secret and who it can be handed to (Save each). Change its value: typed once, never shown. Retire this secret ends every handle on it; it asks you to confirm.', at: 'left' },
      { sel: '[aria-label="Current secret settings"]', title: 'Current settings', text: 'How secrets are stored and sealed on this Lys, as set at install.', at: 'left' },
    ],
    keys: ROWS,
  },
  settings: {
    title: 'Settings',
    topic: 'apps.configuration',
    regions: [
      { sel: 'nav[aria-label="Settings views"]', title: 'Settings', text: 'Your account changes your sign-in email and password. This service shows startup settings; Services and sign-in providers adds providers such as Google. Apps has its own place in the menu.', at: 'below' },
      { sel: '[aria-label="Your account"]', title: 'Your account', text: 'Choose Change email or Change password to open an empty form. Save applies the change and closes the form; Cancel closes it without saving.', at: 'right' },
      { sel: '#screen .card.pane h2:first-of-type', title: 'Layout', text: 'Which side the menu and Help sit on, and whether the menu shows a name beside every icon (Icons or Labels). These are yours, for this browser.', at: 'right' },
      { sel: '.model-windows', title: 'Model context windows', text: 'How many tokens each model’s context holds, which Lys needs to work out an agent’s % of context. Side work marks a model whose calls are not an agent’s own context. Add a model by the name its calls use.', at: 'right' },
      { sel: '.facts', title: 'Settings in effect', text: 'What this Lys is running with: sign-in provider and session length, where permissions are checked, secret storage, machine and settings records, the running build, and how records are stored and signed. Read here; set at install or upgrade.', at: 'left' },
    ],
    keys: [],
  },
  connections: {
    title: 'Services and sign-in providers',
    topic: 'start.google',
    regions: [
      { sel: 'nav[aria-label="Settings views"]', title: 'Settings', text: 'This service, Services and sign-in providers, and Apps.', at: 'below' },
      { sel: '[aria-label="Sign-in providers"]', title: 'Sign-in providers', text: 'How people sign in besides a Lys password: pick a provider (Google), follow the three steps shown (prepare it, give it Lys’s return address with Copy, enter the client id and secret it issued). The secret is sent once and never shown again.', at: 'right' },
      { sel: '#screen .pane', title: 'Services', text: 'The services this Lys is configured to use, each with its purpose and address.', at: 'c' },
    ],
    keys: [],
  },
  me: {
    title: 'You',
    topic: 'start.sign-in',
    regions: [
      { sel: '#lys-account', title: 'Your sign-in', text: 'Your sign-in email. Change in Settings opens Your account in Settings, where you can change your email or password.', at: 'right' },
      { sel: '[aria-label="Your signed-in sessions"]', title: 'Your sessions', text: 'Every browser you are signed in from, this one first, with when each started and expires. Sign out ends this one; End signs another browser out.', at: 'below' },
      { sel: '[aria-label="What you hold"]', title: 'What you hold', text: 'The access you hold, named by what it is on and what it lets you do. What you may pass on to your agents is marked.', at: 'left' },
    ],
    keys: [],
  },
  graph: {
    title: 'Access map',
    topic: 'access.graph',
    regions: [{ sel: '[aria-label="Permission graph"]', title: 'The graph', text: 'The same answers as Access, drawn. Click a person or agent to light everything it reaches; click a resource to light everyone who reaches it; Enter on one opens its page. Zoom with the buttons or the wheel; Fit shows the whole graph.', at: 'c' }],
    keys: [],
  },
  canvas: {
    title: 'Running agents',
    topic: 'people.operations',
    regions: [
      { sel: 'nav[aria-label="Running agents views"], nav[aria-label="Operations views"]', title: 'Views', text: 'Four views of the same running agents: by computer (each computer, with what runs on it), Canvas (every running agent as a window), Sessions (the same as a list) and the proxy (every model call made through Lys).', at: 'below' },
      { sel: '.session-canvas-scroll', title: 'Running agents', text: 'Each running agent is a window showing its terminal. Drag its bar to move it and its corner to size it; drag the background or use the wheel to move the surface. Closing a window leaves the agent running.', at: 'c' },
      { sel: '[aria-label="Widgets to place"]', title: 'Place', text: 'Add a box to group windows, a note, or a line between two things. Boxes and notes can be labelled and coloured; × removes one.', at: 'above' },
      { sel: '[aria-label="Canvas view"]', title: 'View', text: 'Zoom in and out; the percentage returns to actual size. Find an agent jumps to its window.', at: 'above' },
      { sel: '.canvas-layouts', title: 'Layouts', text: 'Save this arrangement under a name and bring it back later. Save over replaces a saved one; Remove asks first.', at: 'left' },
      { sel: '[aria-label="Connections"]', title: 'Connections', text: 'Which agents are messaging which, drawn as lines between their windows.', at: 'left' },
    ],
    keys: [
      ['arrows, on a focused bar', 'move the window'],
      ['Shift + arrows, on a focused bar', 'size its terminal'],
      ['arrows, on the surface', 'move the surface'],
    ],
  },
  import: {
    title: 'Approve estate grants',
    topic: 'access.grant',
    regions: [
      { sel: '#screen .page-header', title: 'What this is', text: 'Lys was installed with a plan of what your agents may use in your other apps. Opening this page changes nothing; approving it as yourself records each grant with you as the person it traces to.', at: 'below' },
      { sel: '#screen table', title: 'The plan', text: 'Each line: what, its kind, what the agents may do with it, which agents, and why it is in the plan. Approve records all of them; nothing is approved one at a time.', at: 'c' },
    ],
    keys: [],
  },
};

/** What a page with no entry of its own shows: the rail and the shell's keys, and the full help's index. */
export const ANY_PAGE: PageHelp = { title: 'This page', topic: '', regions: [], keys: [] };

/** The page a hash path shows, as PAGES keys it; an agent's tab has its own entry where one is written. */
export function pageKeyOf(pathname: string): string {
  const [, view = '', a = '', b = ''] = pathname.split('/');
  if (view === '' || view === 'dashboard') return 'dashboard';
  if (view === 'team') return a ? 'start' : 'dashboard';
  if (view === 'agents' && a === 'new') return 'add-agent';
  if (view === 'people') return a === 'new' ? 'add-person' : 'people';
  if (view === 'file') return b === 'start' ? 'start' : PAGES['file:' + b] ? 'file:' + b : 'file';
  if (view === 'access') return a === 'drafts' ? 'drafts' : a === 'import' ? 'import' : 'access';
  if (view === 'apps' && a === 'connect') return 'apps-connect';
  return view;
}
