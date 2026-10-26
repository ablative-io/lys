import { readTogether } from '../../reads';
import { RuntimeCounts } from '../runtime/RuntimeCounts';
import { Teams } from '../teams/Teams';
import { useEffect, useState } from 'react';
import { Navigate, useLocation, useNavigate, useParams } from 'react-router';
import { api, useLoad } from '../../api';
import { organisationActs } from '../grants/organisation-acts';
import type { PeopleView } from '../../generated';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { OrgTeam } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { useShell } from '../../shell/ShellContext';
import type { KindFilter } from '../../shell/ShellContext';
import { DirectoryGate as Gate, problemWords, STATUS } from './Words';
import { entries, needsNewPerson, visibleEntries } from './directory';
import type { Entry } from './directory';
import { readTeams } from '../teams/Teams';
import { Reach, readDirectoryReach } from './reach';
import type { DirectoryReach } from './reach';
import type { Load } from '../../api';
import { readRoles, RoleSummary } from '../roles/AssignedRoles';
import { PageHeader } from '../../shell/PageHeader';
import './people.css';

/** The views of People and agents, each with its own address. */
const VIEWS: [KindFilter, string, string][] = [
  ['all', 'All', '/people'], ['person', 'People', '/people/view/people'], ['agent', 'Agents', '/people/view/agents'],
  ['teams', 'Teams', '/people/view/teams'],
];

const EVERYONE = 'Everyone who works here, human or not. Every agent has a person who answers for it.';

/** The page's head, naming the caller's organisation (DIRECTORY-109 R4). Add person is offered to whoever holds organisation.admit on it, read from their own grants (ACCESS-009 R4). */
function PeopleHead() {
  const shell = useShell();
  const navigate = useNavigate();
  const acts = useLoad(async () => {
    const [me, list] = await Promise.all([api.me(), api.grants()]);
    return organisationActs(me, list.grants);
  }, 'people-head-acts');
  const mayAddPeople = acts.status === 'ok' && acts.data.admit;
  const organisation = acts.status === 'ok' ? acts.data.organisation?.display_name : undefined;
  return (
    <>
      <PageHeader title="People and agents" subtitle={organisation ? `${organisation}. ${EVERYONE}` : EVERYONE} actions={<>
          <a className="btn primary" href="#/agents/new">Add agent</a>
          {mayAddPeople ? <a className="btn" href="#/people/new">Add person</a> : null}
        </>} />
      <nav className="tabs" aria-label="People and agents views">
        {VIEWS.map(([k, l, path]) => (
          <a key={k} data-kind={k} href={'#' + path} className={shell.filterKind === k ? 'on' : ''} aria-current={shell.filterKind === k ? 'page' : undefined}
            onClick={(event) => { event.preventDefault(); shell.setFilterKind(k); shell.setCursor(0); navigate(path); }}>
            {l}
          </a>
        ))}
      </nav>
    </>
  );
}

function Stat({ n, l, warn }: { n: number; l: string; warn?: boolean }) {
  return (
    <div className="stat">
      <div className={warn && n ? 'n warn' : 'n'}>
        {n}
      </div>
      <div className="l">
        {l}
      </div>
    </div>
  );
}

/** A person's row carries their agents folded under it; an agent's row names the person it answers to. */
type Row = Entry & { agents: Entry[] };

/** Retired records are left out until asked for. A retired person whose agents are not retired stays listed: those agents have no one answering. */
function rowsOf(view: PeopleView, kind: KindFilter, retired: boolean): Row[] {
  const all = visibleEntries(view, retired);
  const agentsOf = (id: string) => all.filter((x) => x.kind === 'agent' && x.person?.id === id);
  if (kind === 'agent') return all.filter((x) => x.kind === 'agent').map((x) => ({ ...x, agents: [] }));
  return all.filter((x) => x.kind === 'person').map((x) => ({ ...x, agents: kind === 'person' ? [] : agentsOf(x.id) }));
}

/** A record's state in words a person reads: never the service's own name for it. */
const stateWords = (state: string): string => STATUS[state] ?? 'Unknown state';

const held = (x: Entry) => ({ id: x.id, person: x.person?.id ?? null });

function holds(rows: Row[]): string {
  const people = rows.filter((x) => x.kind === 'person').length;
  const agents = rows.reduce((sum, x) => sum + (x.kind === 'agent' ? 1 : x.agents.length), 0);
  const words = [people ? people.toLocaleString('en-AU') + (people === 1 ? ' person' : ' people') : '', agents ? agents.toLocaleString('en-AU') + (agents === 1 ? ' agent' : ' agents') : ''];
  return words.filter(Boolean).join(', ') || 'no one yet';
}

function List({ view, teams, me, reach }: { view: PeopleView; teams: OrgTeam[]; me: string; reach: Load<DirectoryReach> }) {
  const roles = useLoad(readRoles, 'directory-roles');
  const shell = useShell();
  const navigate = useNavigate();
  const admin = view.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const all = entries(view);
  const names = new Map(all.map((x) => [x.id, x.display_name]));
  const [retired, setRetired] = useState(false);
  const retiredCount = all.filter((x) => x.state === 'retired').length;
  const scoped = rowsOf(view, shell.filterKind, retired).filter((x) => inWhose(whose, teams, me, held(x)));
  const groups = groupByTeam(scoped, held, teams, whose, (id) => names.get(id) ?? 'someone outside your view');
  const [shown, setShown] = useState<string[]>([]);
  useEffect(() => {
    shell.setRows(shown);
    return () => shell.setRows([]);
  }, [shown.join(',')]);
  // An address that names an agent opens the list with that agent chosen, listing agents by themselves when it is folded under its person.
  const wanted = useParams().agent;
  useEffect(() => {
    if (!wanted) return;
    const at = shown.indexOf('#/file/' + wanted);
    if (at >= 0) shell.setCursor(at);
    else if (shown.length && shell.filterKind !== 'agent' && all.some((x) => x.id === wanted && x.kind === 'agent')) shell.setFilterKind('agent');
  }, [wanted, shown.join(',')]);
  const selectedHref = shown[Math.min(shell.cursor, Math.max(0, shown.length - 1))] ?? null;
  const selected = all.find((x) => '#/file/' + x.id === selectedHref) ?? null;
  const active = (kind: string) => all.filter((x) => x.kind === kind && x.state === 'active').length;
  const columns: Column<Row>[] = [
    { head: 'Name', cell: (x) => {
      const person = admin && x.kind === 'person' ? view.people.find((person) => person.id === x.id) : undefined;
      const signIn = person?.sign_in_email;
      const admitted = person?.admitted_email;
      const differs = signIn && admitted && signIn.trim().toLowerCase() !== admitted.trim().toLowerCase();
      return <>{x.display_name}{signIn ? <span className="sec"> {differs ? <>Signs in as {signIn}, added as {admitted} <a href={'#/file/' + x.id + '/credentials'}>Account</a></> : signIn}{person?.email_recorded_at_sign_in ? <> (recorded at sign-in)</> : null}</span> : null}</>;
    } },
    { head: 'Role', cell: (x) => <span className="sec"><RoleSummary load={roles} id={x.id} /></span> },
    { head: 'State', cell: (x) => <span className="state-of" data-state={x.state}><span className={'dot s-' + x.state} />{stateWords(x.state)}</span> },
    // Who the agent reports to: its person, or another agent, with the person who answers for it said beside that agent.
    { head: 'Answers to', cell: (x) => x.person ? <span className="sec">{x.reportsTo ? <>{x.reportsTo.display_name ?? 'An agent outside your view'}<span className="dim"> (agent), for {x.person.display_name}</span></> : x.person.display_name}{needsNewPerson(x) ? <span className="answers-warn"> ({stateWords(x.person.state).toLowerCase()})</span> : null}</span> : null },
    { head: 'Reaches', cell: (x) => <Reach load={reach} id={x.id} compact /> },
  ];
  return (
    <>
      <div className="stat-strip">
        <Stat n={active('person')} l="people active" />
        <Stat n={active('agent')} l="agents active" />
        <Stat n={all.filter(needsNewPerson).length} l="with no one answering" warn />
        <RuntimeCounts />
      </div>
      {admin ? null : <p className="note">You see yourself and the agents that answer to you. The administrator sees everyone.</p>}
      <div className="body one">
        <Listing<Row>
          groups={groups} columns={columns} id={(x) => x.id} href={(x) => '#/file/' + x.id} words={(x) => x.display_name}
          noun={shell.filterKind === 'agent' ? 'agents' : shell.filterKind === 'person' ? 'people' : 'people and agents'}
          holds={holds} under={shell.filterKind === 'all' ? (x) => x.agents.map((agent) => ({ ...agent, agents: [] })) : undefined} underNoun={(n, row) => { const active = row.agents.filter(agent => agent.state === 'active').length; return n + (n === 1 ? ' agent' : ' agents') + (active === n ? '' : ' (' + active + ' active)'); }} underHead={shell.filterKind === 'all' ? 'Agents' : undefined}
          selected={selected?.id ?? null} select={(x) => shell.setCursor(Math.max(0, shown.indexOf('#/file/' + x.id)))} open={(x) => navigate('/file/' + x.id)}
          tools={<><WhoseSelect whose={whose} set={setWhose} teams={teams} admin={admin} />{retiredCount ? <button type="button" className="btn" aria-pressed={retired} onClick={() => setRetired(!retired)}>{retired ? 'Hide retired' : 'Show retired (' + retiredCount + ')'}</button> : null}</>} shown={setShown}
        />
      </div>
    </>
  );
}

export function People() {
  const shell = useShell();
  // The address says which view is shown; the view is kept in the shell so the keys and the list agree with it.
  const { view } = useParams();
  const location = useLocation();
  const addressed = VIEWS.find(([, , path]) => path === '/people/view/' + view)?.[0];
  useEffect(() => { shell.setFilterKind(addressed ?? 'all'); }, [view]);
  // Sessions no agent claims are about what is running, so they are listed with the running agents, not with people.
  if (view === 'found') return <Navigate to="/canvas?view=sessions" replace />;
  // Service accounts are a page of Settings; an old link to them here keeps working and keeps what it asked.
  if (view === 'accounts') return <Navigate to={{ pathname: '/settings/service-accounts', search: location.search }} replace />;
  if (addressed === 'teams') {
    return (
      <div className="page fill">
        <PeopleHead />
        <Teams />
      </div>
    );
  }
  return <DirectoryPage />;
}

function DirectoryPage() {
  const load = useLoad(() => readTogether({ view: api.people(), me: api.me(), teams: readTeams().then((teams) => ({ teams, refused: '' }), (problem: unknown) => ({ teams: [], refused: problemWords(problem) })) }), 'people');
  const reach = useLoad(readDirectoryReach, 'directory-reach');
  return (
    <Gate
      load={load}
      title="Directory"
      ok={(d) => (
        <div className="page fill">
          <PeopleHead />
          {d.teams.refused ? <p className="why-not">Teams cannot be read, so everyone is listed without their team. {d.teams.refused}</p> : null}
          <List view={d.view} teams={d.teams.teams} me={d.me.person.id} reach={reach} />
        </div>
      )}
    />
  );
}
