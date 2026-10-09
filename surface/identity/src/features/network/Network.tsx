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
import { subscribeChanges } from '../../live';
import { AddComputerRow } from './AddMachine';
import type { Machine, NetworkView } from './contract';
import { ConnectRow } from './JoinCode';
import type { Connecting } from './JoinCode';
import { MachineDetail, status, waiting } from './MachineDetail';
import { readIdentities } from './MachineIdentities';
import type { MachineIdentity } from './contract';
import type { RunnerRecord } from './MachineDetail';
import { Act } from '../../shell/Act';

/**
 * What Lys knows of a computer's runner: still being asked; answered, with its record and whether the runner answered
 * just now (null when it has none) and why not, and whether a connection code for it waits to be used; or not read at
 * all, with why in `reason`.
 */
export interface Asked { asked: 'asking' | 'answered' | 'unread'; runner: RunnerRecord | null; answers: boolean | null; refusal: string | null; reason: string | null; joining: boolean }

/** One computer with what Lys knows of it now. */
export interface Computer extends Asked { machine: Machine; running: RuntimeSession[] | null }

const ASKING: Asked = { asked: 'asking', runner: null, answers: null, refusal: null, reason: null, joining: false };

/** A machine's runner as the service answers it, asked just now. */
type RunnerAnswer = { runner: RunnerRecord | null; answers: boolean | null; refusal?: string; reason?: string; joining?: boolean };

function readRunner(answer: RunnerAnswer): Asked {
  if (answer.answers !== true && answer.answers !== false && answer.answers !== null) throw new Error('The service did not say whether the computer\'s runner answers.');
  return { asked: 'answered', runner: answer.runner, answers: answer.answers, refusal: answer.refusal ?? null, reason: answer.reason ?? null, joining: answer.joining === true };
}

/** Ask one computer's runner, handing what is known of it to `kept`. */
function askRunner(machine: string, signal: AbortSignal, kept: (asked: Asked) => void): void {
  request<RunnerAnswer>('/network/machines/' + encodeURIComponent(machine) + '/runner', undefined, 'POST', signal).then(readRunner).then(kept,
    (problem: unknown) => kept({ ...ASKING, asked: 'unread', reason: problem instanceof Refused ? problem.refusal.refusal + ': ' + problem.refusal.reason : problem instanceof Error ? problem.message : String(problem) }));
}

/**
 * Each computer's runner, asked on its own: each answer fills its own row when it arrives, so a runner that never
 * answers holds only its own row, and the page is drawn without waiting on any of them.
 */
function useRunners(machines: Machine[]): { known: Map<string, Asked>; feed: string; reask: (ids: string[]) => void } {
  const [known, setKnown] = useState(() => new Map<string, Asked>());
  const [feed, setFeed] = useState('');
  const [again, setAgain] = useState<{ ids: string[] } | null>(null);
  useEffect(() => {
    let current = true;
    // Leaving the page, or a new list of computers, ends every ask still waiting, so a runner that never answers holds no connection after it.
    const leave = new AbortController();
    setKnown(new Map());
    for (const machine of machines) askRunner(machine.id, leave.signal, (asked) => { if (current) setKnown((all) => new Map(all).set(machine.id, asked)); });
    return () => { current = false; leave.abort(); };
  }, [machines]);
  const waitingIds = machines.filter((machine) => { const asked = known.get(machine.id); return asked?.asked === 'answered' && asked.answers !== true && waiting(asked); })
    .map((machine) => machine.id).join(' ');
  // While a computer waits for its runner, each change Lys signals asks its runner again, so it reads Up once it connects.
  useEffect(() => {
    if (!waitingIds) return undefined;
    return subscribeChanges(() => { setFeed(''); setAgain({ ids: waitingIds.split(' ') }); }, (problem) => setFeed(problemWords(problem)));
  }, [waitingIds]);
  useEffect(() => {
    if (!again) return undefined;
    let current = true;
    const leave = new AbortController();
    for (const id of again.ids) askRunner(id, leave.signal, (asked) => { if (current) setKnown((all) => new Map(all).set(id, asked)); });
    return () => { current = false; leave.abort(); };
  }, [again]);
  return { known, feed, reask: (ids) => setAgain({ ids }) };
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
  // A connection code lives here, above the page's reads, so a reload never loses one that is shown only once.
  const [connecting, setConnecting] = useState<Connecting | null>(null);
  const load = useLoad(() => readTogether({
    computers: readComputers(), people: api.people(), me: api.me(),
    teams: readTeams().then((teams) => ({ teams, refused: '' }), (problem: unknown) => ({ teams: [], refused: problemWords(problem) })),
    // The machines the joins made, and what each holds (ACCESS-005 R2): the administrator's to read; anyone else is told nothing of them.
    machines: readIdentities().then((list): Machines => ({ list, refused: '' }), (problem: unknown): Machines => ({ list: [], refused: problemWords(problem) })),
  }), 'network:' + revision);
  return <div className="page fill">
    <Gate load={load} title="Network" ok={(data) => <Computers {...data} teams={data.teams.teams} teamsRefused={data.teams.refused} machines={data.machines} notice={notice} connecting={connecting} connect={setConnecting} refresh={(message) => { setNotice(message); setRevision((value) => value + 1); }} />} />
  </div>;
}

/** The machines as read for the page: every one, or none and why. */
type Machines = { list: MachineIdentity[]; refused: string };

function Computers({ computers: read, people, me, teams, teamsRefused, machines, notice, connecting, connect, refresh }: { computers: Awaited<ReturnType<typeof readComputers>>; people: PeopleView; me: { person: { id: string } }; teams: OrgTeam[]; teamsRefused: string; machines: Machines; notice: string;
  connecting: Connecting | null; connect: (next: Connecting | null) => void; refresh: (message: string) => void }) {
  const admin = people.scope === 'directory';
  const [whose, setWhose] = useWhose(admin);
  const [show, setShow] = useState<Show>('all');
  const { known, feed, reask } = useRunners(read.machines);
  // A new code shown beside the list leaves the page as it is, and asks that computer's runner again.
  const connectHere = (next: Connecting) => { connect(next); if (next.given) reask([next.machine]); };
  const done = () => connect(null);
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
      {admin ? <Act symbol="add" name="Add a computer" word="Computer" tone="primary" onClick={() => name.current?.focus()} /> : null}
    </div>
    {/* How many computers there are is the list's own count; the strip says only what the list does not. */}
    <div className="stat-strip">
      <div className="stat"><div className="n">{count(up)}</div><div className="l">up now</div></div>
      <div className="stat"><div className="n" style={computers.some(attention) ? { color: 'var(--warn)' } : undefined}>{count(computers.filter(attention).length)}</div><div className="l">not answering</div></div>
      <div className="stat"><div className="n">{count(running)}</div><div className="l">{running === 1 ? 'agent running' : 'agents running'}</div></div>
    </div>
    {notice ? <p role="status">{notice}</p> : null}
    {feed ? <p className="why-not">Lys stopped hearing about changes, so a computer waiting for its runner reads Up only when this page is opened again. {feed}</p> : null}
    {teamsRefused ? <p className="why-not">Teams cannot be read, so computers are listed without their team. {teamsRefused}</p> : null}
    {admin && machines.refused ? <p className="why-not">The machines cannot be read, so what each computer holds as a machine is not shown. {machines.refused}</p> : null}
    <div className="body halves">
      <Listing<Computer> groups={groups} columns={columns} id={(computer) => computer.machine.id} href={(computer) => '#/network?computer=' + computer.machine.id}
        words={(computer) => computer.machine.name + ' ' + computer.machine.may_run.map((agent) => agent.display_name).join(' ')} noun="computers" empty="No computer yet. Add the one Lys runs on to start agents here."
        holds={(items) => counted(items.length, 'computers')}
        selected={selected?.machine.id ?? null} select={(computer) => setPicked(computer.machine.id)} open={(computer) => setPicked(computer.machine.id)}
        foot={admin ? <>
          {connecting?.at === 'row' ? <ConnectRow connecting={connecting} connect={connectHere} done={done} width={columns.length} /> : null}
          <AddComputerRow person={me.person.id} changed={changed} name={name} connect={connect} />
        </> : null}
        tools={<>
          <WhoseSelect whose={whose} set={setWhose} teams={teams} admin={admin} />
          <div className="seg">{([['all', 'All'], ['attention', 'Not answering']] as [Show, string][]).map(([key, label]) => <button key={key} className={show === key ? 'on' : ''} aria-pressed={show === key} onClick={() => setShow(key)}>{label}</button>)}</div>
          {retiredCount ? <button type="button" className="btn" aria-pressed={retired} onClick={() => setRetired(!retired)}>{retired ? 'Hide retired' : 'Show retired (' + retiredCount + ')'}</button> : null}
        </>} />
      <div className="pane">
        {selected ? <MachineDetail key={selected.machine.id} computer={selected} admin={admin} me={me.person.id} teams={teams} names={names} changed={changed}
          machines={admin && !machines.refused ? machines.list : null}
          connecting={connecting} connect={connectHere} done={done} /> : computers.length ? <p className="dim">Choose a computer.</p> : null}
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
