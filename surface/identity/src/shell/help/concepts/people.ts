import type { Seed } from './types';

/** People, agents and the shell around them. The first entries keep the mock-up's own words (the old dock's). */
export const PEOPLE: Seed[] = [
  {
    id: 'people.kind',
    title: 'Person or agent',
    summary: 'Both are identities. Every agent answers to a person.',
    body: [
      'People sign in. Agents are registered by a person who then answers for them. Anything a person can do here, an agent can do through its tools, if it holds the grant.',
      'People and agents (g p) lists everyone, with tabs for People, Agents, Teams, Service accounts and Found. A person’s row carries their agents folded under it.',
    ],
    steps: [{ text: 'Open People and agents.', href: '#/people', link: 'Open People and agents' }],
    related: ['people.state', 'start.agent', 'start.person'],
  },
  {
    id: 'people.state',
    title: 'State',
    summary: 'Registered, active, suspended or retired. Authority only.',
    body: [
      'State says what an identity is allowed to be doing, not whether anything is running. Sessions are counted separately, as the runtimes report them.',
      'An active identity may be suspended, then reinstated; either may be retired, which is final. Retired records are hidden from lists until you press Show retired.',
    ],
    steps: [],
    related: ['people.kind', 'trouble.start'],
  },
  {
    id: 'people.agent-file',
    title: 'An agent’s page',
    summary: 'Everything about one agent, in tabs: Overview, Settings, Access, Limits and goals, Sessions, Credentials, Record.',
    body: [
      'Overview says who it answers to, its teams and state, and is where Start is. Settings holds its program, model, computer, folder and prompt. Access is what it may reach. Limits and goals holds its spending limit, budget and goals.',
      'Sessions lists its runs and memory; Credentials its credentials and certificates; Record every change to it, with a signed proof of each. Keys 1 to 7 switch tabs, and every tab has its own address.',
      'A person’s page has the same tabs without Settings.',
    ],
    steps: [],
    related: ['people.settings', 'access.give', 'budgets.limit'],
  },
  {
    id: 'people.settings',
    title: 'An agent’s settings',
    summary: 'The program, model, computer and folder an agent starts with.',
    body: [
      'On the agent’s Settings tab: the program it uses and which installed copy, the model, the computer it runs on, its working folder and its system prompt. More settings holds the rest.',
      'If Lys did not find the program itself, type the command you use to start it in Terminal, such as claude, or its full path.',
      'Starts are refused until the settings are saved; the refusal says what is missing.',
    ],
    steps: [
      { text: 'Open the agent’s page and its Settings tab.' },
      { text: 'Choose the program, computer and folder, then save.' },
    ],
    related: ['trouble.start', 'computers.may-start', 'people.agent-file'],
  },
  {
    id: 'people.dashboard',
    title: 'The dashboard',
    summary: 'Your agents, what waits for you and what is running now.',
    body: [
      'Getting started shows what a new Lys still needs, in order, until every step is done: where agents run, Google sign-in (optional), Cambium, and your first agent. A step leaves the list once it is done; "Not this one" counts as an answer.',
      'Agents lists yours by team, with where each runs, its budget and its goals; Start, Watch and Stop sit in each row. A part that could not be read says so in words; press them to see why. Waiting for you counts requests to decide, drafts to approve and reviews due, each linked to its list. Running now lists live sessions; Stop all, beside its heading, stops every agent at once and asks why first.',
      'Choose a running agent to watch it in a small picture over the corner, or open it full size in Operations.',
    ],
    steps: [
      { text: 'Choose where agents run: say yes for this computer, or Not this one.', href: '#/dashboard', link: 'Open the dashboard' },
      { text: 'Optionally, let people sign in with Google.', href: '#/connections', link: 'Open sign-in providers' },
      { text: 'Connect Cambium and approve what it asks for.', href: '#/apps', link: 'Open Apps' },
      { text: 'Add your first agent; on a first run Add and run starts it at once.', href: '#/agents/new', link: 'Add an agent' },
      { text: 'Watch it under Running now, or stop everything with Stop all.' },
    ],
    related: ['start.computer', 'start.google', 'start.cambium', 'start.agent', 'access.drafts', 'people.operations'],
  },
  {
    id: 'people.operations',
    title: 'Operations',
    summary: 'Each running agent is a window you can move.',
    body: [
      'Drag a window by its bar, drag its corner to size it, drag the background or use the wheel to move the surface. With a bar focused, arrows move the window and Shift with arrows sizes its terminal; with the surface focused, arrows move the surface. Closing a terminal view leaves the process running.',
    ],
    steps: [{ text: 'Open Operations (g c).', href: '#/canvas', link: 'Open Operations' }],
    related: ['people.dashboard'],
  },
  {
    id: 'people.rail',
    title: 'The rail',
    summary: 'Every screen, one key away.',
    body: ['g then a letter goes to a screen. [ shows or hides labels. \\ moves the help panel to the other side.'],
    steps: [],
    related: ['start.keys'],
  },
  {
    id: 'people.sections',
    title: 'Sections',
    summary: 'Each section has its own address.',
    body: ['Keys 1 to 7 switch sections of a file. Every section can be linked to.'],
    steps: [],
    related: ['people.agent-file'],
  },
];
