/** The teams an identity is in, from the teams the service keeps. Being in a team confers nothing. */
import { request, useLoad } from '../../api';
import type { Team } from '../teams/contract';
import { ErrorWords } from '../people/Words';

export function TeamsOf({ id }: { id: string }) {
  const load = useLoad(() => request<{ teams: Team[] }>('/teams'), 'teams-of:' + id);
  if (load.status === 'loading') return <span className="dim">Reading teams…</span>;
  if (load.status === 'refused') return <><span className="why-not">The teams could not be read.</span><ErrorWords problem={load.refused} /></>;
  const held = load.data.teams.filter((team) => team.state === 'active' && team.members.includes(id));
  if (!held.length) return <span className="dim">In no team</span>;
  return <>{held.map((team, index) => <span key={team.id}>{index ? ', ' : ''}<a href="#/people/view/teams">{team.name}</a></span>)} <span className="note">membership gives no access</span></>;
}
