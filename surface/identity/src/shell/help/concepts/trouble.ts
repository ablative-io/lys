import type { Seed } from './types';

/**
 * Something isn't working: the refusals people meet most, in the words the screens use
 * (features/people/Words.tsx, features/sign-in/SignIn.tsx), and what to do about each.
 */
export const TROUBLE: Seed[] = [
  {
    id: 'trouble.read',
    title: 'Reading a refusal',
    summary: 'Every refusal says what happened and what to do; the service’s own code sits under Error details.',
    body: [
      'Lys never shows a bare error. The sentence says what was refused and the next step; Error details, folded beneath it, holds the service’s name for the refusal, which is what to quote to an administrator.',
      '"Lys refused without explaining why" means the service gave no reason: quote the name under Error details.',
    ],
    steps: [],
    related: ['trouble.permission', 'trouble.unknown'],
  },
  {
    id: 'trouble.signin',
    title: 'I can’t sign in',
    summary: 'What each sign-in refusal means.',
    body: [
      '"That email or password is not right": check both; the email is the one you were added with.',
      '"That sign-in expired or was already used. Try again.": start the sign-in again from the sign-in page.',
      '"Too many sign-ins failed from here": wait a little, then try again.',
      'Signing in with Google reaches Lys only if an administrator has added you as a person with your Google email address. Lys never makes an account from a Google login.',
      '"Your sign-in account is not connected to a person in the directory": ask the administrator to connect it.',
    ],
    steps: [],
    related: ['start.sign-in', 'start.google'],
  },
  {
    id: 'trouble.permission',
    title: '"You do not have permission to do this"',
    summary: 'Your grants do not cover this change.',
    body: [
      'Ask the directory administrator, or the agent’s responsible person, to make the change or to give you the access.',
      'To see why, ask under Access → Ask: a no answer says which kind of refusal it is (no grant, identity not active, authority withdrawn upstream).',
      '"This agent is not among the records you can see" means it answers to someone else: ask its responsible person.',
    ],
    steps: [{ text: 'Ask whether you can.', href: '#/access/can', link: 'Open Ask' }],
    related: ['access.check', 'access.requests'],
  },
  {
    id: 'trouble.start',
    title: 'An agent won’t start',
    summary: 'Start is refused until the agent has what it needs.',
    body: [
      '"Choose the folder this agent works in before it can start", "Save this agent’s settings before starting it" and "Choose the installed program this agent uses": open its Settings tab, choose what is named, and save.',
      '"This agent is not ready to start": check its state on Overview; its access may be not turned on yet, paused or retired.',
      '"No runner is connected, so Lys cannot start agents here", or the computer is not answering: see Computers.',
      'If the computer does not let the agent start there, its May start here line on Computers does not name it.',
    ],
    steps: [{ text: 'Check that computer under Computers.', href: '#/network', link: 'Open Computers' }],
    related: ['people.settings', 'computers.may-start', 'trouble.computer'],
  },
  {
    id: 'trouble.computer',
    title: 'A computer is not answering',
    summary: 'Lys cannot hear its runner.',
    body: [
      'The Computers page counts the computers not answering; filter by Not answering to see them. "Waiting for its runner to connect" means the runner on that computer has not reached Lys.',
      'If it was never connected, or has to connect again, get a connection code on Computers and run the command shown on that computer. A new code replaces any code given before.',
    ],
    steps: [{ text: 'Open Computers.', href: '#/network', link: 'Open Computers' }],
    related: ['computers.runner', 'computers.add'],
  },
  {
    id: 'trouble.app-missing',
    title: 'An app’s channels are missing',
    summary: 'The app is not connected or not approved yet.',
    body: ['Channels appear as places you can give only once their app is connected and approved on Apps. If the app shows waiting, review and approve it.', 'Once approved, a channel you can see but cannot give is one you hold no passable access to.'],
    steps: [{ text: 'Open Apps.', href: '#/apps', link: 'Open Apps' }],
    related: ['start.app', 'trouble.no-access-to-give'],
  },
  {
    id: 'trouble.no-access-to-give',
    title: '"You hold no access you can give an agent"',
    summary: 'You can pass on only access you hold and may pass to agents.',
    body: [
      'The administrator can give their agent the one standing choice, "set up and run agents", on its Access tab: Give records the administrator’s roots first (the agents and apps directories, their own person record and the computer Lys runs on), then gives the five grants, the fifth letting it issue your agents’ certificates. It gives no access to an app’s channels; those need a separate grant you hold and may pass to agents.',
      'Anyone else asks the administrator for a grant that may be passed to agents.',
    ],
    steps: [],
    related: ['access.give', 'access.grant'],
  },
  {
    id: 'trouble.unknown',
    title: '"The outcome is unknown"',
    summary: 'Lys may have saved a change but did not confirm it.',
    body: ['When an answer did not confirm a change, the screen keeps what you entered and says so. Use Check, or reload the page, to see whether Lys saved it before sending it again.', '"Someone changed these rules first" means another person saved first: open the tab again to see what was saved, then edit.'],
    steps: [],
    related: ['trouble.read', 'trouble.unreachable'],
  },
  {
    id: 'trouble.unreachable',
    title: '"Lys could not be reached"',
    summary: 'The page cannot reach the Lys service.',
    body: ['Ask the administrator to check the service. Keep any change whose outcome is unknown, and check it once Lys answers again.', 'On the dashboard, Reconnect appears when its live feed stopped.'],
    steps: [],
    related: ['trouble.unknown'],
  },
];
