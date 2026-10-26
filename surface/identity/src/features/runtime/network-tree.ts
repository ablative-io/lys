import { useEffect, useRef, useState } from 'react';
import { api, operationId, Refused, request, seats } from '../../api';
import type { SeatList, SeatView } from '../../api';
import type { RuntimeSession } from './RuntimeSessions';
import type { Unanswered } from './Sessions';
import { readBounded, readEveryPage } from '../../reads';
import type { Paged } from '../../reads';
import { readableMachine } from '../network/contract';
import type { Machine, NetworkView } from '../network/contract';
import { said } from './canvas-kept';
import { counted } from '../../shell/count';

export interface TreeLeaf { seat: SeatView; name: string; reason: string | null }
/**
 * An agent on a computer: every agent the computer may run and every agent holding a seat there, once each, with its
 * seats in the order the service listed them. `running` names its live sessions on that computer outside any seat.
 */
export interface TreeAgent { id: string; name: string; reason: string | null; leaves: TreeLeaf[]; running: RuntimeSession[] }
/** A computer: its agents, and every seat on it in one list for whoever wants them flat. */
export interface TreeBranch { machine: Machine; agents: TreeAgent[]; leaves: TreeLeaf[] }
export interface TreeRead extends SeatList { machines: Machine[] }
/** Agents' names as they arrive, and why a name could not be read. */
export interface AgentNames { names: Record<string, string>; reasons: Record<string, string> }
/** The live sessions, and which sessions' runners did not answer, as one line, or null when all did. */
export interface RunningRead { sessions: RuntimeSession[]; unanswered: string | null }

/** What a seatless agent's row selects: its live session, told from a seat's name by this prefix. */
export const SESSION_CHOICE = 'session:';
/** What an agent's own row selects when it is not one seat's row: the agent, told from a seat's name by this prefix. */
export const AGENT_CHOICE = 'agent:';

export const online = (seat: SeatView): boolean => seat.state === 'online-idle' || seat.state === 'online-working';

function networkPage(value: NetworkView & Paged): NetworkView & Paged {
  if (!value || !Array.isArray(value.machines) || !value.machines.every((machine) => readableMachine(machine)
    && typeof machine.kind === 'string' && machine.may_run.every((agent) => typeof agent.display_name === 'string'))
    || (value.next !== undefined && value.next !== null && typeof value.next !== 'string')) {
    throw new Error('Lys answered, but its list of computers could not be read. Reload the page; if it happens again, ask the administrator to check Lys.');
  }
  return value;
}

/**
 * The live sessions, as the runtime keeps them. Read on its own, never as part of the tree: the tree is drawn from
 * the network and the seats the moment they answer, and this read only adds the running marks when it arrives. A
 * session whose runner did not answer is named in `unanswered`, for the rows it leaves unmarked.
 */
export async function readRunning(): Promise<RunningRead> {
  const answer = await request<{ sessions: RuntimeSession[]; unanswered: Unanswered[] }>('/runtime/live');
  if (!answer || !Array.isArray(answer.sessions) || !Array.isArray(answer.unanswered)
    || answer.sessions.some((session) => typeof session.session !== 'string' || typeof session.machine !== 'string' || session.shown === 'stopped')) {
    throw new Error('Lys answered, but its list of running sessions could not be read. Reload the page; if it happens again, ask the administrator to check Lys.');
  }
  const unanswered = answer.unanswered.length ? counted(answer.unanswered.length, 'runners') + ' did not answer, so ' + (answer.unanswered.length === 1 ? 'its' : 'their') + ' sessions are not shown as running.' : '';
  return { sessions: answer.sessions, unanswered: unanswered || null };
}

/**
 * The computers and the seats: all the tree needs to be drawn. Names the computers do not give arrive afterwards, on
 * their own (see `useAgentNames`), so the first draw never waits on them.
 */
export async function readNetworkTree(): Promise<TreeRead> {
  const [network, list] = await Promise.all([
    readEveryPage<NetworkView & Paged>('/network', (before, page) => ({ ...networkPage(before), machines: [...before.machines, ...networkPage(page).machines] })).then(networkPage),
    seats.list(),
  ]);
  return { ...list, machines: network.machines };
}

/** Every name the computers give, by agent. */
const givenNames = (tree: TreeRead): Record<string, string> => Object.fromEntries(tree.machines.flatMap((machine) => machine.may_run.map((agent) => [agent.id, agent.display_name])));

/** One agent's name from the directory, or why it could not be read. */
export async function readAgentName(agent: string): Promise<{ name: string } | { reason: string }> {
  try {
    const record = await api.agent(agent);
    if (typeof record.display_name !== 'string' || !record.display_name.trim()) throw new Error('Lys gave this agent no name.');
    return { name: record.display_name };
  } catch (error) { return { reason: said(error) }; }
}

/**
 * The names of the agents holding seats whose computers do not name them, read after the tree is drawn and filled in
 * as each answers, the way the live sessions fill in the running marks. Until a name arrives its row shows the
 * agent's id. A name already read is not read again when the tree is read again.
 */
export function useAgentNames(tree: TreeRead | null): AgentNames {
  const [read, setRead] = useState<AgentNames>({ names: {}, reasons: {} });
  const given = tree ? givenNames(tree) : {};
  const missing = tree ? [...new Set(tree.seats.map((seat) => seat.agent))].filter((agent) => !(agent in given) && !(agent in read.names)).sort() : [];
  const key = missing.join('\n');
  useEffect(() => {
    if (!key) return undefined;
    let live = true;
    void readBounded(key.split('\n'), async (agent) => {
      const answer = await readAgentName(agent);
      if (!live) return;
      setRead((before) => 'name' in answer
        ? { names: { ...before.names, [agent]: answer.name }, reasons: Object.fromEntries(Object.entries(before.reasons).filter(([id]) => id !== agent)) }
        : { names: before.names, reasons: { ...before.reasons, [agent]: answer.reason } });
    });
    return () => { live = false; };
  }, [key]);
  return { names: { ...read.names, ...given }, reasons: read.reasons };
}

/**
 * The tree's branches: each computer in use, under it every agent it may run that is not retired (in the service's
 * order, once each however many times the service listed it; a retired one only while something of it still runs) and every agent holding a seat there, each with its seats and with the
 * sessions a runner holds for it outside any seat.
 */
export function branches(network: Pick<TreeRead, 'machines'>, list: SeatList, names: Record<string, string> = {}, nameReasons: Record<string, string> = {}, running: RuntimeSession[] = []): TreeBranch[] {
  const byMachine = new Map<string, SeatView[]>();
  for (const seat of list.seats) {
    const held = byMachine.get(seat.machine);
    if (held) held.push(seat); else byMachine.set(seat.machine, [seat]);
  }
  return network.machines.filter((machine) => machine.state !== 'retired').map((machine) => {
    // A retired agent has nothing to open: it is left out, unless something of it still runs here and needs stopping.
    const retired = new Set(machine.may_run.filter((agent) => agent.state === 'retired').map((agent) => agent.id));
    const stillRuns = (id: string) => running.some((session) => session.machine === machine.id && session.agent === id)
      || (byMachine.get(machine.id) ?? []).some((seat) => seat.agent === id && online(seat));
    const shown = (id: string) => !retired.has(id) || stillRuns(id);
    const allowed = new Map(machine.may_run.filter((agent) => shown(agent.id)).map((agent) => [agent.id, agent.display_name]));
    const leaves: TreeLeaf[] = (byMachine.get(machine.id) ?? []).filter((seat) => shown(seat.agent)).map((seat) => ({ seat,
      name: names[seat.agent] ?? allowed.get(seat.agent) ?? seat.agent,
      reason: [seat.reason, list.runner === 'unknown' ? list.reason ?? 'Runner unknown: no reason returned.' : null, nameReasons[seat.agent]].filter((value) => value).join('; ') || null,
    }));
    const agents: TreeAgent[] = [];
    const agent = (id: string, name: string): TreeAgent => {
      const held = agents.find((each) => each.id === id);
      if (held) return held;
      const seated = new Set(leaves.map((leaf) => leaf.seat.session).filter((session) => session));
      const made = { id, name, reason: nameReasons[id] ?? null, leaves: [], running: running.filter((session) => session.machine === machine.id && session.agent === id && !seated.has(session.session)) };
      agents.push(made);
      return made;
    };
    for (const [id, name] of allowed) agent(id, name);
    for (const leaf of leaves) agent(leaf.seat.agent, leaf.name).leaves.push(leaf);
    return { machine, agents, leaves };
  });
}

/** An uncertain outcome retains its operation so a second press cannot duplicate the first act. */
export function useSeatAction(seat: SeatView, change: (seat: SeatView) => void) {
  const held = useRef<{ act: string; text?: string; operation: string } | null>(null);
  const working = useRef(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  // The service says "Start again" in its own reason when a fresh start is what fixes it; the words shown are plain, so that is kept apart.
  const [startAgain, setStartAgain] = useState(false);
  // What failed, whole, for the Error details fold under the plain words.
  const [problem, setProblem] = useState<unknown>(null);
  const [done, setDone] = useState<string | null>(null);
  const [stopEnded, setStopEnded] = useState(false);
  const [waitingSince, setWaitingSince] = useState<number | null>(null);
  const [endingNow, setEndingNow] = useState(false);
  const callers = useRef(new Set<AbortController>());
  useEffect(() => () => { for (const caller of callers.current) caller.abort(); }, []);
  const run = async (act: 'start' | 'restart' | 'stop' | 'send', text?: string): Promise<boolean> => {
    if (working.current) return false;
    if (!held.current || held.current.act !== act || held.current.text !== text) held.current = { act, text, operation: operationId() };
    working.current = true; setBusy(true); setFailure(null); setStartAgain(false); setProblem(null); setDone(null);
    const caller = new AbortController(); callers.current.add(caller);
    try {
      const operation = held.current.operation;
      if (act === 'start' || act === 'restart') {
        const answer = act === 'restart' ? await seats.restart(seat.name, operation, false, caller.signal) : await seats.start(seat.name, operation, caller.signal);
        change(answer.seat); setDone(act === 'restart' ? 'Restarted.' : 'Started.');
      } else if (act === 'stop') {
        setWaitingSince(Date.now());
        const answer = await seats.stop(seat.name, operation, false, caller.signal);
        setStopEnded(answer.ended);
        change(answer.seat); setDone(answer.ended ? 'Stopped.' : 'Asked to stop. Its computer has not yet confirmed it stopped.');
      } else {
        await seats.send(seat.name, operation, text ?? '', caller.signal);
        setDone('Sent.');
      }
      held.current = null;
      return true;
    } catch (error) {
      if (error instanceof Refused && error.status >= 400 && error.status < 500) held.current = null;
      setFailure(said(error));
      setProblem(error);
      setStartAgain(error instanceof Refused && error.refusal.reason.includes('Start again'));
      return false;
    } finally { callers.current.delete(caller); working.current = false; setBusy(false); setWaitingSince(null); }
  };
  const endNow = async () => {
    if (!seat.session || endingNow) return;
    setEndingNow(true);
    const caller = new AbortController(); callers.current.add(caller);
    try { await request('/runtime/sessions/' + encodeURIComponent(seat.session) + '/end-now', {}, 'POST', caller.signal); }
    catch (error) { if (!caller.signal.aborted) { setFailure(said(error)); setProblem(error); } }
    finally { callers.current.delete(caller); setEndingNow(false); }
  };
  return { busy, failure, problem, startAgain, done, stopEnded, run, waitingSince, endingNow, endNow };
}
