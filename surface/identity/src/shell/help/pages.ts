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
  text: 'Every screen, one key away: g then the letter beside its name. Help (?) opens this layer; the full help is one button below. [ shows or hides the labels.',
  at: 'right',
};

const ROWS: readonly (readonly [string, string])[] = [['j / k', 'move down / up the list'], ['Enter', 'open the row']];

const AGENT_TABS = 'Overview, Settings, Access, Limits and goals, Sessions, Credentials and Record: keys 1 to 7, and each has its own address.';

export const PAGES: Readonly<Record<string, PageHelp>> = {
  dashboard: {
    title: 'Dashboard',
    topic: 'people.dashboard',
    regions: [
      { sel: 'section.getting-started', title: 'Getting started', text: 'What this Lys still needs, in order: where agents run, Google sign-in (optional), Cambium, and your first agent. A step leaves the list once it is done; Not this one counts as an answer.', at: 'below' },
      { sel: 'section.dash-agents', title: 'Agents', text: 'Your agents, grouped by team. Open one for its page, where Start is.', at: 'in-bl' },
      { sel: 'section[aria-label="Waiting for you"]', title: 'Waiting for you', text: 'Requests to decide, drafts to approve and reviews due; each opens its list. A count that could not be read names its refusal instead of a number.', at: 'left' },
      { sel: 'section[aria-label="Running now"]', title: 'Running now', text: 'Every live session. Choose one to watch it in a small picture, or open it full size. Stop all, beside the heading, stops every agent at once and asks why first.', at: 'left' },
      { sel: '.dash-widgets', title: 'Readings', text: 'Usage and the other readings, as the service reports them.', at: 'above' },
    ],
    keys: [],
  },
  people: {
    title: 'People and agents',
    topic: 'people.kind',
    regions: [
      { sel: '.page-header .head-actions', title: 'Add', text: 'Add agent asks its name, program, folder and computer. Add person asks a name and an email; access is optional.', at: 'below' },
      { sel: 'nav[aria-label="People and agents views"]', title: 'Views', text: 'All, People, Agents, Teams, Service accounts and Found; each view has its own address.', at: 'below', dy: 30 },
      { sel: '.stat-strip', title: 'At a glance', text: 'People and agents active, and agents with no one answering for them: those need a new person.', at: 'below', dy: 60 },
      { sel: '.listing', title: 'The list', text: 'A person’s row carries their agents folded under it; an agent’s row names who it answers to. Retired records are hidden until Show retired.', at: 'c' },
    ],
    keys: ROWS,
  },
  'add-agent': {
    title: 'Add an agent',
    topic: 'start.agent',
    regions: [
      { sel: 'form.add-agent', title: 'The agent', text: 'Its name, the program it uses, the folder it works in and the computer it runs on. On a first run, Add and run adds it and starts it at once; otherwise Add, then Start on its page. It is given no access unless you choose some.', at: 'right' },
      { sel: '.first-run-mode', title: 'Careful mode', text: 'This program has no mode that keeps it to its folder with the internet off, so it starts in its most careful mode, named here. Change it under More settings.', at: 'right' },
      { sel: 'details.more-settings', title: 'More settings', text: 'Everything a first run does not ask: the mode it runs in, who it answers to, its team and its access. Open it to change any of them; what you leave keeps its default.', at: 'right' },
      { sel: 'fieldset.agent-grants-scope', title: 'Its access in Lys', text: 'Only access you hold and may pass to agents is offered. Cambium’s channels appear once Cambium is approved on Apps.', at: 'right' },
    ],
    keys: [],
  },
  'add-person': {
    title: 'Add a person',
    topic: 'start.person',
    regions: [
      { sel: 'form[aria-label="Add a person"]', title: 'The person', text: 'Their name and email. If they will sign in with Google, use their Google email. A first grant is optional: they can start with none.', at: 'right' },
    ],
    keys: [],
  },
  file: {
    title: 'A person or agent',
    topic: 'people.agent-file',
    regions: [
      { sel: '.file .head', title: 'Name and state', text: 'The state is authority only: active, suspended or retired says what it may do, not whether anything is running. The menu beside it changes the state.', at: 'below' },
      { sel: 'nav.tabs[aria-label="Agent sections"], nav.tabs[aria-label="Sections"]', title: 'Sections', text: AGENT_TABS + ' A person has no Settings.', at: 'below', dy: 30 },
      { sel: '.agent-overview', title: 'Overview', text: 'Who it answers to, where it runs, and Start.', at: 'in-tr' },
    ],
    keys: [['1 to 7', 'switch sections']],
  },
  'file:access': {
    title: 'Access',
    topic: 'access.give',
    regions: [
      { sel: '.file-section-head', title: 'Grants', text: 'What it holds. Give passes on access you hold and may pass to agents; only its responsible person can give it access.', at: 'below' },
      { sel: '[data-act="agent-roots"]', title: 'Let me give agents access', text: 'A new administrator holds nothing to pass on, so Give does nothing yet. Press this once: Lys records the access you may pass to agents, and Give works from then on.', at: 'right' },
      { sel: '[data-cambium-first]', title: 'Cambium', text: 'Cambium’s channels can be given once Cambium is connected and approved on Apps.', at: 'below' },
      { sel: '.access-halves', title: 'What it can reach, and a check', text: 'Its reach summed up, and a question: can it do one thing? The answer shows the chain to a person, or says why not.', at: 'above' },
    ],
    keys: [['c', 'ask the check']],
  },
  'file:budgets': {
    title: 'Limits and goals',
    topic: 'budgets.limit',
    regions: [
      { sel: 'form[aria-label="Spending limit"]', title: 'Spending limit', text: 'Dollars a period. Applies as soon as it is saved; when it is reached, the session ends. Empty and save to remove it.', at: 'right' },
      { sel: '[aria-label="Schedules"]', title: 'Schedules', text: 'What is sent to the agent, and when.', at: 'right' },
      { sel: '[aria-label="Managed controls"]', title: 'Managed controls', text: 'Requests sent to a running session. A recorded admission confirms the program received it, not that a goal was completed.', at: 'left' },
    ],
    keys: [],
  },
  'file:provisioning': {
    title: 'Settings',
    topic: 'people.settings',
    regions: [
      { sel: '[aria-label="Settings of this agent"]', title: 'How it starts', text: 'The program and installed copy, the model, the computer, its working folder and its prompt. Starts are refused until these are saved.', at: 'right' },
      { sel: '[aria-label="What this agent may do"]', title: 'What it may do', text: 'Its rules: what it may do, what happens when it tries something a rule covers, and who enforces each rule.', at: 'right' },
      { sel: '[aria-label="Connected tools"]', title: 'Connected tools', text: 'The connected tools (MCP servers) it is started with.', at: 'right' },
    ],
    keys: [],
  },
  network: {
    title: 'Network',
    topic: 'computers.network',
    regions: [
      { sel: '.page-header .head-actions', title: 'Add a computer', text: 'Puts the keyboard in the add row: name it, then connect it from that computer with a one-time code.', at: 'below' },
      { sel: '.stat-strip', title: 'At a glance', text: 'Computers up now, those not answering, and agents running.', at: 'below', dy: 30 },
      { sel: '.body.halves .listing', title: 'Computers', text: 'Each computer with its status, what is running and who may start there. Not answering filters to the ones Lys cannot hear.', at: 'in-bl' },
      { sel: '.body.halves > .pane', title: 'The chosen computer', text: 'Its runner, what is running now and who may start here. A connection code is shown once and replaces any before it.', at: 'left' },
    ],
    keys: ROWS,
  },
  apps: {
    title: 'Apps',
    topic: 'apps.apps',
    regions: [
      { sel: 'nav[aria-label="Configuration views"]', title: 'Configuration', text: 'This service, Services and sign-in providers, and Apps.', at: 'below' },
      { sel: '[aria-label="Known apps"]', title: 'Known apps', text: 'Apps Lys knows how to connect, Cambium among them. Connect asks once for the app’s web address and reads what it asks for.', at: 'below', dy: 30 },
      { sel: '.app-card', title: 'The app', text: 'Its state and what it asks for. Nothing it registers takes effect until you approve it here. Decline takes an optional reason, in Why, if you decline.', at: 'left' },
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
      { sel: 'nav[aria-label="Access views"]', title: 'Views', text: 'Grants, Ask, Requests, Drafts, Reviews, Resources and Model; each has its own address.', at: 'below' },
      { sel: '.grant-table', title: 'Grants', text: 'Every grant you may see: who holds what, on what, and from whom. Last used is an exercise seen where access is enforced; not seen never means never used.', at: 'c' },
      { sel: '[aria-label="Question"]', title: 'Ask', text: 'Ask any way round. Every yes traces to a person; every no says why.', at: 'below' },
    ],
    keys: [['c', 'ask the check']],
  },
  drafts: {
    title: 'Drafts',
    topic: 'access.drafts',
    regions: [
      { sel: 'nav[aria-label="Access views"]', title: 'Views', text: 'Drafts sits beside Grants, Requests and Reviews.', at: 'below' },
      { sel: 'table.drafts', title: 'Drafts', text: 'What your agents prepared for you to decide, with the request exactly as it would be sent. Nothing is done until you approve it; Refuse asks why.', at: 'c' },
    ],
    keys: [],
  },
  requests: {
    title: 'Requests',
    topic: 'access.requests',
    regions: [{ sel: 'nav[aria-label="Access views"]', title: 'Requests', text: 'Access someone asked for, waiting for a decision. The rail’s Access count includes them.', at: 'below' }],
    keys: [],
  },
  reviews: {
    title: 'Reviews',
    topic: 'access.reviews',
    regions: [{ sel: 'nav[aria-label="Access views"]', title: 'Reviews', text: 'Grants due a review, and agents whose person is no longer active.', at: 'below' }],
    keys: [],
  },
  secrets: {
    title: 'Secrets',
    topic: 'access.sealed',
    regions: [
      { sel: 'nav[aria-label="Secrets views"]', title: 'Views', text: 'Entries, Access (who may use each) and Activity.', at: 'below' },
      { sel: '#screen .pane', title: 'Secrets', text: 'A secret’s value is never shown, and seeing it here does not give permission to use it. Only its owner changes its value.', at: 'c' },
    ],
    keys: ROWS,
  },
  settings: {
    title: 'Configuration',
    topic: 'apps.configuration',
    regions: [
      { sel: 'nav[aria-label="Configuration views"]', title: 'Configuration', text: 'This service, Services and sign-in providers (Google sign-in is added there), and Apps.', at: 'below' },
      { sel: '#screen .card.pane', title: 'This service', text: 'Which side the menu and Help sit on, whether the menu shows labels, and the settings in effect.', at: 'c' },
    ],
    keys: [],
  },
  connections: {
    title: 'Services and sign-in providers',
    topic: 'start.google',
    regions: [
      { sel: 'nav[aria-label="Configuration views"]', title: 'Configuration', text: 'This service, Services and sign-in providers, and Apps.', at: 'below' },
      { sel: '#screen .pane', title: 'Services and sign-in providers', text: 'The services this Lys is configured to use, and the providers people sign in with, such as Google. A provider’s client secret is sent once and never shown again.', at: 'c' },
    ],
    keys: [],
  },
  me: {
    title: 'You',
    topic: 'start.sign-in',
    regions: [
      { sel: '.you-sessions', title: 'Your sessions', text: 'Where you are signed in; this one is listed first.', at: 'below' },
      { sel: '[aria-label="What you hold"]', title: 'What you hold', text: 'The access you hold, named by what it is on.', at: 'left' },
    ],
    keys: [],
  },
  graph: {
    title: 'Graph',
    topic: 'access.graph',
    regions: [
      { sel: '[aria-label="Permission graph"]', title: 'The graph', text: 'The same answers as Access, drawn. Click a person or agent to light what it reaches; click a resource to light who reaches it.', at: 'c' },
    ],
    keys: [],
  },
  roles: {
    title: 'Roles',
    topic: 'access.grant',
    regions: [
      { sel: '#screen .page-header', title: 'Roles', text: 'Describe a job, assign it to people or agents, and review changes before updating their assigned version.', at: 'below' },
    ],
    keys: [],
  },
  canvas: {
    title: 'Operations',
    topic: 'people.operations',
    regions: [
      { sel: '.session-canvas-scroll', title: 'Running agents', text: 'Each running agent is a window. Drag its bar to move it and its corner to size it; drag the background or use the wheel to move the surface. Closing a terminal view leaves the process running.', at: 'c' },
    ],
    keys: [['arrows, on a focused bar', 'move the window'], ['Shift + arrows, on a focused bar', 'size its terminal'], ['arrows, on the surface', 'move the surface']],
  },
};

/** What a page with no entry of its own shows: the rail and the shell's keys, and the full help's index. */
export const ANY_PAGE: PageHelp = { title: 'This page', topic: '', regions: [], keys: [] };

/** The page a hash path shows, as PAGES keys it; an agent's tab has its own entry where one is written. */
export function pageKeyOf(pathname: string): string {
  const [, view = '', a = '', b = ''] = pathname.split('/');
  if (view === '' || view === 'dashboard' || view === 'team') return 'dashboard';
  if (view === 'agents' && a === 'new') return 'add-agent';
  if (view === 'people') return a === 'new' ? 'add-person' : 'people';
  if (view === 'file') return PAGES['file:' + b] ? 'file:' + b : 'file';
  if (view === 'access') return a === 'drafts' ? 'drafts' : 'access';
  if (view === 'resources' || view === 'model') return 'access';
  if (view === 'apps' && a === 'connect') return 'apps-connect';
  return view;
}
