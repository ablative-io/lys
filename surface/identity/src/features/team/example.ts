/** Example data in the shape the tree read (DIRECTORY-071 R2) answers, shown until that read is wired. */

export type Session = 'live' | 'stopped';

export interface Budget {
  holder: 'agent' | 'team';
  measure: 'tokens';
  limit: number;
  spent: number;
}

export interface Profile {
  version: string;
  reviewed_by: string;
  harness: 'Claude Code' | 'Codex';
  program: string;
  model: string;
  mcp_servers: string[];
  writable: string[] | null;
}

export interface TreeAgent {
  id: string;
  name: string;
  role: string;
  session: Session;
  goals: string[];
  budget: Budget;
  profile: Profile;
}

export interface TreeTeam {
  id: string;
  name: string;
  lead: TreeAgent;
  members: TreeAgent[];
  teams: TreeTeam[];
}

export interface Tree {
  root: { id: string; name: string };
  teams: TreeTeam[];
}

const claude = (version: string, reviewer: string, servers: string[]): Profile => ({
  version, reviewed_by: reviewer, harness: 'Claude Code', program: 'claude', model: 'claude-opus-5-5', mcp_servers: servers, writable: null,
});
const codex = (version: string, folder: string, servers: string[]): Profile => ({
  version, reviewed_by: 'Archie', harness: 'Codex', program: 'codex', model: 'set by profile', mcp_servers: servers, writable: [folder],
});
const agent = (id: string, name: string, role: string, session: Session, goals: string[], spent: number, limit: number, profile: Profile): TreeAgent => ({
  id, name, role, session, goals, budget: { holder: 'agent', measure: 'tokens', limit, spent }, profile,
});

const lysSeat = (name: string, goal: string, spent: number, session: Session = 'live') =>
  agent(name.toLowerCase(), name, 'Lys builder', session, [goal], spent, 4_000_000,
    codex('v3', 'stack/lys/worktrees/' + name.toLowerCase(), ['cambium']));

export const example: Tree = {
  root: { id: 'tom', name: 'Tom' },
  teams: [{
    id: 'estate', name: 'Estate',
    lead: agent('waffles', 'Waffles', 'Estate lead', 'live', ['Tonight: the team tree demonstration before bed'], 2_150_000, 8_000_000, claude('v12', 'Tom', ['cambium', 'dot', 'argus'])),
    members: [],
    teams: [
      {
        id: 'lys', name: 'Lys',
        lead: agent('archie', 'Archie', 'Lys lead', 'live', ['Review and land DIRECTORY-071 to 075', 'Shape the tree screen with Tom'], 3_420_000, 6_000_000, claude('v9', 'Waffles', ['cambium', 'dot', 'argus'])),
        members: [
          lysSeat('Brisket', '#121: the reviewed profile reaches the seat, then DIRECTORY-072', 1_870_000),
          lysSeat('Crumpet', 'DIRECTORY-071: nested teams and the tree read', 960_000),
          lysSeat('Pikelet', 'DIRECTORY-073: restart and prompts with no screen open', 2_640_000),
          lysSeat('Scone', 'DIRECTORY-074: confinement per seat', 0, 'stopped'),
        ],
        teams: [],
      },
      {
        id: 'cambium', name: 'Cambium',
        lead: agent('chippy', 'Chippy', 'Cambium lead', 'live', ['Cambium board'], 1_200_000, 6_000_000, claude('v7', 'Waffles', ['cambium', 'dot'])),
        members: [], teams: [],
      },
      {
        id: 'haematite', name: 'Haematite',
        lead: agent('apollo', 'Apollo', 'Haematite lead', 'live', ['Haematite first green'], 1_510_000, 6_000_000, claude('v5', 'Waffles', ['cambium', 'dot'])),
        members: [agent('gypsy', 'Gypsy', 'Haematite builder', 'live', ['Clear the eleven clippy errors'], 3_700_000, 4_000_000, codex('v2', 'stack/haematite', ['cambium']))],
        teams: [],
      },
    ],
  }],
};
