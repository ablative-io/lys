/** The organisation as a tree beside the chosen agent, with no header: the stage is the agent's terminal, its settings, or its budgets and goals, and the chosen node in the tree carries the switches and actions. */
import { useState } from 'react';
import { useParams } from 'react-router';
import { Refused, api, request, useLoad } from '../../api';
import { entries } from '../people/directory';
import type { Entry } from '../people/directory';
import type { Team as TeamRecord } from '../teams/contract';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import type { GoalsView } from '../usage/contract';
import { Terminal } from '../runtime/Terminal';
import { Provisioning } from '../provisioning/Provisioning';
import { AgentUsage } from '../usage/Usage';
import { Start } from './Start';
import '../runtime/terminal.css';
import '../usage/usage.css';
import './team.css';

const TABS = [['terminal', 'Terminal'], ['settings', 'Settings'], ['budgets', 'Budgets and goals']] as const;
type Tab = (typeof TABS)[number][0];

/** The first open goal of an agent, read once when its node is drawn. */
function Goal({ agent }: { agent: string }) {
  const load = useLoad(async () => request<GoalsView>('/agents/' + encodeURIComponent(agent) + '/goals'), 'team-goal:' + agent);
  if (load.status !== 'ok') return null;
  const open = load.data.goals.find((item) => item.standing === 'open');
  return open ? <span className="tree-goal">{open.goal.words}</span> : null;
}

function Controls({ agent, tab, session }: { agent: string; tab: Tab; session: RuntimeSession | undefined }) {
  const [said, setSaid] = useState('');
  const compact = async () => {
    if (!session) return;
    try { await request('/runtime/sessions/' + encodeURIComponent(session.session) + '/compact', {}); setSaid('Compaction asked for at the next turn.'); }
    catch (error) { setSaid(error instanceof Refused ? error.refusal.reason : String(error)); }
  };
  const base = '#/team/' + encodeURIComponent(agent) + '/';
  return <div className="tree-controls">
    <div className="seg">{TABS.map(([id, words]) => <a key={id} className={tab === id ? 'on' : undefined} href={base + id}>{words}</a>)}</div>
    <div className="tree-acts">
      <button type="button" className="btn" disabled={!session} onClick={() => void compact()}>Compact</button>
      <button type="button" className="btn" disabled title="Restart is being built tonight">Restart</button>
      <button type="button" className="btn" disabled title="Asking for an MCP server is being built tonight">Ask for MCP</button>
    </div>
    {said ? <span role="status" className="dim">{said}</span> : null}
  </div>;
}

function AgentNode({ agent, session, open, tab }: { agent: Entry; session: RuntimeSession | undefined; open: string | undefined; tab: Tab }) {
  return <li><a className="tree-row" aria-current={open === agent.id ? 'page' : undefined} href={'#/team/' + encodeURIComponent(agent.id) + '/' + tab}>
    <span className={'dot ' + (session ? 's-active' : 's-retired')} aria-label={session ? 'running' : 'not running'} />
    <span className="tree-text"><span className="tree-name">{agent.display_name}</span><Goal agent={agent.id} /></span>
  </a>{open === agent.id ? <Controls agent={agent.id} tab={tab} session={session} /> : null}</li>;
}

export function Team() {
  const { agent: open, tab: asked } = useParams();
  const tab: Tab = TABS.some(([id]) => id === asked) ? asked as Tab : 'terminal';
  const [revision, setRevision] = useState(0);
  const load = useLoad(async () => {
    const [teams, people, me, live] = await Promise.all([
      request<{ teams: TeamRecord[] }>('/teams'), api.people(), api.me(),
      request<{ sessions: RuntimeSession[] }>('/runtime/live'),
    ]);
    return { teams: teams.teams.filter((team) => team.state === 'active'), people: entries(people), me, sessions: live.sessions };
  }, 'team-tree:' + revision);
  if (load.status === 'loading') return <div className="team-screen" />;
  if (load.status === 'refused') {
    const { status, refusal } = load.refused;
    return <p className="why-not team-failed">Could not read your teams and agents: {status === 401 ? 'you are not signed in. Sign in to Lys, then reload this page.' : status ? `the service answered ${status}: ${refusal.reason}` : refusal.reason}</p>;
  }
  const { teams, people, me, sessions } = load.data;
  const agents = people.filter((entry) => entry.kind === 'agent');
  const byId = (id: string) => agents.find((entry) => entry.id === id);
  const live = (id: string) => sessions.find((entry) => entry.agent === id);
  const node = (entry: Entry) => <AgentNode key={entry.id} agent={entry} session={live(entry.id)} open={open} tab={tab} />;
  const inTeams = new Set(teams.flatMap((team) => team.members));
  const persons = people.filter((entry) => entry.kind === 'person' && (entry.id === me.person.id || agents.some((agent) => agent.person?.id === entry.id)));
  const under = (person: string) => <ul className="tree-under">
    {teams.filter((team) => team.owner === person).map((team) => <li key={team.id}><span className="tree-group">{team.name}</span>
      <ul className="tree-under">{team.members.flatMap((id) => { const entry = byId(id); return entry ? [node(entry)] : []; })}</ul></li>)}
    {agents.filter((entry) => !inTeams.has(entry.id) && entry.person?.id === person).map(node)}
  </ul>;
  const chosen = open ? byId(open) : undefined;
  const session = chosen ? live(chosen.id) : undefined;
  return <div className="team-screen">
    <nav className="tree" aria-label="Teams and agents"><ul className="tree-top">
      {persons.map((person) => <li key={person.id}><span className="tree-group tree-person">{person.display_name}</span>{under(person.id)}</li>)}
    </ul></nav>
    <main className="team-stage">
      {!chosen ? <p className="session-empty">Choose an agent in the tree.</p> : null}
      {chosen && tab === 'terminal' ? (session ? <Terminal key={session.session} session={session.session} agent={chosen.id} />
        : <Start key={chosen.id} agent={chosen.id} started={() => setRevision((value) => value + 1)} />) : null}
      {chosen && tab === 'settings' ? <Provisioning id={chosen.id} /> : null}
      {chosen && tab === 'budgets' ? <AgentUsage agent={chosen.id} /> : null}
    </main>
  </div>;
}
