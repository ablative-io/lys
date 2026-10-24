import type { Seed } from './types';

/** Access and permissions: grants, asking, requests, drafts, reviews and secrets. */
export const ACCESS: Seed[] = [
  {
    id: 'access.grant',
    title: 'Grant',
    summary: 'Who holds which relation on what, and the person it traces back to.',
    body: [
      'Every grant is five facts: who holds it, the relation, the object, who it derives from, and whether it may be passed on. Withdrawing a grant withdraws everything derived from it.',
      'Access (g a) lists every grant you may see. Last used is an exercise seen where access is enforced; not seen means none was observed, never that it was never used.',
    ],
    steps: [{ text: 'Open Access.', href: '#/access', link: 'Open Access' }],
    related: ['access.check', 'access.give', 'access.graph'],
  },
  {
    id: 'access.check',
    title: 'Can this be done?',
    summary: 'Ask any way round; every answer shows its path.',
    body: [
      'Ask, under Access, answers what someone can reach, who can reach something, and whether one identity can do one thing.',
      'Yes answers show the chain to a person. No answers say which kind of refusal it is: no grant, identity not active, authority withdrawn upstream, or a type the model does not have.'
    ],
    steps: [{ text: 'Open Access → Ask.', href: '#/access/can', link: 'Open Ask' }],
    related: ['access.grant', 'trouble.permission'],
  },
  {
    id: 'access.give',
    title: 'Give an agent access',
    summary: 'Pass on access you hold to an agent that answers to you.',
    body: [
      'On the agent’s Access tab, Give passes on one of your own grants that may be passed to agents. Only the agent’s responsible person can give it access.',
      'If you hold nothing you can give, the tab says so. The administrator can still give their agent the one standing choice, "set up and run agents": Give records the administrator’s roots first (the agents and apps directories, their own person record and the computer Lys runs on), then gives the five grants, the fifth letting it issue your agents’ certificates. It gives no access to an app’s channels; those need a separate grant you hold and may pass to agents.',
      'Beside the grants, "What it can reach" sums them up, and the check box answers whether it can do one thing.',
    ],
    steps: [{ text: 'Open the agent’s page, then Access.' }, { text: 'Press Give and choose what to pass on.' }],
    related: ['access.grant', 'start.channels', 'trouble.no-access-to-give'],
  },
  {
    id: 'access.app-channels',
    title: 'An app’s channels',
    summary: 'Once an app is approved, its channels are places you can grant.',
    body: [
      'Until the app is connected and approved on Apps, none of its channels can be given, and Access says to connect it first.'
    ],
    steps: [{ text: 'Connect the app on Apps.', href: '#/apps', link: 'Open Apps' }],
    related: ['start.app', 'start.channels'],
  },
  {
    id: 'access.restricted',
    title: 'Restricted places',
    summary: 'Reached only by a grant on the place itself.',
    body: [
      'A restricted place is shown apart: nothing held on the place it sits in reaches it. Give a grant on the place itself.'
    ],
    steps: [],
    related: ['access.grant']
  },
  {
    id: 'access.requests',
    title: 'Requests',
    summary: 'Someone asked for access; you decide.',
    body: [
      'Access → Requests lists what was asked, by whom, on what. The rail’s Access count includes requests waiting for a decision.'
    ],
    steps: [{ text: 'Open Requests.', href: '#/requests', link: 'Open Requests' }],
    related: ['access.drafts', 'access.reviews'],
  },
  {
    id: 'access.drafts',
    title: 'Drafts',
    summary: 'What your agents prepared for you to decide. Nothing is done until you approve it.',
    body: [
      'An agent may prepare a change it is not allowed to make and leave it for you. Each draft shows the request exactly as it would be sent.',
      'Approve does it. Decline asks why you decline it, and the reason is shown with the draft, with who decided and when.'
    ],
    steps: [{ text: 'Open Access → Drafts.', href: '#/access/drafts', link: 'Open Drafts' }],
    related: ['access.requests', 'people.dashboard'],
  },
  {
    id: 'access.reviews',
    title: 'Reviews',
    summary: 'Grants due a look, and agents needing an active owner.',
    body: [
      'Access → Reviews lists grants due a review and agents whose person is no longer active. The rail’s Access count includes them.'
    ],
    steps: [{ text: 'Open Reviews.', href: '#/reviews', link: 'Open Reviews' }],
    related: ['access.requests', 'people.state'],
  },
  {
    id: 'access.graph',
    title: 'Access map',
    summary: 'The same answers as Access, drawn.',
    body: ['Click a person or agent to light what it reaches; click a resource to light who reaches it.'],
    steps: [{ text: 'Open the access map (g h).', href: '#/graph', link: 'Open the access map' }],
    related: ['access.grant']
  },
  {
    id: 'access.sealed',
    title: 'Secrets',
    summary: 'A secret’s value is never shown, to anyone.',
    body: [
      'Secret values go into the store once. Agents use virtual credentials made from them; people use them through the same. Seeing a secret listed does not give permission to use it.',
      'Only its owner can change its value; the value is sent once and never shown again. Retiring a secret ends every handle on it.',
    ],
    steps: [{ text: 'Open Secrets (g v).', href: '#/secrets', link: 'Open Secrets' }],
    related: ['access.scope'],
  },
  {
    id: 'access.scope',
    title: 'Scope',
    summary: 'Organisation, team, or yours alone.',
    body: ['A personal secret is yours. An agent uses it only through a virtual credential you issue, acting for you.'],
    steps: [],
    related: ['access.sealed']
  },
];
