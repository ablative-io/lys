/** The organisation as a narrow tree beside the chosen agent's live terminal, from the service's own teams, people and running sessions. */
import { useParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import { Gate } from '../signin/Gate';
import { entries } from '../people/directory';
import type { Team as TeamRecord } from '../teams/contract';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Terminal } from '../runtime/Terminal';
import { Settings } from './Settings';
import '../runtime/terminal.css';
import './team.css';

interface Known { name: (id: string) => string; live: (id: string) => RuntimeSession | undefined }

function Agent({ id, known, open }: { id: string; known: Known; open: string | undefined }) {
  const session = known.live(id);
  return <a className="tree-row" aria-current={open === id ? 'page' : undefined} href={'#/team/' + encodeURIComponent(id)}>
    <span className={'dot ' + (session ? 's-active' : 's-retired')} aria-label={session ? 'running' : 'not running'} />
    <span className="tree-name">{known.name(id)}</span>
  </a>;
}

export function Team() {
  const { agent: open } = useParams();
  const load = useLoad(async () => {
    const [teams, people, me, live] = await Promise.all([
      request<{ teams: TeamRecord[] }>('/teams'), api.people(), api.me(),
      request<{ sessions: RuntimeSession[] }>('/runtime/live'),
    ]);
    return { teams: teams.teams.filter((team) => team.state === 'active'), people: entries(people), me, sessions: live.sessions };
  }, 'team-tree');
  return <div className="team-screen">
    <Gate load={load} title="Team" ok={({ teams, people, me, sessions }) => {
      const known: Known = {
        name: (id) => people.find((entry) => entry.id === id)?.display_name ?? id,
        live: (id) => sessions.find((entry) => entry.agent === id),
      };
      const inTeams = new Set(teams.flatMap((team) => team.members));
      const loose = people.filter((entry) => entry.kind === 'agent' && !inTeams.has(entry.id) && entry.person?.id === me.person.id);
      const session = open ? known.live(open) : undefined;
      return <>
        <nav className="tree" aria-label="Teams">
          <div className="tree-root"><span className="tree-name">{known.name(me.person.id)}</span></div>
          {teams.map((team) => <section className="tree-team" key={team.id}>
            <div className="eyebrow">{team.name}</div>
            {team.members.map((id) => <Agent key={id} id={id} known={known} open={open} />)}
          </section>)}
          {loose.length ? <section className="tree-team"><div className="eyebrow">Not in a team</div>
            {loose.map((entry) => <Agent key={entry.id} id={entry.id} known={known} open={open} />)}</section> : null}
        </nav>
        <main className="team-stage">
          {open ? <Settings key={open} agent={open} /> : null}
          {!open ? <p className="session-empty">Choose an agent to open its terminal.</p>
            : session ? <Terminal key={session.session} session={session.session} agent={open} />
              : <p className="session-empty">{known.name(open)} is not running. <a href={'#/file/' + encodeURIComponent(open)}>Open its file</a> to start it.</p>}
        </main>
      </>;
    }} />
  </div>;
}
