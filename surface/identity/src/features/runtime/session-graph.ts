/** Canvas connections are recorded membership and grants, never inferred message delivery or authority. */
import { Refused, request } from '../../api';
import { resourceText } from '../../generated/grants';
import { readGrantWorld } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import type { Team } from '../teams/contract';
import type { RuntimeSession } from './RuntimeSessions';
import type { Unanswered } from './Sessions';

export interface SessionNode {
  id: string; title: string; detail: string; column: 'teams' | 'sessions' | 'resources';
  session?: RuntimeSession;
}
export interface SessionEdge { id: string; from: string; to: string; label: string; kind: 'membership' | 'grant'; stands: boolean }
export interface SessionGraph { nodes: SessionNode[]; edges: SessionEdge[]; notices: string[]; unanswered: Unanswered[] }

export function graphFromRecords(sessions: RuntimeSession[], teams: Team[], world: GrantWorld | null): Pick<SessionGraph, 'nodes' | 'edges'> {
  const nodes = new Map<string, SessionNode>();
  const edges: SessionEdge[] = [];
  for (const session of sessions) {
    const id = 'session:' + session.session;
    nodes.set(id, { id, column: 'sessions', session,
      title: session.agent ? world?.who.get(session.agent)?.name ?? session.agent : 'Unattached session',
      detail: session.machine_name ?? session.machine,
    });
    if (!session.agent) continue;
    for (const team of teams) {
      if (!team.members.includes(session.agent)) continue;
      const teamId = 'team:' + team.id;
      nodes.set(teamId, { id: teamId, column: 'teams', title: team.name, detail: team.state === 'active' ? 'Team membership' : 'Retired team' });
      edges.push({ id: teamId + ':' + id, from: teamId, to: id, label: team.state === 'active' ? 'Member; no permission implied' : 'Recorded member of retired team', kind: 'membership', stands: team.state === 'active' });
    }
    for (const grant of world?.list.grants ?? []) {
      if (grant.holder !== session.agent) continue;
      const resourceId = 'resource:' + JSON.stringify([grant.resource.kind, grant.resource.id]);
      nodes.set(resourceId, { id: resourceId, column: 'resources', title: grant.resource.id, detail: grant.resource.kind });
      edges.push({ id: grant.id + ':' + id, from: id, to: resourceId,
        label: grant.actions.join(', ') + ' on ' + resourceText(grant.resource) + (grant.standing.stands ? '' : ' — ' + grant.standing.refusal + ': ' + grant.standing.reason),
        kind: 'grant', stands: grant.standing.stands,
      });
    }
  }
  return { nodes: [...nodes.values()], edges };
}

function unavailable(domain: string, error: unknown): string {
  return domain + ' unavailable: ' + (error instanceof Refused ? error.refusal.refusal + ': ' + error.refusal.reason : String(error));
}

export async function readSessionGraph(): Promise<SessionGraph> {
  const [live, teams, world] = await Promise.allSettled([
    request<{ sessions: RuntimeSession[]; unanswered: Unanswered[] }>('/runtime/live'),
    request<{ teams: Team[] }>('/teams'), readGrantWorld(),
  ]);
  if (live.status === 'rejected') throw live.reason;
  if (!Array.isArray(live.value.sessions) || !Array.isArray(live.value.unanswered)
    || live.value.sessions.some((session) => session.shown === 'stopped')) {
    throw new Error('The service did not return a valid live session list.');
  }
  if (teams.status === 'fulfilled' && !Array.isArray(teams.value.teams)) throw new Error('The service did not return a team list.');
  const graph = graphFromRecords(live.value.sessions, teams.status === 'fulfilled' ? teams.value.teams : [], world.status === 'fulfilled' ? world.value : null);
  return { ...graph, unanswered: live.value.unanswered, notices: [
    ...(teams.status === 'rejected' ? [unavailable('Team connections', teams.reason)] : []),
    ...(world.status === 'rejected' ? [unavailable('Names and grant connections', world.reason)] : []),
    'Message connections are unavailable. No message delivery records were read.',
  ] };
}
