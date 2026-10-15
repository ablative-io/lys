import type { Seed } from './types';

/**
 * Getting started, in the order a person sets up a new Lys. The dashboard's own Getting started list
 * (features/dashboard/GettingStarted.tsx) asks the same things; these sheets say what each is for.
 */
export const START: Seed[] = [
  {
    id: 'start.sign-in',
    title: 'Sign in',
    summary: 'Lys knows who you are before it shows you anything.',
    body: [
      'Open Lys at the address your administrator gave you and sign in with your email and password, or with a provider such as Google if one is set up.',
      'Signing in with Google works only for someone already added as a person here: Lys links the Google login to the person with the same email address. It never makes a new account from a Google login.',
      'Your own page, You (g u), lists the sessions you are signed in with and the access you hold.',
    ],
    steps: [
      { text: 'Sign in with your password or with Google.' },
      { text: 'Check your account and sessions on You.', href: '#/me', link: 'Open You' },
    ],
    related: ['start.google', 'trouble.signin', 'people.kind'],
  },
  {
    id: 'start.computer',
    title: 'Add this computer',
    summary: 'Agents run on computers Lys knows about; the first is usually the one Lys is on.',
    body: [
      'On a new Lys the dashboard asks "Run agents on this computer?" Say yes only if this is a computer you want agents to work on; "Not this one" leaves agents to other computers.',
      'Saying yes records the computer with the name you give it, and lets your own agents start there. Lys starts agents on it through its own runner.',
      'Another computer is added under Network: give it a name, then get a connection code and run the command shown on that computer. The code works once and is not shown again.',
    ],
    steps: [
      { text: 'On the dashboard, answer "Run agents on this computer?" with its name and "Yes, run agents here".', href: '#/dashboard', link: 'Open the dashboard' },
      { text: 'Or add a computer by name on Network, and connect it with its code.', href: '#/network', link: 'Open Network' },
    ],
    related: ['computers.network', 'computers.runner', 'computers.may-start'],
  },
  {
    id: 'start.cambium',
    title: 'Connect Cambium',
    summary: 'Cambium is the team chat; connecting it lets you give agents its channels.',
    body: [
      'Apps, under Configuration, lists the apps Lys knows how to connect, Cambium among them, each with Connect. Connect asks once for Cambium’s web address and reads what Cambium asks for.',
      'Nothing an app asks for takes effect until you approve it on Apps. If Cambium has already asked, the dashboard says so: review what it asks for and approve it.',
      'Until Cambium is approved, an agent’s Access tab says "Connect Cambium on Apps first; its channels appear here once it is approved."',
    ],
    steps: [
      { text: 'Open Apps under Configuration and press Connect beside Cambium.', href: '#/apps', link: 'Open Apps' },
      { text: 'Give Cambium’s web address, such as https://cambium.example.com.' },
      { text: 'Review the permissions it asks for, then approve it.' },
    ],
    related: ['apps.apps', 'access.cambium-channels', 'trouble.cambium-missing'],
  },
  {
    id: 'start.agent',
    title: 'Add an agent',
    summary: 'An agent is an identity that works for a person; Lys starts it and checks everything it does.',
    body: [
      'Add an agent asks four things: its name, the program it uses (Claude Code or Codex), the folder it works in, and the computer it runs on.',
      'Who it answers to, its team and the access it is given in Lys can be chosen too; on a first run they sit under More settings. Unless you choose otherwise it joins no team and is given no access.',
      'When it is added you arrive on its own page, where Start runs it.',
    ],
    steps: [
      { text: 'Open People and agents and press Add agent.', href: '#/agents/new', link: 'Add an agent' },
      { text: 'Give it a name, choose the program, the folder and the computer.' },
      { text: 'Add it, then press Start on its page.' },
    ],
    related: ['people.agent-file', 'access.give', 'budgets.limit', 'trouble.start'],
  },
  {
    id: 'start.channels',
    title: 'Give an agent Cambium channels',
    summary: 'An agent reaches a Cambium channel only through a grant you give it.',
    body: [
      'Once Cambium is connected, its channels are places in Lys you can give access to.',
      'On the agent’s page, open Access and press Give. You can pass on only access you hold yourself and are allowed to pass to agents.',
    ],
    steps: [
      { text: 'Connect Cambium first.', href: '#/apps', link: 'Open Apps' },
      { text: 'Open the agent’s page, then its Access tab, and press Give.' },
      { text: 'Choose the channel and what the agent may do there.' },
    ],
    related: ['access.give', 'start.cambium', 'access.restricted'],
  },
  {
    id: 'start.limit',
    title: 'Set a spending limit',
    summary: 'Cap what an agent may spend, and say what happens when it is reached.',
    body: [
      'On the agent’s page, Limits and goals has its spending limit: an amount of dollars a period. It applies as soon as it is saved; when it is reached, the session ends. Leave it empty and save to remove it.',
      'Below it, Budget holds further limits, each with "Spend at most", a period and what happens "When it’s hit", and can warn you at a percentage of a limit first. Whichever limit is reached first acts first.',
      'If Lys cannot measure spending for that agent yet, it says so and the limit cannot be set.',
    ],
    steps: [
      { text: 'Open the agent’s page, then Limits and goals.' },
      { text: 'Under Spending limit, give the amount and the period, then save.' },
    ],
    related: ['budgets.limit', 'budgets.goals', 'budgets.personal'],
  },
  {
    id: 'start.person',
    title: 'Add a person',
    summary: 'Add someone to the directory; giving them access is optional.',
    body: [
      'Add a person asks for their name and email. Access is optional: they can start with none and be given it later.',
      'If they will sign in with Google, use the email address of their Google account.',
    ],
    steps: [
      { text: 'Open People and agents and press Add person.', href: '#/people/new', link: 'Add a person' },
      { text: 'Give their name and email; choose a first grant only if they need one now.' },
    ],
    related: ['start.google', 'people.kind', 'access.grant'],
  },
  {
    id: 'start.google',
    title: 'Google sign-in',
    summary: 'Let people sign in with their Google account instead of a Lys password.',
    body: [
      'Google is added under Configuration, on Services and sign-in providers. It is optional; the dashboard lists it as an optional step.',
      'A Google login reaches Lys only for a person already added with that email address; it never creates an account.',
    ],
    steps: [
      { text: 'Open Configuration → Services and sign-in providers and add Google.', href: '#/connections', link: 'Open sign-in providers' },
      { text: 'Add each person with their Google email address.', href: '#/people/new', link: 'Add a person' },
    ],
    related: ['start.sign-in', 'start.person', 'trouble.signin'],
  },
  {
    id: 'start.keys',
    title: 'Keyboard shortcuts',
    summary: 'Every screen is a key or two away.',
    body: [
      'g then a letter goes to a screen; ? shows what is on this page; ⌘K opens the command palette. The full table is in the page help (press ?).',
      'In a list, j and k move the cursor and Enter opens the row. On a person’s or agent’s page, 1 to 7 switch its tabs.',
    ],
    steps: [],
    related: ['people.rail'],
  },
];
