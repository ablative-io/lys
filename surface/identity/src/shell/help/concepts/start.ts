import type { Seed } from './types';

/**
 * Getting started, in the order a person sets up a new Lys: the setup page first, then the dashboard's own Getting
 * started list (features/dashboard/GettingStarted.tsx) in its order (where agents run, Google, an app to connect,
 * the first agent), then what follows. These sheets say what each step is for.
 */
export const START: Seed[] = [
  {
    id: 'start.setup',
    title: 'Set up Lys',
    summary: 'The first page of a new Lys makes you its administrator.',
    body: [
      'A new Lys opens on Set up Lys and asks for a setup code first. Run lys identity setup-code on the computer Lys is installed on: it asks you to prove you own that computer with Touch ID (or the computer’s login password) before it makes a code, and it makes none on a computer that cannot ask. The code works once.',
      'Then give your first name, last name and email, and a password twice. The rule for the password is shown above the field before you type, as this Lys’s own configuration sets it; a password outside it is refused with that rule.',
      'Set up and sign in makes you the administrator and signs you in. The same page, opened with a fresh setup code, sets a new administrator password.',
    ],
    steps: [
      { text: 'On the computer Lys runs on, run lys identity setup-code and confirm with Touch ID.' },
      { text: 'Enter the code, then your first name, last name, email and password.' },
      {
        text: 'Press Set up and sign in; the dashboard opens with Getting started.',
        href: '#/dashboard',
        link: 'Open the dashboard'
      },
    ],
    related: ['start.sign-in', 'start.computer', 'people.dashboard'],
  },
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
      { text: 'Change your sign-in email or password in Settings.', href: '#/settings/account', link: 'Your account' },
      { text: 'Check your signed-in sessions on You.', href: '#/me', link: 'Open You' }
    ],
    related: ['start.google', 'trouble.signin', 'people.kind'],
  },
  {
    id: 'start.computer',
    title: 'Add this computer',
    summary: 'Agents run on computers Lys knows about; the first is usually the one Lys is on.',
    body: [
      'On a new Lys, Home asks whether your agents should run on the computer Lys runs on, by its name. Say yes only if that is a computer you want agents to work on; "Not this one" leaves agents to other computers. Lys keeps that answer for you on this browser, so the step counts as answered and is not asked again.',
      'Saying yes records the computer with the name you give it, and lets your own agents start there. Lys starts agents on it through its own runner.',
      'Another computer is added under Computers: give it a name, then get a connection code and run the command shown on that computer. The code works once and is not shown again.',
    ],
    steps: [
      {
        text: 'On Home, answer whether agents run on the computer Lys runs on with its name and "Yes, run agents here".',
        href: '#/dashboard',
        link: 'Open the dashboard'
      },
      {
        text: 'Or add a computer by name on Computers, and connect it with its code.',
        href: '#/network',
        link: 'Open Computers'
      },
    ],
    related: ['computers.network', 'computers.runner', 'computers.may-start'],
  },
  {
    id: 'start.google',
    title: 'Google sign-in',
    summary: 'Let people sign in with their Google account instead of a Lys password.',
    body: [
      'Google is added under Settings, on Services and sign-in providers. It is optional; the dashboard lists it as an optional step, and the step is done once Google is set up and switched on there.',
      'A Google login reaches Lys only for a person already added with that email address; it never creates an account.',
    ],
    steps: [
      {
        text: 'Open Settings → Services and sign-in providers and add Google.',
        href: '#/connections',
        link: 'Open sign-in providers'
      },
      { text: 'Add each person with their Google email address.', href: '#/people/new', link: 'Add a person' },
    ],
    related: ['start.sign-in', 'start.person', 'trouble.signin'],
  },
  {
    id: 'start.app',
    title: 'Connect an app',
    summary: 'Connecting an app, such as your team’s chat, lets you give agents its channels.',
    body: [
      'Apps, under Settings, lists the apps Lys knows how to connect, each with Connect. Connect asks once for the app’s web address; Continue sends your browser there, and the app brings you back to Lys to approve what it asks for.',
      'Nothing an app asks for takes effect until you approve it on Apps. If an app has already asked, the dashboard says so: review what it asks for and approve it.',
      'Until the app is approved, an agent’s Access tab says to connect it on Apps first; its channels appear there once it is approved.',
    ],
    steps: [
      { text: 'Open Apps under Settings and press Connect beside the app.', href: '#/apps', link: 'Open Apps' },
      { text: 'Give the app’s web address.' },
      { text: 'Review the permissions it asks for, then approve it.' }
    ],
    related: ['apps.apps', 'access.app-channels', 'trouble.app-missing'],
  },
  {
    id: 'start.agent',
    title: 'Add an agent',
    summary: 'An agent is an identity that works for a person; Lys starts it and checks everything it does.',
    body: [
      'Add an agent asks four things: its name, the program it uses (Claude Code or Codex), the folder it works in, and the computer it runs on.',
      'Who it answers to, its team and the access it is given in Lys can be chosen too. Unless you choose otherwise it joins no team and is given no access.',
      'On a first run the button is Add and run: it adds the agent and starts it on this computer straight away, with no Start to press. Otherwise the button is Add, and you arrive on the agent’s own page, where Start runs it.',
      'On a first run Lys starts the program in a mode that limits its changes to its own folder with the internet off. If the program has no such mode, a note on the form says it starts in its most careful mode instead, and names that mode. Every choice the form does not ask (the mode, who it answers to, its team and its access) is under More settings.',
    ],
    steps: [
      { text: 'Open People and agents and press Add agent.', href: '#/agents/new', link: 'Add an agent' },
      { text: 'Give it a name, choose the program, the folder and the computer.' },
      { text: 'On a first run press Add and run; it starts at once. Otherwise press Add, then Start on its page.' }
    ],
    related: ['people.agent-file', 'access.give', 'budgets.limit', 'trouble.start'],
  },
  {
    id: 'start.channels',
    title: 'Give an agent an app’s channels',
    summary: 'An agent reaches an app’s channel only through a grant you give it.',
    body: [
      'Once an app is connected, its channels are places in Lys you can give access to.',
      'On the agent’s page, open Access and press Give. You can pass on only access you hold yourself and are allowed to pass to agents.',
      'Access to an app’s channel comes from a grant on that app’s resource that you hold and may pass to agents. The one standing choice, "set up and run agents", records the administrator’s roots first (the agents and apps directories, their own person record and the computer Lys runs on), then gives the five grants, the fifth letting it issue your agents’ certificates. It gives no access to an app’s channels; those need a separate grant you hold and may pass to agents.',
    ],
    steps: [
      { text: 'Connect the app first.', href: '#/apps', link: 'Open Apps' },
      { text: 'First hold a grant on the app’s channel that permits passing its actions to agents.' },
      { text: 'Open the agent’s page, then its Access tab, and press Give.' },
      { text: 'Choose the channel and what the agent may do there.' }
    ],
    related: ['access.give', 'start.app', 'access.restricted'],
  },
  {
    id: 'start.limit',
    title: 'Set a spending limit',
    summary: 'Cap what an agent may spend, and say what happens when it is reached.',
    body: [
      'On the agent’s page, Limits and goals has its spending limit: an amount of dollars a period. It applies as soon as it is saved; what happens when it is reached is chosen beside it. Leave it empty and save to remove it.',
      'Below it, Budget holds further limits, each with a "Limit", a period and what happens "When it’s hit", and can warn you at a percentage of a limit first. Whichever limit is reached first acts first.',
      'If Lys cannot measure spending for that agent yet, it says so and the limit cannot be set.',
    ],
    steps: [
      { text: 'Open the agent’s page, then Limits and goals.' },
      { text: 'Under Spending limit, give the amount and the period, then save.' }
    ],
    related: ['budgets.limit', 'budgets.goals', 'budgets.personal'],
  },
  {
    id: 'start.person',
    title: 'Add a person',
    summary: 'Add someone to the directory; giving them access is optional.',
    body: [
      'Add a person asks for their name and email. Access is optional: they can start with none and be given it later.',
      'If they will sign in with Google, use the email address of their Google account.'
    ],
    steps: [
      { text: 'Open People and agents and press Add person.', href: '#/people/new', link: 'Add a person' },
      { text: 'Give their name and email; choose a first grant only if they need one now.' }
    ],
    related: ['start.google', 'people.kind', 'access.grant'],
  },
  {
    id: 'start.keys',
    title: 'Keyboard shortcuts',
    summary: 'Every screen is a key or two away.',
    body: [
      'g then a letter goes to a screen; ? (outside a field) or ⌘/ (anywhere, a field included) shows what is on this page; ⌘K opens the command palette. The full table is in the page help.',
      'In a list, j and k move the cursor and Enter opens the row. On a person’s or agent’s page, 1 to 7 switch its tabs.'
    ],
    steps: [],
    related: ['people.rail'],
  },
];
