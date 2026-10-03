/** The organisation as a tree: the signed-in person at the root, each team under its owner, each member under its team, and a lead's own team under the lead. */
import type { Entry } from '../people/directory';
import type { Team } from '../teams/contract';

export interface Branch { team: Team | null; members: Node[] }
export interface Node { entry: Entry; branches: Branch[] }

export function buildTree(root: Entry, teams: Team[], people: Entry[]): Node {
  const byId = new Map(people.map((entry) => [entry.id, entry]));
  const inTeams = new Set(teams.flatMap((team) => team.members));
  const node = (entry: Entry, seen: Set<string>): Node => {
    const path = new Set(seen).add(entry.id);
    const branches: Branch[] = teams.filter((team) => team.owner === entry.id).map((team) => ({
      team,
      members: team.members.flatMap((id) => { const member = byId.get(id); return member && member.kind === 'agent' && !path.has(id) ? [node(member, path)] : []; }),
    }));
    const loose = people.filter((other) => other.kind === 'agent' && other.person?.id === entry.id && !inTeams.has(other.id) && !path.has(other.id));
    if (loose.length) branches.push({ team: null, members: loose.map((other) => node(other, path)) });
    return { entry, branches };
  };
  return node(root, new Set());
}

/** Every agent in the tree, in reading order. */
export function agentsOf(node: Node): Entry[] {
  return node.branches.flatMap((branch) => branch.members.flatMap((member) => [member.entry, ...agentsOf(member)]));
}
