import { ServiceAccountsList } from '../service-accounts/ServiceAccounts';
import { readTogether } from '../../reads';
import { RuntimeCounts } from '../runtime/RuntimeCounts';
import { Teams } from '../teams/Teams';
import { RuntimeSessions } from '../runtime/RuntimeSessions';
import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { api, useLoad } from '../../api';
import type { PeopleView } from '../../generated';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { OrgTeam } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { useShell } from '../../shell/ShellContext';
import type { KindFilter } from '../../shell/ShellContext';
import { DirectoryGate as Gate, problemWords } from './Words';
import { entries, needsNewPerson } from './directory';
import type { Entry } from './directory';
import { readTeams } from '../teams/Teams';
import { Reach, readDirectoryReach } from './reach';
import { readRoles, RoleSummary } from '../roles/AssignedRoles';

/** The views of People and agents, each with its own address. */
const VIEWS: [KindFilter, string, string][] = [
  ['all', 'All', '/people'], ['person', 'People', '/people/view/people'], ['agent', 'Agents', '/people/view/agents'],
  ['teams', 'Teams', '/people/view/teams'], ['accounts', 'Service accounts', '/people/view/accounts'], ['found', 'Found', '/people/view/found'],
];

function PeopleHead() {
  const shell = useShell();
  const navigate = useNavigate();
  return (
    <>
      <div className="head">
        <div>
          <h1>People and agents</h1>
          <p className="sub">Everyone who works here, human or not. Every agent answers to a person.</p>
        </div>
        <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
          <a className="btn primary" href="#/agents/new">Add agent</a>
          <a className="btn" href="#/people/new">Add person</a>
        </div>
      </div>
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
      <div className="n" style={warn && n ? { color: 'var(--warn)' } : undefined}>
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
  const all = entries(view).filter((x) => retired || x.state !== 'retired' || (x.kind === 'person' && entries(view).some((agent) => agent.kind === 'agent' && agent.person?.id === x.id && agent.state !== 'retired')));
  const agentsOf = (id: string) => all.filter((x) => x.kind === 'agent' && x.person?.id === id);
  if (kind === 'agent') return all.filter((x) => x.kind === 'agent').map((x) => ({ ...x, agents: [] }));
  return all.filter((x) => x.kind === 'person').map((x) => ({ ...x, agents: kind === 'person' ? [] : agentsOf(x.id) }));
}

const held = (x: Entry) => ({ id: x.id, person: x.person?.id ?? null });

function holds(rows: Row[]): string {
  const people = rows.filter((x) => x.kind === 'person').length;
  const agents = rows.reduce((sum, x) => sum + (x.kind === 'agent' ? 1 : x.agents.length), 0);
  const words = [people ? people.toLocaleString('en-AU') + (people === 1 ? ' person' : ' people') : '', agents ? agents.toLocaleString('en-AU') + (agents === 1 ? ' agent' : ' agents') : ''];
  return words.filter(Boolean).join(', ') || 'no one yet';
}

function List({ view, teams, me }: { view: PeopleView; teams: OrgTeam[]; me: string }) {
  const reach = useLoad(readDirectoryReach, 'directory-reach');
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
    { head: 'Name', cell: (x) => x.display_name },
    { head: 'Role', cell: (x) => <span className="sec"><RoleSummary load={roles} id={x.id} /></span> },
    { head: 'State', cell: (x) => <><span className={'dot s-' + x.state} />{x.state}</> },
    { head: 'Answers to', cell: (x) => x.person ? <span className="sec">{x.person.display_name}{needsNewPerson(x) ? <span style={{ color: 'var(--warn)' }}> ({x.person.state})</span> : null}</span> : null },
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
      {admin ? null : <p className="note">Your own records: you and the agents that answer to you. A directory administrator sees everyone through the directory&apos;s own routes.</p>}
      <div className="body one">
        <Listing<Row>
          groups={groups} columns={columns} id={(x) => x.id} href={(x) => '#/file/' + x.id} words={(x) => x.display_name}
          noun={shell.filterKind === 'agent' ? 'agents' : shell.filterKind === 'person' ? 'people' : 'people and agents'}
          holds={holds} under={shell.filterKind === 'all' ? (x) => x.agents.map((agent) => ({ ...agent, agents: [] })) : undefined} underNoun={(n) => n + (n === 1 ? ' agent' : ' agents')} underHead={shell.filterKind === 'all' ? 'Agents' : undefined}
          selected={selected?.id ?? null} select={(x) => shell.setCursor(Math.max(0, shown.indexOf('#/file/' + x.id)))} open={(x) => navigate('/file/' + x.id)}
          tools={<><WhoseSelect whose={whose} set={setWhose} teams={teams} admin={admin} />{retiredCount ? <button type="button" className="btn" aria-pressed={retired} onClick={() => setRetired(!retired)}>{retired ? 'Hide retired' : 'Show retired (' + retiredCount + ')'}</button> : null}</>} shown={setShown}
        />
      </div>
    </>
  );
}

export function People() {
  const shell = useShell();
  const load = useLoad(() => readTogether({ view: api.people(), me: api.me(), teams: readTeams().then((teams) => ({ teams, refused: '' }), (problem: unknown) => ({ teams: [], refused: problemWords(problem) })) }), 'people');
  // The address says which view is shown; the view is kept in the shell so the keys and the list agree with it.
  const { view } = useParams();
  const addressed = VIEWS.find(([, , path]) => path === '/people/view/' + view)?.[0];
  useEffect(() => { shell.setFilterKind(addressed ?? 'all'); }, [view]);
  if (shell.filterKind === 'teams' || shell.filterKind === 'found' || shell.filterKind === 'accounts') {
    return (
      <div className="page fill">
        <PeopleHead />
        {shell.filterKind === 'found' ? <RuntimeSessions found /> : shell.filterKind === 'accounts' ? <ServiceAccountsList /> : <Teams />}
      </div>
    );
  }
  return (
    <Gate
      load={load}
      title="Directory"
      ok={(d) => (
        <div className="page fill">
          <PeopleHead />
          {d.teams.refused ? <p className="why-not">Teams cannot be read, so everyone is listed without their team. {d.teams.refused}</p> : null}
          <List view={d.view} teams={d.teams.teams} me={d.me.person.id} />
        </div>
      )}
    />
  );
}
