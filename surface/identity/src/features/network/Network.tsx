import { readTogether } from '../../reads';
/** The Network: the computers agents run on, grouped by the teams whose agents start there: whether each is up, what runs on it now, and who may start there. */
import { useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router';
import { Refused, api, request, useLoad } from '../../api';
import type { PeopleView } from '../../generated';
import { Listing } from '../../shell/Listing';
import { counted } from '../../shell/count';
import type { Column } from '../../shell/Listing';
import { groupByTeam, inWhose } from '../../shell/org';
import type { Held, OrgTeam, Whose } from '../../shell/org';
import { useWhose, WhoseSelect } from '../../shell/Whose';
import { problemWords } from '../people/Words';
import type { RuntimeSession } from '../runtime/RuntimeSessions';
import { Gate } from '../signin/Gate';
import { readTeams } from '../teams/Teams';
import { AddComputerRow } from './AddMachine';
import type { Machine, NetworkView } from './contract';
import { MachineDetail, status } from './MachineDetail';
import type { RunnerRecord } from './MachineDetail';

/**
 * What Lys knows of a computer's runner: still being asked; answered, with its record and whether the runner answered
 * just now (null when it has none) and why not; or not read at all, with why in `reason`.
 */
export interface Asked { asked: 'asking' | 'answered' | 'unread'; runner: RunnerRecord | null; answers: boolean | null; refusal: string | null; reason: string | null }

/** One computer with what Lys knows of it now. */
export interface Computer extends Asked { machine: Machine; running: RuntimeSession[] | null }

const ASKING: Asked = { asked: 'asking', runner: null, answers: null, refusal: null, reason: null };

/** A machine's runner as the service answers it, asked just now. */
type RunnerAnswer = { runner: RunnerRecord | null; answers: boolean | null; refusal?: string; reason?: string };

function readRunner(answer: RunnerAnswer): Asked {
  if (answer.answers !== true && answer.answers !== false && answer.answers !== null) throw new Error('The service did not say whether the computer\'s runner answers.');
  return { asked: 'answered', runner: answer.runner, answers: answer.answers, refusal: answer.refusal ?? null, reason: answer.reason ?? null };
}

/**
 * Each computer's runner, asked on its own: each answer fills its own row when it arrives, so a runner that never
 * answers holds only its own row, and the page is drawn without waiting on any of them.
 */
function useRunners(machines: Machine[]): Map<string, Asked> {
  const [known, setKnown] = useState(() => new Map<string, Asked>());
  useEffect(() => {
    let current = true;
    setKnown(new Map());
    for (const machine of machines) {
      const kept = (asked: Asked) => { if (current) setKnown((all) => new Map(all).set(machine.id, asked)); };
      request<RunnerAnswer>('/network/machines/' + encodeURIComponent(machine.id) + '/runner').then(readRunner).then(kept,
        (problem: unknown) => kept({ ...ASKING, asked: 'unread', reason: problem instanceof Refused ? problem.refusal.refusal + ': ' + problem.refusal.reason : problem instanceof Error ? problem.message : String(problem) }));
    }
    return () => { current = false; };
  }, [machines]);
  return known;
}

type Show = 'all' | 'attention';

/** The computers and what runs on them; their runners are asked afterwards, each on its own. */
async function readComputers(): Promise<{ machines: Machine[]; live: RuntimeSession[] | null }> {
  const { view, live } = await readTogether({
    view: request<NetworkView>('/network'),
    live: request<{ sessions: RuntimeSession[] }>('/runtime/live').then((answer) => answer.sessions, () => null),
  });
  return { machines: view.machines, live };
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
  const load = useLoad(() => readTogether({
    computers: readComputers(), people: api.people(), me: api.me(),
    teams: readTeams().then((teams) => ({ teams, refused: '' }), (problem: unknown) => ({ teams: [], refused: problemWords(problem) })),
  }), 'network:' + revision);
  return <div className="page fill">
    <Gate load={load} title="Network" ok={(data) => <Computers {...data} teams={data.teams.teams} teamsRefused={data.teams.refused} notice={notice} refresh={(message) => { setNotice(message); setRevision((value) => value + 1); }} />} />
  </div>;
}

function Computers({ computers: read, people, me, teams, teamsRefused, notice, refresh }: { computers: Awaited<ReturnType<typeof readComputers>>; people: PeopleView; me: { person: { id: string } }; teams: OrgTeam[]; teamsRefused: string; notice: string; refresh: (message: string) => void }) {
  const admin = people.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const [show, setShow] = useState<Show>('all');
  const known = useRunners(read.machines);
  const computers: Computer[] = read.machines.map((machine) => ({ machine, ...(known.get(machine.id) ?? ASKING),
    running: read.live?.filter((session) => session.machine === machine.id && session.shown !== 'stopped') ?? null }));
  const [search] = useSearchParams();
  const adding = search.get('add') === 'computer';
  const name = useRef<HTMLInputElement>(null);
  // Arriving to add a computer, as from an agent's page, puts the keyboard in the add row's name box.
  useEffect(() => { if (adding) name.current?.focus(); }, [adding]);
  const [picked, setPicked] = useState<string | null>(null);
  const names = new Map(people.people.flatMap((person) => [[person.id, person.display_name] as const, ...person.agents.map((agent) => [agent.id, agent.display_name] as const)]));
  const attention = (computer: Computer) => status(computer).state === 'down';
  const [retired, setRetired] = useState(false);
  const retiredCount = computers.filter((computer) => computer.machine.state === 'retired').length;
  const scoped = computers.filter((computer) => inScope(whose, teams, me.person.id, people, computer)).filter((computer) => show === 'all' || attention(computer)).filter((computer) => retired || computer.machine.state !== 'retired');
  const groups = groupByTeam(scoped, held, teams, whose, (id) => names.get(id) ?? 'someone outside your view');
  const selected = computers.find((computer) => computer.machine.id === picked) ?? scoped[0] ?? null;
  const count = (n: number) => n.toLocaleString('en-AU');
  const up = computers.filter((computer) => status(computer).state === 'up').length;
  const running = computers.reduce((sum, computer) => sum + (computer.running?.length ?? 0), 0);
  const columns: Column<Computer>[] = [
    { head: 'Computer', cell: (computer) => computer.machine.name },
    { head: 'Status', cell: (computer) => { const now = status(computer); return <><span className={'dot ' + (now.state === 'up' ? 's-active' : now.state === 'down' ? 's-suspended' : 's-retired')} />{now.words}</>; } },
    { head: 'Running now', cell: (computer) => computer.running === null ? <span className="dim">cannot tell</span> : computer.running.length ? counted(computer.running.length, 'agents') : <span className="dim">nothing</span> },
    { head: 'May start here', cell: (computer) => <span className="sec">{mayStart(computer.machine)}</span> },
  ];
  const changed = (message: string) => { refresh(message); };
  return <>
    <div className="head">
      <div><div className="eyebrow">Where agents run</div><h1>Network</h1></div>
      {admin ? <button className="btn primary" onClick={() => name.current?.focus()}>+ Add a computer</button> : null}
    </div>
    {/* How many computers there are is the list's own count; the strip says only what the list does not. */}
    <div className="stat-strip">
      <div className="stat"><div className="n">{count(up)}</div><div className="l">up now</div></div>
      <div className="stat"><div className="n" style={computers.some(attention) ? { color: 'var(--warn)' } : undefined}>{count(computers.filter(attention).length)}</div><div className="l">not answering</div></div>
      <div className="stat"><div className="n">{count(running)}</div><div className="l">{running === 1 ? 'agent running' : 'agents running'}</div></div>
    </div>
    {notice ? <p role="status">{notice}</p> : null}
    {teamsRefused ? <p className="why-not">Teams cannot be read, so computers are listed without their team. {teamsRefused}</p> : null}
    <div className="body halves">
      <Listing<Computer> groups={groups} columns={columns} id={(computer) => computer.machine.id} href={(computer) => '#/network?computer=' + computer.machine.id}
        words={(computer) => computer.machine.name + ' ' + computer.machine.may_run.map((agent) => agent.display_name).join(' ')} noun="computers" empty="No computer yet. Add the one Lys runs on to start agents here."
        holds={(items) => counted(items.length, 'computers')}
        selected={selected?.machine.id ?? null} select={(computer) => setPicked(computer.machine.id)} open={(computer) => setPicked(computer.machine.id)}
        foot={admin ? <AddComputerRow person={me.person.id} changed={changed} name={name} /> : null}
        tools={<>
          <WhoseSelect whose={whose} set={setWhose} teams={teams} admin={admin} />
          <div className="seg">{([['all', 'All'], ['attention', 'Not answering']] as [Show, string][]).map(([key, label]) => <button key={key} className={show === key ? 'on' : ''} onClick={() => setShow(key)}>{label}</button>)}</div>
          {retiredCount ? <button type="button" className="btn" aria-pressed={retired} onClick={() => setRetired(!retired)}>{retired ? 'Hide retired' : 'Show retired (' + retiredCount + ')'}</button> : null}
        </>} />
      <div className="pane">
        {selected ? <MachineDetail key={selected.machine.id} computer={selected} admin={admin} me={me.person.id} teams={teams} names={names} changed={changed} /> : computers.length ? <p className="dim">Choose a computer.</p> : null}
      </div>
    </div>
  </>;
}

/** Who may start agents on a computer, every one named. */
export function mayStart(machine: Machine): string {
  if (machine.runtime === null) return 'Lys does not start agents here.';
  const roles = (machine.may_run_roles ?? []).map((role) => 'anyone holding ' + role);
  const all = [...machine.may_run.map((agent) => agent.display_name), ...roles];
  return all.length ? all.join(', ') : 'No agent may start here yet.';
}
