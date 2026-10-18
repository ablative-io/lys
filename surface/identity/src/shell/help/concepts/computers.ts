import type { Seed } from './types';

/** Computers: where agents run, and who may start there. */
export const COMPUTERS: Seed[] = [
  {
    id: 'computers.network',
    title: 'Computers',
    summary: 'The computers agents run on, and who may start there.',
    body: [
      'Computers (g t) lists every computer with its status, what is running now and who may start agents there. Choose one to see its detail.',
      'The strip counts computers up now, those not answering, and agents running.',
    ],
    steps: [{ text: 'Open Computers.', href: '#/network', link: 'Open Computers' }],
    related: ['computers.add', 'computers.runner', 'computers.may-start'],
  },
  {
    id: 'computers.add',
    title: 'Add a computer',
    summary: 'Name it, then connect it from that computer with a one-time code.',
    body: [
      'Add a computer puts the keyboard in the add row: give it a name. To connect it, get a connection code and run the command shown on that computer; the command asks for the code. The code works once and is not shown again, and a new code replaces any given before.',
      'Computers also offers the lys download for that computer.',
    ],
    steps: [
      { text: 'On Computers, press Add a computer and name it.', href: '#/network?add=computer', link: 'Add a computer' },
      { text: 'Get a connection code and run the command on that computer.' },
    ],
    related: ['computers.runner', 'start.computer'],
  },
  {
    id: 'computers.runner',
    title: 'Runner',
    summary: 'The program on a computer through which Lys starts agents.',
    body: [
      'The computer Lys runs on uses Lys’s own runner. On another computer, its runner connects to Lys from there.',
      'With no runner connected, Lys cannot start agents on that computer, and Computers says "Waiting for its runner to connect".',
    ],
    steps: [],
    related: ['trouble.computer', 'computers.network'],
  },
  {
    id: 'computers.may-start',
    title: 'May start here',
    summary: 'Which agents, or holders of which roles, may start on a computer.',
    body: [
      'Each computer names who may start there. "No agent may start here yet" means nobody; "Lys does not start agents here" means the computer has no runner for Lys.',
      'An agent you add on a computer you own is allowed there as you add it.',
    ],
    steps: [],
    related: ['computers.network', 'trouble.start'],
  },
];
