import { actionWords, resourceWords } from '../grants/action-words';
/** Canvas connections are recorded membership and grants, never inferred message delivery or authority. */
import { Refused, request } from '../../api';
import { readGrantWorld } from '../grants/model';
import type { GrantWorld } from '../grants/model';
import type { Team } from '../teams/contract';
import type { RuntimeSession } from './RuntimeSessions';
import type { Unanswered } from './Sessions';
import type { MessageConnection } from './message-connections';

export interface SessionNode {
  id: string; title: string; detail: string; column: 'teams' | 'sessions' | 'resources';
  session?: RuntimeSession;
}
export interface SessionEdge { id: string; from: string; to: string; label: string; kind: 'membership' | 'grant' | 'message' | 'identity'; stands: boolean }
export interface SessionGraph { nodes: SessionNode[]; edges: SessionEdge[]; notices: string[]; unanswered: Unanswered[]; names: Record<string, string> }

export function withMessages(graph: SessionGraph, messages: MessageConnection[]): SessionGraph {
  const nodes = new Map(graph.nodes.map((node) => [node.id, node]));
  const edges = new Map(graph.edges.map((edge) => [edge.id, edge]));
  const identity = (id: string, column: 'teams' | 'resources') => {
    const key = 'identity:' + id;
    if (!nodes.has(key)) nodes.set(key, { id: key, column, title: graph.names[id] ?? id, detail: 'Message identity (not a terminal delivery)' });
    for (const node of graph.nodes) if (node.session?.agent === id) {
      const edgeId = key + ':' + node.id;
      edges.set(edgeId, { id: edgeId, from: key, to: node.id, kind: 'identity', label: 'Session belongs to this identity', stands: true });
    }
    return key;
  };
  for (const message of messages) for (const recipient of message.recipients) {
    const from = identity(message.source, 'teams');
    const to = identity(recipient, 'resources');
    const id = 'message:' + message.message + ':' + recipient;
    edges.set(id, { id, from, to, kind: 'message', stands: true, label: `${message.addressing === 'direct' ? 'Direct message' : 'Explicit mention'} ${message.message} in ${message.stream}` });
  }
  return { ...graph, nodes: [...nodes.values()], edges: [...edges.values()] };
}

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
    if (!world) continue;
    for (const grant of world.list.grants) {
      if (grant.holder !== session.agent) continue;
      const resourceId = 'resource:' + JSON.stringify([grant.resource.kind, grant.resource.id]);
      nodes.set(resourceId, { id: resourceId, column: 'resources', title: grant.resource.id, detail: grant.resource.kind });
      edges.push({ id: grant.id + ':' + id, from: id, to: resourceId,
        label: actionWords(world.model, grant.resource, grant.actions) + ' on ' + resourceWords(grant.resource) + (grant.standing.stands ? '' : ' — ' + grant.standing.refusal + ': ' + grant.standing.reason),
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
  return { ...graph, names: world.status === 'fulfilled' ? Object.fromEntries([...world.value.who].map(([id, person]) => [id, person.name])) : {}, unanswered: live.value.unanswered, notices: [
    ...(teams.status === 'rejected' ? [unavailable('Team connections', teams.reason)] : []),
    ...(world.status === 'rejected' ? [unavailable('Names and grant connections', world.reason)] : []),
  ] };
}
