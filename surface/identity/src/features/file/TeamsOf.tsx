/** The teams an identity is in, from the teams the service keeps. Being in a team confers nothing. */
import { request, useLoad } from '../../api';
import type { TeamView } from '../teams/Teams';

export function TeamsOf({ id }: { id: string }) {
  const load = useLoad(() => request<{ teams: TeamView[] }>('/teams'), 'teams-of:' + id);
  if (load.status === 'loading') return <span className="dim">…</span>;
  if (load.status === 'refused') return <span className="why-not">{load.refused.refusal.refusal}: {load.refused.message}</span>;
  const held = load.data.teams.filter((team) => team.state === 'active' && team.members.includes(id));
  if (!held.length) return <span className="dim">In no team</span>;
  return <>{held.map((team, index) => <span key={team.id}>{index ? ', ' : ''}<a href="#/teams">{team.name}</a></span>)} <span className="note">membership gives no access</span></>;
}
