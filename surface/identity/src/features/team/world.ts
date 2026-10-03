/** What the team screen and its dock both read: the tree, and the sessions the runners report. */
import { api, request, useLoad } from '../../api';
import { entries } from '../people/directory';
import type { Team as TeamRecord } from '../teams/contract';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { buildTree } from './tree';
import type { Node } from './tree';

export interface World { tree: Node; sessions: RuntimeSession[] }

export function useWorld(revision: number) {
  return useLoad(async (): Promise<World> => {
    const [teams, people, me, live] = await Promise.all([
      request<{ teams: TeamRecord[] }>('/teams'), api.people(), api.me(), request<{ sessions: RuntimeSession[] }>('/runtime/live'),
    ]);
    const all = entries(people);
    const root = all.find((entry) => entry.id === me.person.id);
    if (!root) throw new Error('The directory did not list you.');
    return { tree: buildTree(root, teams.teams.filter((team) => team.state === 'active'), all), sessions: live.sessions };
  }, 'team-world:' + revision);
}

export const liveOf = (sessions: RuntimeSession[], id: string) => sessions.find((entry) => entry.agent === id && entry.shown !== 'stopped');

/** The agents an address opens: the chosen one first, then those added beside it. */
export function openAgents(pathname: string, search: string): string[] {
  const chosen = decodeURIComponent(pathname.split('/')[2] ?? '');
  const beside = new URLSearchParams(search).get('with')?.split(',').filter(Boolean).map(decodeURIComponent) ?? [];
  return chosen ? [chosen, ...beside.filter((id) => id !== chosen)] : [];
}

export function addressOf(agents: string[]): string {
  const [chosen, ...beside] = agents;
  return '#/team/' + encodeURIComponent(chosen) + (beside.length ? '?with=' + beside.map(encodeURIComponent).join(',') : '');
}
