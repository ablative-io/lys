/** The computers agents run on, grouped by the teams whose agents start there: whether each is up, what runs on it now, and who may start there. */
import { useState } from 'react';
import { useSearchParams } from 'react-router';
import { api, request, useLoad } from '../../api';
import type { PeopleView } from '../../generated';
import { Listing } from '../../shell/Listing';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { Held, OrgTeam, Whose } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { problemWords } from '../people/Words';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Gate } from '../signin/Gate';
import { readTeams } from '../teams/Teams';
import { AddMachine } from './AddMachine';
import type { Machine, NetworkView } from './contract';
import { MachineDetail, status } from './MachineDetail';
import type { RunnerRecord } from './MachineDetail';

/** One computer with what Lys knows of it now. */
export interface Computer { machine: Machine; runner: RunnerRecord | null; running: RuntimeSession[] | null; reports: boolean }

type Show = 'all' | 'attention';

async function readComputers(): Promise<Computer[]> {
  const view = await request<NetworkView>('/network');
  const runners = await Promise.all(view.machines.map((machine) => request<{ runner: RunnerRecord | null }>('/network/machines/' + encodeURIComponent(machine.id) + '/runner').then((answer) => answer.runner)));
  const live = await request<{ sessions: RuntimeSession[] }>('/runtime/live').then((answer) => answer.sessions, () => null);
  return view.machines.map((machine, index) => ({ machine, runner: runners[index], reports: view.reports_served, running: live?.filter((session) => session.machine === machine.id && session.shown !== 'stopped') ?? null }));
}

/** A computer belongs where the agents that may start on it belong. */
const held = (computer: Computer): Held => ({ id: computer.machine.id, person: null, also: computer.machine.may_run.map((agent) => agent.id) });

function inScope(whose: Whose, teams: OrgTeam[], me: string, people: PeopleView, computer: Computer): boolean {
  if (whose.kind !== 'mine') return inWhose(whose, teams, me, held(computer));
  const mine = new Set(people.people.filter((person) => person.id === me).flatMap((person) => person.agents.map((agent) => agent.id)));
  return computer.machine.named_by === me || computer.machine.may_run.some((agent) => mine.has(agent.id));
}

export function Network() {
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const load = useLoad(async () => ({
    computers: await readComputers(), people: await api.people(), me: await api.me(),
    teams: await readTeams().then((teams) => ({ teams, refused: '' }), (problem: unknown) => ({ teams: [], refused: problemWords(problem) })),
  }), 'network:' + revision);
  return <div className="page fill">
    <Gate load={load} title="Computers" ok={(data) => <Computers {...data} teams={data.teams.teams} teamsRefused={data.teams.refused} notice={notice} refresh={(message) => { setNotice(message); setRevision((value) => value + 1); }} />} />
  </div>;
}

function Computers({ computers, people, me, teams, teamsRefused, notice, refresh }: { computers: Computer[]; people: PeopleView; me: { person: { id: string } }; teams: OrgTeam[]; teamsRefused: string; notice: string; refresh: (message: string) => void }) {
  const admin = people.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const [show, setShow] = useState<Show>('all');
  const [search, setSearch] = useSearchParams();
  const adding = search.get('add') === 'computer';
  const setAdding = (value: boolean) => {
    const next = new URLSearchParams(search);
    if (value) next.set('add', 'computer'); else next.delete('add');
    setSearch(next, { replace: !value });
  };
  const [picked, setPicked] = useState<string | null>(null);
  const names = new Map(people.people.flatMap((person) => [[person.id, person.display_name] as const, ...person.agents.map((agent) => [agent.id, agent.display_name] as const)]));
  const attention = (computer: Computer) => status(computer).state === 'down';
  const scoped = computers.filter((computer) => inScope(whose, teams, me.person.id, people, computer)).filter((computer) => show === 'all' || attention(computer));
  const groups = groupByTeam(scoped, held, teams, whose, (id) => names.get(id) ?? 'someone outside your view');
  const selected = computers.find((computer) => computer.machine.id === picked) ?? scoped[0] ?? null;
  const count = (n: number) => n.toLocaleString('en-AU');
  const up = computers.filter((computer) => status(computer).state === 'up').length;
  const running = computers.reduce((sum, computer) => sum + (computer.running?.length ?? 0), 0);
  const columns: Column<Computer>[] = [
    { head: 'Computer', cell: (computer) => computer.machine.name },
    { head: 'Status', cell: (computer) => { const now = status(computer); return <><span className={'dot ' + (now.state === 'up' ? 's-active' : now.state === 'down' ? 's-suspended' : 's-retired')} />{now.words}</>; } },
    { head: 'Running now', cell: (computer) => computer.running === null ? <span className="dim">cannot tell</span> : computer.running.length ? count(computer.running.length) + (computer.running.length === 1 ? ' agent' : ' agents') : <span className="dim">nothing</span> },
    { head: 'May start here', cell: (computer) => <span className="sec">{mayStart(computer.machine)}</span> },
  ];
  const changed = (message: string) => { setAdding(false); refresh(message); };
  return <>
    <div className="head">
      <div><div className="eyebrow">Where agents run</div><h1>Computers</h1><p className="sub">Every computer Lys may start agents on, and what is running there now.</p></div>
      {admin && !adding ? <button className="btn primary" onClick={() => setAdding(true)}>+ Add a computer</button> : null}
    </div>
    <div className="stat-strip">
      <div className="stat"><div className="n">{count(computers.filter((computer) => computer.machine.state !== 'retired').length)}</div><div className="l">computers</div></div>
      <div className="stat"><div className="n">{count(up)}</div><div className="l">up now</div></div>
      <div className="stat"><div className="n" style={computers.some(attention) ? { color: 'var(--warn)' } : undefined}>{count(computers.filter(attention).length)}</div><div className="l">not heard from</div></div>
      <div className="stat"><div className="n">{count(running)}</div><div className="l">agents running</div></div>
    </div>
    {notice ? <p role="status">{notice}</p> : null}
    {teamsRefused ? <p className="why-not">Teams cannot be read, so computers are listed without their team. {teamsRefused}</p> : null}
    <div className="body">
      <Listing<Computer> groups={groups} columns={columns} id={(computer) => computer.machine.id} href={(computer) => '#/network?computer=' + computer.machine.id}
        words={(computer) => computer.machine.name + ' ' + computer.machine.may_run.map((agent) => agent.display_name).join(' ')} noun="computers"
        holds={(items) => count(items.length) + (items.length === 1 ? ' computer' : ' computers')}
        selected={selected?.machine.id ?? null} select={(computer) => { setPicked(computer.machine.id); setAdding(false); }} open={(computer) => { setPicked(computer.machine.id); setAdding(false); }}
        tools={<>
          <WhoseSelect whose={whose} set={setWhose} teams={teams} admin={admin} />
          <div className="seg">{([['all', 'All'], ['attention', 'Not heard from']] as [Show, string][]).map(([key, label]) => <button key={key} className={show === key ? 'on' : ''} onClick={() => setShow(key)}>{label}</button>)}</div>
        </>} />
      <div className="detail">
        {adding && admin ? <AddMachine person={me.person.id} agents={people.people.flatMap((person) => person.agents)} changed={changed} cancel={() => setAdding(false)} />
          : selected ? <MachineDetail computer={selected} admin={admin} names={names} changed={changed} /> : <p className="dim">{computers.length ? 'Choose a computer.' : 'No computers yet. Add the one Lys runs on to start agents here.'}</p>}
      </div>
    </div>
  </>;
}

/** Who may start agents on a computer, in a line that stays short however many there are. */
export function mayStart(machine: Machine): string {
  if (machine.runtime === null) return 'Lys does not start agents here.';
  const roles = (machine.may_run_roles ?? []).map((role) => 'anyone holding ' + role);
  const agents = machine.may_run.map((agent) => agent.display_name);
  const listed = agents.length > 3 ? [...agents.slice(0, 2), agents.length - 2 + ' more agents'] : agents;
  const all = [...listed, ...roles];
  return all.length ? all.join(', ') : 'No agent may start here yet.';
}
