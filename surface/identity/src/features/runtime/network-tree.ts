import { useRef, useState } from 'react';
import { api, operationId, Refused, seats } from '../../api';
import type { SeatList, SeatView, UnregisteredSession } from '../../api';
import { readBounded, readEveryPage } from '../../reads';
import type { Paged } from '../../reads';
import { readableMachine } from '../network/contract';
import type { Machine, NetworkView } from '../network/contract';
import { said } from './canvas-kept';

export interface TreeLeaf { seat: SeatView; name: string; reason: string | null }
/**
 * An agent on a computer: every agent the computer may run and every agent holding a seat there, once each, with its
 * seats in the order the service listed them. `running` names the sessions a runner holds for it outside any seat.
 */
export interface TreeAgent { id: string; name: string; reason: string | null; leaves: TreeLeaf[]; running: UnregisteredSession[] }
/** A computer: its agents, and every seat on it in one list for whoever wants them flat. */
export interface TreeBranch { machine: Machine; agents: TreeAgent[]; leaves: TreeLeaf[] }
export interface TreeRead extends SeatList { machines: Machine[]; names: Record<string, string>; nameReasons: Record<string, string>; unregistered: UnregisteredSession[]; unregisteredReason: string | null }

export const online = (seat: SeatView): boolean => seat.state === 'online-idle' || seat.state === 'online-working';

/** How long the tree waits for the runners' session list before drawing without it; the reason is kept and shown. */
export const UNREGISTERED_WAIT_MS = 4000;

function networkPage(value: NetworkView & Paged): NetworkView & Paged {
  if (!value || !Array.isArray(value.machines) || !value.machines.every((machine) => readableMachine(machine)
    && typeof machine.kind === 'string' && machine.may_run.every((agent) => typeof agent.display_name === 'string'))
    || (value.next !== undefined && value.next !== null && typeof value.next !== 'string')) {
    throw new Error('NetworkUnreadable: the service did not answer readable machines and agent names.');
  }
  return value;
}

/** The sessions held for no seat, or the reason they are not known; a slow runner read does not hold the tree. */
async function readUnregistered(): Promise<{ sessions: UnregisteredSession[]; reason: string | null }> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<never>((_resolve, reject) => { timer = setTimeout(() => reject(new Error('RunnerSlow: the session list did not answer within ' + UNREGISTERED_WAIT_MS + ' ms.')), UNREGISTERED_WAIT_MS); });
  try {
    const answer = await Promise.race([seats.unregistered(), late]);
    const unanswered = (answer.unanswered ?? []).map((runner) => runner.name + ': ' + runner.reason).join('; ');
    return { sessions: answer.sessions, reason: unanswered || null };
  } catch (error) { return { sessions: [], reason: said(error) }; }
  finally { clearTimeout(timer); }
}

export async function readNetworkTree(): Promise<TreeRead> {
  const [network, list, unregistered] = await Promise.all([
    readEveryPage<NetworkView & Paged>('/network', (before, page) => ({ ...networkPage(before), machines: [...before.machines, ...networkPage(page).machines] })).then(networkPage),
    seats.list(),
    readUnregistered(),
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
  return { ...list, machines: network.machines, names, nameReasons, unregistered: unregistered.sessions, unregisteredReason: unregistered.reason };
}

/**
 * The tree's branches: each computer in use, under it every agent it may run (in the service's order, once each
 * however many times the service listed it) and every agent holding a seat there, each with its seats and with the
 * sessions a runner holds for it outside any seat.
 */
export function branches(network: Pick<TreeRead, 'machines'>, list: SeatList, names: Record<string, string> = {}, nameReasons: Record<string, string> = {}, unregistered: UnregisteredSession[] = []): TreeBranch[] {
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
      const made = { id, name, reason: nameReasons[id] ?? null, leaves: [], running: unregistered.filter((session) => session.machine === machine.id && session.agent === id) };
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
