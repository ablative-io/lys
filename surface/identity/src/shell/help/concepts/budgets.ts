import type { Seed } from './types';

/** Budgets: what an agent may spend, and the goals it works to. */
export const BUDGETS: Seed[] = [
  {
    id: 'budgets.limit',
    title: 'Spending limit',
    summary: 'The most an agent may spend a period; the session ends when it is reached.',
    body: [
      'On an agent’s Limits and goals tab. It applies as soon as it is saved, and when it is reached the session ends. Leave the amount empty and save to remove it.',
      'If Lys cannot measure spending for that agent yet, the tab says so and the limit cannot be set.',
    ],
    steps: [
      { text: 'Open the agent’s page, then Limits and goals.' },
      { text: 'Give the amount and the period under Spending limit, and save.' },
    ],
    related: ['budgets.budget', 'start.limit'],
  },
  {
    id: 'budgets.budget',
    title: 'Budget',
    summary: 'Further limits, each acting on its own, with an optional warning.',
    body: [
      'Each limit is "Spend at most" an amount a period, and what happens "When it’s hit". Each limit acts on its own: whichever is reached first acts first.',
      'You can be warned at a percentage of a limit; leave it blank for no warning.',
    ],
    steps: [],
    related: ['budgets.limit', 'budgets.personal'],
  },
  {
    id: 'budgets.goals',
    title: 'Goals',
    summary: 'What an agent is working towards, with a deadline and where it stands.',
    body: ['Goals sit on the agent’s Limits and goals tab. Type a goal in the table’s last row and press Enter or Set; a deadline is optional. Only open goals can be changed: reword one where it stands, or switch it off.'],
    steps: [],
    related: ['budgets.limit', 'budgets.controls'],
  },
  {
    id: 'budgets.controls',
    title: 'Schedules, variables and controls',
    summary: 'What is sent to an agent on a schedule, the values it is given, and requests sent to its session.',
    body: [
      'Messages it is sent lists what Lys sends the agent at each moment, such as when its context fills; choose Its own words to change one for this agent, and Preview to see it without sending.',
      'Schedules lists what is sent to the agent and when; Variables the values kept for it, each until removed or until a time you choose. Managed controls send a request to a running session; a recorded admission confirms the program received it, not that a goal was completed.',
    ],
    steps: [],
    related: ['budgets.goals'],
  },
  {
    id: 'budgets.personal',
    title: 'Limits across a person’s agents',
    summary: 'A person’s page sets limits over all their agents together.',
    body: ['On a person’s Limits and goals tab. A requested change does not replace the enforced budget until an administrator applies it.'],
    steps: [],
    related: ['budgets.budget'],
  },
];
