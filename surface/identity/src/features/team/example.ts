/** The tree read's response contract and illustrative data for the screen sketch. */
export interface TreeBudget {
  holder: { kind: 'agent' | 'team'; id: string };
  measure: 'tokens' | 'running_ms';
  version: number;
  limit: number;
  spent: number;
  remaining: number;
}

export interface TreeGoal {
  id: string;
  words: string;
  standing: 'open' | 'met' | 'missed' | 'withdrawn';
  deadline: number;
}

export interface TreeProfile {
  version: number;
  operation: string;
  program: string | null;
  model: string | null;
  mcp_servers: string[];
  writable_folders: string[];
}

export interface TreeSession {
  id: string;
  machine: string;
  state: 'live' | 'stopped' | 'unconfirmed';
}

export interface TreeAgent {
  id: string;
  name: string;
  state: 'live' | 'stopped' | 'unconfirmed';
  sessions: TreeSession[];
  goals: TreeGoal[];
  budgets: TreeBudget[];
  profile: TreeProfile | null;
}

export interface TreeTeam {
  id: string;
  name: string;
  owner: string;
  parent: string | null;
  lead: TreeAgent | null;
  agents: TreeAgent[];
  children: TreeTeam[];
}

export interface TreeResponse {
  caller: { kind: 'person' | 'agent'; id: string; name: string };
  teams: TreeTeam[];
}

const owner = 'person-example';
const deliveryTeam = 'op-00000000000000000000000000000002';
const rootTeam = 'op-00000000000000000000000000000001';

function agent(
  name: string,
  index: number,
  state: 'live' | 'stopped',
  goal: string,
  spent: number,
): TreeAgent {
  const id = `agent-example-${index}`;
  return {
    id,
    name,
    state,
    sessions: [{ id: `session-example-${index}`, machine: 'machine-example', state }],
    goals: [{ id: `goal-example-${index}`, words: goal, standing: 'open', deadline: 0 }],
    budgets: [{
      holder: { kind: 'agent', id },
      measure: 'tokens',
      version: 1,
      limit: 100_000,
      spent,
      remaining: 100_000 - spent,
    }],
    profile: {
      version: 1,
      operation: `profile-example-${index}`,
      program: '/example/bin/seat',
      model: 'example-model',
      mcp_servers: ['identity', 'messages'],
      writable_folders: [`/example/workspaces/seat-${index}`],
    },
  };
}

/** Illustrative values only; this object is never evidence of a live session or a grant. */
export const example: TreeResponse = {
  caller: { kind: 'person', id: owner, name: 'Tom' },
  teams: [{
    id: rootTeam,
    name: 'Platform',
    owner,
    parent: null,
    lead: agent('Waffles', 1, 'live', 'Coordinate the work and the demonstration.', 24_000),
    agents: [],
    children: [{
      id: deliveryTeam,
      name: 'Lys',
      owner,
      parent: rootTeam,
      lead: agent('Archie', 2, 'live', 'Brief, review and direct identity delivery.', 18_000),
      agents: [
        agent('Brisket', 3, 'live', 'Carry the reviewed profile into the agent start.', 31_000),
        agent('Crumpet', 4, 'live', 'Record nested teams and answer the visible tree.', 12_000),
        agent('Pikelet', 5, 'stopped', 'Prepare the next reviewed delivery.', 6_000),
      ],
      children: [],
    }],
  }],
};
