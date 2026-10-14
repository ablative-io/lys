import { useRef, useState } from 'react';
import { api, operationId, Refused, request, seats } from '../../api';
import type { SeatList, SeatView } from '../../api';
import type { RuntimeSession } from './RuntimeSessions';
import type { Unanswered } from './Sessions';
import { readBounded, readEveryPage } from '../../reads';
import type { Paged } from '../../reads';
import { readableMachine } from '../network/contract';
import type { Machine, NetworkView } from '../network/contract';
import { said } from './canvas-kept';

export interface TreeLeaf { seat: SeatView; name: string; reason: string | null }
/**
 * An agent on a computer: every agent the computer may run and every agent holding a seat there, once each, with its
 * seats in the order the service listed them. `running` names its live sessions on that computer outside any seat.
 */
export interface TreeAgent { id: string; name: string; reason: string | null; leaves: TreeLeaf[]; running: RuntimeSession[] }
/** A computer: its agents, and every seat on it in one list for whoever wants them flat. */
export interface TreeBranch { machine: Machine; agents: TreeAgent[]; leaves: TreeLeaf[] }
export interface TreeRead extends SeatList { machines: Machine[]; names: Record<string, string>; nameReasons: Record<string, string> }
/** The live sessions, and which sessions' runners did not answer, as one line, or null when all did. */
export interface RunningRead { sessions: RuntimeSession[]; unanswered: string | null }

/** What a seatless agent's row selects: its live session, told from a seat's name by this prefix. */
export const SESSION_CHOICE = 'session:';

export const online = (seat: SeatView): boolean => seat.state === 'online-idle' || seat.state === 'online-working';

function networkPage(value: NetworkView & Paged): NetworkView & Paged {
  if (!value || !Array.isArray(value.machines) || !value.machines.every((machine) => readableMachine(machine)
    && typeof machine.kind === 'string' && machine.may_run.every((agent) => typeof agent.display_name === 'string'))
    || (value.next !== undefined && value.next !== null && typeof value.next !== 'string')) {
    throw new Error('NetworkUnreadable: the service did not answer readable machines and agent names.');
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
    throw new Error('RuntimeLiveUnreadable: the service did not answer a readable live session list.');
  }
  const unanswered = answer.unanswered.map((each) => each.session + ': ' + each.refusal + ': ' + each.reason).join('; ');
  return { sessions: answer.sessions, unanswered: unanswered || null };
}

export async function readNetworkTree(): Promise<TreeRead> {
  const [network, list] = await Promise.all([
    readEveryPage<NetworkView & Paged>('/network', (before, page) => ({ ...networkPage(before), machines: [...before.machines, ...networkPage(page).machines] })).then(networkPage),
    seats.list(),
  ]);
  const names: Record<string, string> = Object.fromEntries(network.machines.flatMap((machine) => machine.may_run.map((agent) => [agent.id, agent.display_name])));
  const nameReasons: Record<string, string> = {};
  await readBounded([...new Set(list.seats.map((seat) => seat.agent))].filter((agent) => !(agent in names)), async (agent) => {
    try {
      const record = await api.agent(agent);
      if (typeof record.display_name !== 'string' || !record.display_name.trim()) throw new Error('AgentNameUnreadable: the service gave no display name.');
      names[agent] = record.display_name;
    } catch (error) { nameReasons[agent] = said(error); }
  });
  return { ...list, machines: network.machines, names, nameReasons };
}

/**
 * The tree's branches: each computer in use, under it every agent it may run (in the service's order, once each
 * however many times the service listed it) and every agent holding a seat there, each with its seats and with the
 * sessions a runner holds for it outside any seat.
 */
export function branches(network: Pick<TreeRead, 'machines'>, list: SeatList, names: Record<string, string> = {}, nameReasons: Record<string, string> = {}, running: RuntimeSession[] = []): TreeBranch[] {
  const byMachine = new Map<string, SeatView[]>();
  for (const seat of list.seats) {
    const held = byMachine.get(seat.machine);
    if (held) held.push(seat); else byMachine.set(seat.machine, [seat]);
  }
  return network.machines.filter((machine) => machine.state !== 'retired').map((machine) => {
    const allowed = new Map(machine.may_run.map((agent) => [agent.id, agent.display_name]));
    const leaves: TreeLeaf[] = (byMachine.get(machine.id) ?? []).map((seat) => ({ seat,
      name: names[seat.agent] ?? allowed.get(seat.agent) ?? 'Agent name unavailable (' + seat.name + ')',
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
  const [done, setDone] = useState<string | null>(null);
  const [stopEnded, setStopEnded] = useState(false);
  const run = async (act: 'start' | 'stop' | 'send', text?: string): Promise<boolean> => {
    if (working.current) return false;
    if (!held.current || held.current.act !== act || held.current.text !== text) held.current = { act, text, operation: operationId() };
    working.current = true; setBusy(true); setFailure(null); setDone(null);
    try {
      const operation = held.current.operation;
      if (act === 'start') {
        const answer = await seats.start(seat.name, operation);
        change(answer.seat); setDone('Started as session ' + answer.session + '.');
      } else if (act === 'stop') {
        const answer = await seats.stop(seat.name, operation, false);
        setStopEnded(answer.ended);
        change(answer.seat); setDone(answer.ended ? 'Session ' + answer.session + ' ended.' : 'Stop asked of session ' + answer.session + '; its runner has not confirmed it ended.');
      } else {
        const answer = await seats.send(seat.name, operation, text ?? '');
        setDone('Delivered to session ' + answer.session + ' as a user turn.');
      }
      held.current = null;
      return true;
    } catch (error) {
      if (error instanceof Refused && error.status >= 400 && error.status < 500) held.current = null;
      setFailure(said(error));
      return false;
    } finally { working.current = false; setBusy(false); }
  };
  return { busy, failure, done, stopEnded, run };
}
